import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import {
  createSamplePending,
  decide,
  defaultListView,
  filterRecords,
  hashIntentSummary,
  loadDecisions,
  loadPending,
  seedPendingIfMissing,
  sortPendingFirst,
} from '../src/audit-store.js';

async function temporaryDataDir(seed = true) {
  const directory = await mkdtemp(join(tmpdir(), 'apex-audit-'));
  if (seed) {
    await seedPendingIfMissing(directory, [
      {
        requestId: 'req-one',
        status: 'PENDING',
        at: '2026-09-09T18:00:00.000Z',
        principalId: 'agent-docs',
        action: 'documents.read',
        target: 'northstar://documents/demo',
        intentSummary: 'Read the demo document.',
      },
      {
        requestId: 'req-denied',
        status: 'DENIED',
        at: '2026-09-09T17:00:00.000Z',
        principalId: 'agent-export',
        action: 'documents.export',
        target: 'northstar://documents/secret',
        intentSummary: 'Export secret documents.',
      },
      {
        requestId: 'req-approved',
        status: 'APPROVED',
        at: '2026-09-09T16:00:00.000Z',
        principalId: 'agent-docs',
        action: 'documents.list',
        target: 'northstar://documents',
        intentSummary: 'List documents.',
      },
    ]);
  }
  return directory;
}

test('1. approve appends one line; prior decision bytes unchanged', async () => {
  const dataDir = await temporaryDataDir();
  const decisionsPath = join(dataDir, 'decisions.jsonl');
  await writeFile(decisionsPath, '', { flag: 'a' });
  const prior = await readFile(decisionsPath);

  const pending = await loadPending(dataDir);
  const target = pending.find((item) => item.requestId === 'req-one');
  const result = await decide(
    dataDir,
    'req-one',
    'APPROVE',
    'operator-test',
    hashIntentSummary(target.intentSummary),
  );

  const after = await readFile(decisionsPath);
  assert.equal(after.subarray(0, prior.length).equals(prior), true);
  const lines = after.toString('utf8').trim().split('\n').filter(Boolean);
  assert.equal(lines.length, 1);
  const receipt = JSON.parse(lines[0]);
  assert.equal(receipt.requestId, 'req-one');
  assert.equal(receipt.status, 'APPROVED');
  assert.equal(receipt.operatorId, 'operator-test');
  assert.equal(result.receipt.id, receipt.id);

  const updated = await loadPending(dataDir);
  assert.equal(updated.find((item) => item.requestId === 'req-one').status, 'APPROVED');
});

test('2. invalid decide writes nothing', async () => {
  const dataDir = await temporaryDataDir();
  const pendingPath = join(dataDir, 'pending.jsonl');
  const decisionsPath = join(dataDir, 'decisions.jsonl');
  const pendingBefore = await readFile(pendingPath, 'utf8');
  const decisionsBefore = await readFile(decisionsPath, 'utf8');

  await assert.rejects(
    decide(dataDir, 'req-one', 'APPROVE', 'operator-test', 'deadbeef'),
    /intentSummaryHash/,
  );
  await assert.rejects(
    decide(dataDir, 'missing', 'APPROVE', 'operator-test', 'a'.repeat(64)),
    /Unknown requestId/,
  );
  await assert.rejects(
    decide(dataDir, 'req-one', 'MAYBE', 'operator-test', hashIntentSummary('Read the demo document.')),
    /APPROVE or DENY/,
  );

  assert.equal(await readFile(pendingPath, 'utf8'), pendingBefore);
  assert.equal(await readFile(decisionsPath, 'utf8'), decisionsBefore);
});

test('3. double-decide rejected', async () => {
  const dataDir = await temporaryDataDir();
  const pending = await loadPending(dataDir);
  const target = pending.find((item) => item.requestId === 'req-one');
  const hash = hashIntentSummary(target.intentSummary);

  await decide(dataDir, 'req-one', 'DENY', 'operator-test', hash);
  const decisionsPath = join(dataDir, 'decisions.jsonl');
  const afterFirst = await readFile(decisionsPath, 'utf8');

  await assert.rejects(
    decide(dataDir, 'req-one', 'APPROVE', 'operator-test', hash),
    /second decide rejected/,
  );
  assert.equal(await readFile(decisionsPath, 'utf8'), afterFirst);
  const lines = afterFirst.trim().split('\n').filter(Boolean);
  assert.equal(lines.length, 1);
  assert.equal(JSON.parse(lines[0]).status, 'DENIED');
});

