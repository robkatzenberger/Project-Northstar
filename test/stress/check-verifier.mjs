import assert from 'node:assert/strict';
import { cp, mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { verifyRun } from './verify.mjs';

// Challenge the oracle with copies; never alter the original run evidence.
const source = process.argv.length === 3 ? resolve(process.argv[2]) : null;
assert(source, 'Usage: node test/stress/check-verifier.mjs <completed-run-directory>');
const baseline = await verifyRun(source);
assert.equal(baseline.ok, true, 'Original evidence must reconcile before fault injection');
const manifest = JSON.parse(await readFile(join(source, 'manifest.json'), 'utf8'));
const agentFile = `agents/${manifest.agentIds[0]}.jsonl`;
const editJson = async (dir, name, edit) => {
  const path = join(dir, name);
  const value = JSON.parse(await readFile(path, 'utf8'));
  edit(value);
  await writeFile(path, `${JSON.stringify(value)}\n`);
};
const editRows = async (dir, name, edit) => {
  const path = join(dir, name);
  const rows = (await readFile(path, 'utf8')).trimEnd().split('\n').map(JSON.parse);
  edit(rows);
  await writeFile(path, `${rows.map((row) => JSON.stringify(row)).join('\n')}\n`);
};
const cases = [
  ['missing agent response', (dir) => editRows(dir, agentFile, (rows) => {
    rows.splice(rows.findIndex((row) => row.event === 'response'), 1);
  })],
  ['duplicate gate decision', (dir) => editRows(dir, 'gate.jsonl', (rows) => {
    rows.push(structuredClone(rows.find((row) => row.event === 'decision')));
  })],
  ['matching but incorrect gate and agent decisions', async (dir) => {
    const corrupt = (row) => { row.result.status = 'PASS'; row.result.code = 'INJECTED_WRONG_RESULT'; };
    await editRows(dir, 'gate.jsonl', (rows) => corrupt(rows.find((row) =>
      row.event === 'decision' && row.agentId === manifest.agentIds[0] && row.round === 1 && row.seq === 0)));
    await editRows(dir, agentFile, (rows) => corrupt(rows.find((row) =>
      row.event === 'response' && row.round === 1 && row.seq === 0)));
  }],
  ['wrong process identity', (dir) => editRows(dir, agentFile, (rows) => {
    rows.find((row) => row.event === 'send').pid += 1_000_000;
  })],
  ['registry snapshot differs from plan', (dir) => editJson(dir, 'registry-round-01.json', (value) => {
    value.agents[0].whitelisted = !value.agents[0].whitelisted;
  })],
  ['worker exited unsuccessfully', (dir) => editJson(dir, 'run-status.json', (value) => {
    value.exits[0].code = 1;
  })],
  ['round failed to increase requested pace', (dir) => editJson(dir, 'manifest.json', (value) => {
    value.rounds[1].intervalMs = value.rounds[0].intervalMs;
  })],
  ['agent reports incorrect send schedule', (dir) => editRows(dir, agentFile, (rows) => {
    rows.find((row) => row.event === 'send' && row.seq === 1).scheduledMs += 123;
  })],
  ['agent summary differs from its log', (dir) => editJson(dir, `agents/${manifest.agentIds[0]}.summary.json`, (value) => {
    value.rounds[0].received -= 1;
  })],
  ['gate and agent explanations disagree', (dir) => editRows(dir, agentFile, (rows) => {
    rows.find((row) => row.event === 'response').result.reason = 'Injected mismatch';
  })],
  ['unexpected extra ping', (dir) => editRows(dir, 'gate.jsonl', (rows) => {
    const extra = structuredClone(rows.find((row) => row.event === 'decision'));
    extra.seq = 100;
    extra.pingId = `${manifest.runId}/${extra.round}/${extra.agentId}/100`;
    rows.push(extra);
  })],
  ['malformed final log record', async (dir) => {
    const path = join(dir, agentFile);
    await writeFile(path, `${await readFile(path, 'utf8')}{"event":`);
  }],
];
const results = [];
for (const [name, inject] of cases) {
  const scratch = await mkdtemp(join(tmpdir(), 'apex-verifier-challenge-'));
  try {
    await cp(source, scratch, { recursive: true });
    await inject(scratch);
    const verdict = await verifyRun(scratch);
    results.push({ name, detected: !verdict.ok, issueCount: verdict.issueCount,
      firstIssue: verdict.issues[0] ?? null });
    assert.equal(verdict.ok, false, `Verifier missed: ${name}`);
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
}
process.stdout.write(`${JSON.stringify({ runId: manifest.runId, baselinePassed: true,
  passed: results.length, total: cases.length, results }, null, 2)}\n`);
