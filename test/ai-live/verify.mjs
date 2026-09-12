import { readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';

const dir = resolve(process.argv[2]);
const read = (path) => JSON.parse(readFileSync(join(dir, path), 'utf8'));
const rows = (path) => readFileSync(join(dir, path), 'utf8').trimEnd().split('\n').map((line) => JSON.parse(line));
const manifest = read('manifest.json');
const plan = read('test-plan.json');
const gate = rows('gate.jsonl');
const shutdown = read('shutdown.json');
const issues = [];
const fail = (condition, message) => { if (!condition) issues.push(message); };
const resultFields = ['stage', 'status', 'code', 'reason', 'registry', 'principalId', 'intent', 'grant', 'next'].sort();
fail(shutdown.code === 'COMPLETE' && shutdown.processed === 20 && shutdown.sourceUnchanged
  && shutdown.credentialsRemoved && shutdown.credentialScan.literalMatches === 0, 'Gate shutdown/integrity/credential check failed');
fail(plan.cases.length === 20 && new Set(plan.cases.map((c) => c.requestId)).size === 20, 'Expected twenty unique planned cases');
const observed = manifest.participants.flatMap((id) => rows(`agents/${id}.jsonl`).map((r) => ({ ...r, fileAgent: id })));
fail(gate.length === 22 && gate.filter((r) => r.event === 'decision').length === 20, 'Unexpected gate record count');
fail(observed.length === 40 && observed.filter((r) => r.event === 'submitted').length === 20
  && observed.filter((r) => r.event === 'observed').length === 20, 'Unexpected agent record count');
const installations = gate.filter((r) => r.event === 'registry_installed');
fail(installations.length === 2 && installations.every((r, i) => r.runId === manifest.runId && r.round === i + 1), 'Registry transitions disagree');
let installedRound = null;
for (const row of gate) {
  if (row.event === 'registry_installed') installedRound = row.round;
  else fail(row.event === 'decision' && row.round === installedRound, 'Decision outside its installed registry round');
}
for (const registry of manifest.registries) fail(isDeepStrictEqual(read(`registry-${registry.version}.json`), registry), 'Registry snapshot disagreement');
const results = [];
for (const entry of plan.cases) {
  const match = (all, event) => all.filter((r) => r.event === event && r.requestId === entry.requestId);
  const decisions = match(gate, 'decision');
  const submitted = match(observed, 'submitted');
  const receipts = match(observed, 'observed');
  fail(decisions.length === 1 && submitted.length === 1 && receipts.length === 1, `${entry.requestId}: missing/duplicate evidence`);
  if (decisions.length !== 1 || submitted.length !== 1 || receipts.length !== 1) continue;
  const [decision] = decisions;
  const [receipt] = receipts;
  fail(submitted[0].fileAgent === entry.participantId && receipt.fileAgent === entry.participantId,
    `${entry.requestId}: agent file provenance disagreement`);
  for (const row of [decision, submitted[0], receipt]) {
    fail(row.runId === manifest.runId && row.round === entry.round && row.participantId === entry.participantId
      && row.credentialMode === entry.credentialMode && isDeepStrictEqual(row.intent, entry.intent)
      && (!row.fileAgent || row.fileAgent === entry.participantId), `${entry.requestId}: declaration/identity disagreement`);
  }
  fail(isDeepStrictEqual(decision.result, receipt.result), `${entry.requestId}: gate and agent results disagree`);
  const result = decision.result;
  fail(isDeepStrictEqual(Object.keys(result).sort(), resultFields), `${entry.requestId}: unexpected result fields`);
  const registry = manifest.registries.find((r) => r.version === entry.round);
  const agent = registry.agents.find((a) => a.id === entry.participantId);
  const missing = entry.credentialMode === 'missing';
  const malformed = entry.kind === 'extra' && entry.participantId === 'agent.ai.maple';
  const intentVisible = !missing && agent.whitelisted && !malformed;
  const expected = { stage: 'SWITCHBOARD', status: entry.expectedStatus, code: entry.expectedCode,
    registry: { id: registry.id, version: registry.version },
    principalId: missing ? null : entry.participantId,
    intent: intentVisible ? entry.intent : null,
    grant: entry.expectedStatus === 'PASS' ? agent.grants[0] : null,
    next: entry.expectedStatus === 'PASS' ? 'CONSTRAINT_SCREENING' : null };
  fail(typeof result.reason === 'string' && result.reason.length > 0
    && Object.entries(expected).every(([key, value]) => isDeepStrictEqual(result[key], value)), `${entry.requestId}: result violates expected screening boundary`);
  fail(Number.isFinite(receipt.rttMs) && receipt.rttMs >= 0, `${entry.requestId}: invalid client duration`);
  results.push({ requestId: entry.requestId, participantId: entry.participantId, round: entry.round,
    kind: entry.kind, status: result.status, code: result.code, rttMs: receipt.rttMs });
}
const report = { runId: manifest.runId, ok: issues.length === 0, issueCount: issues.length, issues,
  cases: results, counts: Object.fromEntries(['PASS', 'DENY', 'ERROR'].map((status) => [status, results.filter((r) => r.status === status).length])) };
writeFileSync(join(dir, 'verification.json'), `${JSON.stringify(report, null, 2)}\n`, { flag: 'wx' });
process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
process.exitCode = report.ok ? 0 : 1;