test('4. corrupt pending refuses load and decide', async () => {
  const dataDir = await mkdtemp(join(tmpdir(), 'apex-audit-corrupt-'));
  const pendingPath = join(dataDir, 'pending.jsonl');
  const decisionsPath = join(dataDir, 'decisions.jsonl');
  const corrupt = '{"requestId":"x","status":"PENDING"\n';
  await writeFile(pendingPath, corrupt);
  await writeFile(decisionsPath, '');

  await assert.rejects(loadPending(dataDir), /invalid JSON/);
  await assert.rejects(
    decide(dataDir, 'x', 'APPROVE', 'operator-test', 'a'.repeat(64)),
    /invalid JSON/,
  );
  assert.equal(await readFile(pendingPath, 'utf8'), corrupt);
  assert.equal(await readFile(decisionsPath, 'utf8'), '');
});

test('5. default filter is Pending-first and Denied stays visible', () => {
  const records = [
    {
      requestId: 'a',
      status: 'APPROVED',
      at: '2026-09-09T20:00:00.000Z',
      principalId: 'agent-a',
      action: 'documents.list',
      target: 't',
      intentSummary: 'list',
    },
    {
      requestId: 'd',
      status: 'DENIED',
      at: '2026-09-09T19:00:00.000Z',
      principalId: 'agent-d',
      action: 'documents.export',
      target: 't',
      intentSummary: 'export',
    },
    {
      requestId: 'p',
      status: 'PENDING',
      at: '2026-09-09T18:00:00.000Z',
      principalId: 'agent-p',
      action: 'documents.read',
      target: 't',
      intentSummary: 'read',
    },
    {
      requestId: 'e',
      status: 'ERROR',
      at: '2026-09-09T17:00:00.000Z',
      principalId: 'agent-e',
      action: 'system.reboot',
      target: 't',
      intentSummary: 'reboot',
    },
  ];

  const sorted = sortPendingFirst(records);
  assert.deepEqual(
    sorted.map((item) => item.status),
    ['PENDING', 'ERROR', 'DENIED', 'APPROVED'],
  );

  const defaults = defaultListView(records);
  assert.equal(defaults.some((item) => item.status === 'DENIED'), true);
  assert.equal(defaults[0].status, 'PENDING');

  const deniedOnly = filterRecords(records, { status: 'DENIED' });
  assert.equal(deniedOnly.length, 1);
  assert.equal(deniedOnly[0].requestId, 'd');

  const byAgent = filterRecords(records, { agentId: 'agent-p' });
  assert.equal(byAgent.length, 1);
  assert.equal(byAgent[0].requestId, 'p');
});

test('6. round-trip reload after decide', async () => {
  const dataDir = await temporaryDataDir();
  const pending = await loadPending(dataDir);
  const target = pending.find((item) => item.requestId === 'req-one');
  const { receipt } = await decide(
    dataDir,
    'req-one',
    'APPROVE',
    'operator-roundtrip',
    hashIntentSummary(target.intentSummary),
  );

  const reloadedPending = await loadPending(dataDir);
  const reloadedDecisions = await loadDecisions(dataDir);
  assert.equal(reloadedPending.find((item) => item.requestId === 'req-one').status, 'APPROVED');
  assert.equal(reloadedDecisions.length, 1);
  assert.deepEqual(reloadedDecisions[0], receipt);
});

test('sample seed creates pending and empty decisions', async () => {
  const dataDir = await mkdtemp(join(tmpdir(), 'apex-audit-seed-'));
  await seedPendingIfMissing(dataDir, createSamplePending());
  const pending = await loadPending(dataDir);
  const decisions = await loadDecisions(dataDir);
  assert.equal(pending.length >= 4, true);
  assert.equal(pending.some((item) => item.status === 'PENDING'), true);
  assert.equal(pending.some((item) => item.status === 'DENIED'), true);
  assert.deepEqual(decisions, []);
});
