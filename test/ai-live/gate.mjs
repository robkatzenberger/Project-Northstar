import { createHash, randomBytes } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { appendFileSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync,
  renameSync, readdirSync, rmSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createSwitchboard, createTokenAuthenticator } from '../../src/switchboard.js';

// Cooperative local test transport. The mailbox is not a security boundary
// against other processes sharing the host identity.
const here = dirname(fileURLToPath(import.meta.url));
const apex = resolve(here, '../..');
const runId = `ai-live-${new Date().toISOString().replace(/[-:.]/g, '')}`;
const evidence = join(apex, 'test/reports', runId);
const mailbox = mkdtempSync('/private/tmp/apex-ai-live-');
for (const name of ['credentials', 'requests', 'responses']) mkdirSync(join(mailbox, name), { mode: 0o700 });
mkdirSync(join(evidence, 'agents'), { recursive: true });
const ids = ['agent.ai.cedar', 'agent.ai.maple'];
const credentials = ids.map((principalId) => ({ principalId, token: randomBytes(32).toString('hex') }));
const json = (path, value) => writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, { mode: 0o600, flag: 'wx' });
const hash = (path) => createHash('sha256').update(readFileSync(path)).digest('hex');
const sources = ['src/switchboard.js', 'test/ai-live/gate.mjs', 'test/ai-live/client.mjs'];
const sourceSha256 = Object.fromEntries(sources.map((path) => [path, hash(join(apex, path))]));
const registries = [1, 2].map((version) => ({ id: 'apex.ai-live', version,
  agents: ids.map((id, index) => ({ id, whitelisted: index === version - 1,
    grants: [{ id: `grant.${id}`, action: 'switchboard.ping', target: 'gate:test', issuedBy: 'operator:ai-test' }] })) }));
json(join(evidence, 'manifest.json'), { runId, startedAt: new Date().toISOString(),
  participants: ids, registries, sourceSha256, environment: { node: process.version,
    platform: process.platform, arch: process.arch, gatePid: process.pid },
  gitHead: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: apex, encoding: 'utf8' }).trim(),
  scope: 'Two actual AI subagents submit tool calls to one persistent deterministic gate over cooperative local file IPC. No execution or hostile-process isolation.' });
registries.forEach((registry) => json(join(evidence, `registry-${registry.version}.json`), registry));
credentials.forEach(({ principalId, token }) => json(join(mailbox, 'credentials', `${principalId}.json`), { token }));
json(join(mailbox, 'connection.json'), { runId, evidence });
json(join(mailbox, 'control.json'), { round: 1, stop: false });
const authenticate = createTokenAuthenticator(credentials);
const log = (row) => appendFileSync(join(evidence, 'gate.jsonl'), `${JSON.stringify(row)}\n`, { mode: 0o600 });
let round = 0;
let gate;
let processed = 0;
let timer;
let finished = false;
const seen = new Set();

function status() {
  const temporary = join(mailbox, 'status.next');
  writeFileSync(temporary, JSON.stringify({ runId, round, processed }), { mode: 0o600 });
  renameSync(temporary, join(mailbox, 'status.json'));
}

function stop(code) {
  if (finished) return;
  finished = true;
  clearInterval(timer);
  let leaks = 0;
  let filesScanned = 0;
  function scan(dir) {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) scan(path);
      else {
        filesScanned += 1;
        const bytes = readFileSync(path);
        leaks += credentials.filter(({ token }) => bytes.includes(token)).length;
      }
    }
  }
  scan(evidence);
  const sourceUnchanged = Object.entries(sourceSha256).every(([path, digest]) => hash(join(apex, path)) === digest);
  // Only this gate's freshly created temporary mailbox is removed.
  rmSync(mailbox, { recursive: true, force: true });
  json(join(evidence, 'shutdown.json'), { stoppedAt: new Date().toISOString(), code,
    processed, sourceUnchanged, credentialsRemoved: !existsSync(mailbox),
    credentialScan: { filesScanned, credentialsChecked: credentials.length, literalMatches: leaks } });
  process.stdout.write(`${JSON.stringify({ stopped: true, evidence, processed, code, sourceUnchanged, credentialMatches: leaks })}\n`);
  process.exitCode = code === 'COMPLETE' && sourceUnchanged && leaks === 0 ? 0 : 1;
}

function tick() {
  if (finished) return;
  try {
    const control = JSON.parse(readFileSync(join(mailbox, 'control.json'), 'utf8'));
    if (control.stop) {
      const pending = readdirSync(join(mailbox, 'requests'));
      return stop(round === 2 && processed === 20 && pending.length === 0 ? 'COMPLETE' : 'INCOMPLETE');
    }
    if (![1, 2].includes(control.round)) throw new Error('Invalid round');
    if (control.round !== round) {
      round = control.round;
      gate = createSwitchboard(registries[round - 1], { authenticate });
      log({ event: 'registry_installed', runId, round, at: new Date().toISOString() });
      status();
    }
    for (const filename of readdirSync(join(mailbox, 'requests')).filter((name) => name.endsWith('.json')).sort()) {
      const path = join(mailbox, 'requests', filename);
      const envelope = JSON.parse(readFileSync(path, 'utf8'));
      if (!ids.includes(envelope.participantId) || !/^[a-z0-9-]{1,80}$/.test(envelope.requestId)
        || envelope.runId !== runId || envelope.round !== round || seen.has(envelope.requestId)
        || filename !== `${envelope.requestId}.json`) throw new Error('Invalid transport envelope');
      seen.add(envelope.requestId);
      const result = gate.screen(envelope.request);
      const receipt = { runId, round, requestId: envelope.requestId, participantId: envelope.participantId,
        intent: envelope.request.intent, credentialMode: envelope.credentialMode, result };
      log({ event: 'decision', ...receipt, at: new Date().toISOString() });
      const staging = join(mailbox, 'responses', `${envelope.requestId}.next`);
      json(staging, receipt);
      renameSync(staging, join(mailbox, 'responses', `${envelope.requestId}.json`));
      rmSync(path);
      processed += 1;
      status();
    }
  } catch {
    stop('GATE_HARNESS_FAILURE');
  }
}

tick();
if (!finished) {
  process.stdout.write(`${JSON.stringify({ ready: true, runId, mailbox, evidence, gatePid: process.pid })}\n`);
  timer = setInterval(tick, 20);
  setTimeout(() => stop('WATCHDOG_TIMEOUT'), 15 * 60 * 1000).unref();
}
process.on('SIGTERM', () => stop('TERMINATED'));
process.on('SIGINT', () => stop('INTERRUPTED'));
