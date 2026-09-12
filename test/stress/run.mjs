import { fork, execFileSync } from 'node:child_process';
import { randomBytes, randomInt, createHash } from 'node:crypto';
import { mkdirSync, openSync, writeSync, closeSync, writeFileSync, readFileSync,
  readdirSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import os from 'node:os';
import { createSwitchboard, createTokenAuthenticator } from '../../src/switchboard.js';
import { verifyRun } from './verify.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const apex = resolve(here, '../..');
const startedAt = new Date().toISOString();
const runId = `stress-${startedAt.replace(/[-:.]/g, '')}-${randomBytes(4).toString('hex')}`;
const runDir = join(apex, 'test/reports', runId);
mkdirSync(join(runDir, 'agents'), { recursive: true });
const json = (path, value) => writeFileSync(join(runDir, path), `${JSON.stringify(value, null, 2)}\n`,
  { flag: 'wx', mode: 0o600 });
const fd = openSync(join(runDir, 'gate.jsonl'), 'wx', 0o600);
let gateClosed = false;
const log = (row) => writeSync(fd, `${JSON.stringify(row)}\n`);
const digest = (path) => createHash('sha256').update(readFileSync(path)).digest('hex');
const agentIds = Array.from({ length: 10 }, (_, index) => `agent.stress.${String(index + 1).padStart(2, '0')}`);
const credentials = agentIds.map((principalId) => ({ principalId, token: randomBytes(32).toString('hex') }));
const authenticate = createTokenAuthenticator(credentials);
const action = 'switchboard.ping';
const target = 'gate:test';
const intervals = [50, 35, 25, 18, 12, 8, 5, 3, 1, 0];
const rounds = intervals.map((intervalMs, index) => {
  const shuffled = [...agentIds];
  for (let i = shuffled.length - 1; i > 0; i -= 1) {
    const j = randomInt(i + 1);
    [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
  }
  const whitelistCount = randomInt(0, agentIds.length + 1);
  const selected = new Set(shuffled.slice(0, whitelistCount));
  return { round: index + 1, intervalMs, whitelistCount,
    registry: { id: 'apex.stress', version: index + 1,
      agents: agentIds.map((id) => ({ id, whitelisted: selected.has(id),
        grants: [{ id: `grant.${id}`, action, target, issuedBy: 'operator:stress' }] })) } };
});
const sourceFiles = ['src/switchboard.js', 'test/stress/run.mjs', 'test/stress/agent.mjs', 'test/stress/verify.mjs'];
const manifest = { runId, startedAt, agentIds, pingsPerAgent: 100, action, target, rounds,
  randomness: 'crypto.randomInt; uniform count 0..10 inclusive; Fisher-Yates shuffled subset; no rerolls',
  agentType: 'Ten concurrent Node.js child processes, not AI-model agents',
  environment: { node: process.version, platform: process.platform, release: os.release(),
    arch: process.arch, cpus: os.cpus().length, cpuModel: os.cpus()[0]?.model,
    parentPid: process.pid },
  git: { head: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: apex, encoding: 'utf8' }).trim(),
    status: execFileSync('git', ['status', '--short'], { cwd: apex, encoding: 'utf8' }).trim() },
  sourceSha256: Object.fromEntries(sourceFiles.map((path) => [path, digest(join(apex, path))])),
  transport: 'Local Node IPC; one parent process calls the unchanged synchronous Switchboard; one registry snapshot per round',
  scope: 'Pre-policy PASS/DENY screening only; no execution, human verification, sealed evidence, live database, or hostile-process isolation' };
json('manifest.json', manifest);
rounds.forEach(({ round, registry }) => json(`registry-round-${String(round).padStart(2, '0')}.json`, registry));

const children = [];
const completions = [];
let activeRound;
let gate;
let phase;
let fatalError;
let stopping = false;
let pendingResponses = 0;
let responseDrain;
const failure = (code) => {
  fatalError ??= new Error(code);
  phase?.reject(fatalError);
  responseDrain?.reject(fatalError);
};

function barrier(type, round) {
  if (fatalError) throw fatalError;
  if (phase) throw new Error('Overlapping barriers');
  let timer;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    phase = { type, round, seen: new Set(),
      resolve: () => { clearTimeout(timer); phase = null; resolvePromise(); },
      reject: (error) => { clearTimeout(timer); phase = null; rejectPromise(error); } };
    timer = setTimeout(() => failure(`Barrier timeout: ${type}/${round ?? 'startup'}`), 30_000);
  });
  return promise;
}

function send(child, message) {
  child.process.send(message, (error) => { if (error) failure('Control IPC send failed'); });
}

function receive(child, message) {
  if (!message || typeof message !== 'object') return failure('Malformed worker message');
  if (message.type === 'ping') {
    if (!activeRound || message.round !== activeRound.round || message.runId !== runId
      || message.agentId !== child.agentId || message.pid !== child.process.pid) {
      return failure('Ping outside its active round or process identity');
    }
    const fields = { runId, round: message.round, agentId: child.agentId,
      pid: child.process.pid, seq: message.seq, pingId: message.pingId };
    const before = performance.now();
    const result = gate.screen(message.request);
    const evaluateMs = performance.now() - before;
    log({ event: 'decision', ...fields, intent: message.request.intent, result, evaluateMs });
    pendingResponses += 1;
    child.process.send({ type: 'response', round: message.round, pingId: message.pingId, result }, (error) => {
      // A failed run retains partial evidence. Late callbacks after teardown
      // must not write to its closed descriptor or interrupt failure recording.
      if (gateClosed) return;
      log({ event: error ? 'send_error' : 'response_sent', ...fields });
      pendingResponses -= 1;
      if (error) failure('Gate response IPC send failed');
      if (!pendingResponses) responseDrain?.resolve();
    });
    return;
  }
  if (!phase || message.type !== phase.type || (phase.round !== undefined && message.round !== phase.round)
    || phase.seen.has(child.agentId)) return failure('Unexpected or duplicate worker control message');
  if (message.type === 'ready' && (message.agentId !== child.agentId || message.pid !== child.process.pid)) {
    return failure('Worker roster mismatch');
  }
  if (message.type === 'round_complete') child.lastSummary = message.summary;
  phase.seen.add(child.agentId);
  if (phase.seen.size === agentIds.length) phase.resolve();
}

async function drainResponses() {
  if (fatalError) throw fatalError;
  if (!pendingResponses) return;
  await new Promise((resolvePromise, reject) => {
    const timer = setTimeout(() => failure('Response callback drain timeout'), 30_000);
    responseDrain = { resolve: () => { clearTimeout(timer); responseDrain = null; resolvePromise(); },
      reject: (error) => { clearTimeout(timer); responseDrain = null; reject(error); } };
  });
}

const roundTimings = [];
try {
  const ready = barrier('ready');
  for (const { principalId, token } of credentials) {
    const child = { agentId: principalId, process: fork(join(here, 'agent.mjs'),
      [principalId, join(runDir, 'agents', `${principalId}.jsonl`),
        join(runDir, 'agents', `${principalId}.summary.json`)],
      { stdio: ['ignore', 'ignore', 'ignore', 'ipc'], execArgv: ['--unhandled-rejections=strict'] }) };
    children.push(child);
    completions.push(new Promise((resolveExit) => child.process.once('exit', (code, signal) => {
      if (!stopping || code !== 0 || signal) failure('Worker exited unexpectedly');
      resolveExit({ agentId: child.agentId, pid: child.process.pid, code, signal });
    })));
    child.process.on('error', () => failure('Worker process error'));
    child.process.on('message', (message) => {
      try { receive(child, message); } catch { failure('Gate processing failed'); }
    });
    send(child, { type: 'init', runId, credential: token });
  }
  await ready;
  json('roster.json', children.map(({ agentId, process: worker }) => ({ agentId, pid: worker.pid })));
  process.stdout.write(`Evidence: ${runDir}\n10 agents ready; 100 pings each per round.\n`);
  for (const round of rounds) {
    activeRound = round;
    gate = createSwitchboard(round.registry, { authenticate });
    const prepared = barrier('prepared', round.round);
    children.forEach((child) => send(child, { type: 'prepare', plan: {
      round: round.round, intervalMs: round.intervalMs, count: manifest.pingsPerAgent, action, target } }));
    await prepared;
    const complete = barrier('round_complete', round.round);
    const start = performance.now();
    children.forEach((child) => send(child, { type: 'go', round: round.round }));
    await complete;
    await drainResponses();
    const elapsedMs = performance.now() - start;
    roundTimings.push({ round: round.round, elapsedMs,
      completedPingsPerSecond: 1000 / elapsedMs * manifest.pingsPerAgent * agentIds.length });
    process.stdout.write(`Round ${round.round}: ${round.whitelistCount}/10 whitelisted, ${round.intervalMs} ms interval, ${elapsedMs.toFixed(1)} ms elapsed\n`);
    activeRound = null;
  }
  stopping = true;
  children.forEach((child) => send(child, { type: 'stop', expectedRounds: rounds.length }));
  const exitTimer = setTimeout(() => {
    failure('Worker shutdown timeout');
    children.forEach((child) => { if (child.process.exitCode === null) child.process.kill('SIGTERM'); });
  }, 10_000);
  const exits = await Promise.all(completions);
  clearTimeout(exitTimer);
  if (fatalError) throw fatalError;
  closeSync(fd);
  gateClosed = true;
  json('run-status.json', { completed: true, finishedAt: new Date().toISOString(), roundTimings, exits });
  const verification = await verifyRun(runDir);
  json('verification.json', verification);
  for (const [path, expected] of Object.entries(manifest.sourceSha256)) {
    if (digest(join(apex, path)) !== expected) throw new Error('Source changed during run');
  }
  const hashes = {};
  function collect(dir) {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) collect(path);
      else {
        const bytes = readFileSync(path);
        if (credentials.some(({ token }) => bytes.includes(token))) throw new Error('Credential in evidence');
        hashes[relative(runDir, path)] = createHash('sha256').update(bytes).digest('hex');
      }
    }
  }
  collect(runDir);
  json('credential-scan.json', { filesScanned: Object.keys(hashes).length,
    credentialsChecked: credentials.length, literalCredentialMatches: 0 });
  hashes['credential-scan.json'] = digest(join(runDir, 'credential-scan.json'));
  json('evidence-sha256.json', hashes);
  process.stdout.write(`Reconciliation: ${verification.ok ? 'PASS' : 'FAIL'}; ${verification.issueCount} issues; ${verification.expectedPings} planned pings\n`);
  if (!verification.ok) process.exitCode = 1;
} catch (error) {
  stopping = true;
  children.forEach((child) => { if (child.process.exitCode === null) child.process.kill('SIGTERM'); });
  if (!gateClosed) {
    gateClosed = true;
    closeSync(fd);
  }
  // Persist a bounded harness error, never raw credential-bearing messages.
  json('failure.json', { completed: false, finishedAt: new Date().toISOString(),
    code: fatalError?.message ?? 'HARNESS_FAILURE', round: activeRound?.round ?? null });
  process.stderr.write(`Stress test incomplete; retained evidence at ${runDir}\n`);
  process.exitCode = 1;
}
