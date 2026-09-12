import assert from 'node:assert/strict';
import { createHash, randomBytes, randomUUID } from 'node:crypto';
import { mkdtemp, readFile, writeFile, rename, mkdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { createAuditorStore } from '../src/auditor-store.js';
import { createGate, parseRules } from '../src/gate.js';
import { createTokenAuthenticator } from '../src/switchboard.js';

const model = 'approve_escalate_v1';
const digest = (text) => createHash('sha256').update(text).digest('hex');
const registry = {
  id: 'review.fixture', version: 1,
  agents: [
    { id: 'agent.worker', whitelisted: true, grants: [
      { id: 'grant.read', action: 'read_file', target: 'file:demo', issuedBy: 'operator:owner' },
      { id: 'grant.unknown', action: 'unknown_action', target: 'file:demo', issuedBy: 'operator:owner' },
    ] },
    { id: 'agent.paused', whitelisted: false, grants: [] },
  ],
};
const declaration = { agentId: 'agent.worker', action: 'read_file', target: 'file:demo' };

async function temporary(t) {
  const directory = await mkdtemp(join(tmpdir(), 'apex-human-review-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  return directory;
}

async function setup(t) {
  const directory = await temporary(t);
  const store = await createAuditorStore(directory, registry);
  const workerToken = randomBytes(32).toString('hex');
  const pausedToken = randomBytes(32).toString('hex');
  const authenticate = createTokenAuthenticator([
    { principalId: 'agent.worker', token: workerToken },
    { principalId: 'agent.paused', token: pausedToken },
  ]);
  const assess = (intent = declaration, credential = workerToken) => store.assess(
    ({ version, text }, snapshot) => createGate({ registry: snapshot, authenticate, policy: { version, text } })
      .evaluate({ credential, intent }), intent,
  );
  return { store, directory, path: join(directory, 'audit.jsonl'), assess, pausedToken };
}

const decision = (assessmentId, choice = 'APPROVE', comment = 'Verified the requested action and exact target.') =>
  ({ assessmentId, decision: choice, comment });

test('review records exact immutable linkage and context, and survives later policy edits and restart', async (t) => {
  const { store, directory, path, assess } = await setup(t);
  assert.equal(store.getPolicy().decisionModel, model);
  const assessed = await assess();
  assert.equal(assessed.status, 'ESCALATE');
  const before = await readFile(path);
  const original = store.history(100).find(({ id }) => id === assessed.audit.id);
  const receipt = await store.resolveReview(decision(assessed.audit.id, 'APPROVE', '  Verified file:demo with its owner.  '));
  assert.equal(receipt.comment, 'Verified file:demo with its owner.');
  assert.equal(receipt.reviewer, 'operator');
  assert.equal(receipt.actor, 'operator');
  assert.equal(receipt.execution, 'NOT_EXECUTED');
  assert.equal(receipt.assessmentHash, digest(JSON.stringify({ request: original.request, result: original.result })));
  assert.equal((await readFile(path)).subarray(0, before.length).equals(before), true);
  assert.deepEqual(store.history(100).find(({ id }) => id === assessed.audit.id), original);
  assert.equal(store.listReviews()[0].result.status, 'ESCALATE');
  assert.equal(store.listReviews()[0].review.decision, 'APPROVE');
  assessed.switchboard.intent.target = 'file:changed';
  receipt.comment = 'changed outside store';
  const visible = store.listReviews();
  visible[0].request.target = 'file:changed';
  assert.equal(store.listReviews()[0].request.target, 'file:demo');
  assert.equal(store.listReviews()[0].review.comment, 'Verified file:demo with its owner.');
  await store.savePolicy(1, 'ESCALATE read_file file:demo\n');
  const reopened = await createAuditorStore(directory, registry);
  assert.deepEqual(reopened.listReviews(), store.listReviews());
  assert.deepEqual(reopened.history(100), store.history(100));
});

test('required context, exact request fields, reviewer ownership, and UTF-8 byte bounds are enforced', async (t) => {
  const { store, path, assess } = await setup(t);
  const assessed = await assess();
  const before = await readFile(path, 'utf8');
  const invalid = [
    ...['', ' \n\t', 'a'.repeat(2001), '\ud800', '😀'.repeat(501)].map((comment) => decision(assessed.audit.id, 'APPROVE', comment)),
    { assessmentId: assessed.audit.id, decision: 'APPROVE' },
    { ...decision(assessed.audit.id), reviewer: 'agent.worker' },
    { ...decision(assessed.audit.id), target: 'file:other' },
    decision(assessed.audit.id, 'ALLOW'),
    decision(assessed.audit.id, 'ESCALATE'),
    { get assessmentId() { throw new Error('Accessor must not run.'); }, decision: 'APPROVE', comment: 'Context.' },
  ];
  for (const input of invalid) await assert.rejects(store.resolveReview(input), { code: 'INVALID_REVIEW', status: 400 });
  await assert.rejects(store.resolveReview(decision('missing-assessment')), { code: 'ASSESSMENT_NOT_FOUND', status: 404 });
  assert.equal(await readFile(path, 'utf8'), before);
  assert.equal(store.listReviews()[0].review, null);
  assert.equal((await store.resolveReview(decision(assessed.audit.id, 'DENY', '😀'.repeat(500)))).comment.length, 1000);
});

test('human review cannot override failed identity, whitelist, scope, invalid catalog, or automatic approval', async (t) => {
  const { store, assess, pausedToken } = await setup(t);
  const blocked = [
    await assess(declaration, ''),
    await assess({ ...declaration, agentId: 'agent.paused' }, pausedToken),
    await assess({ ...declaration, agentId: 'agent.other' }),
    await assess({ ...declaration, target: 'file:other' }),
    await assess({ ...declaration, action: 'unknown_action' }),
  ];
  assert.deepEqual(blocked.map(({ status }) => status), ['BLOCKED', 'BLOCKED', 'BLOCKED', 'BLOCKED', 'ERROR']);
  await store.savePolicy(1, 'APPROVE read_file file:demo\n');
  const approved = await assess();
  assert.equal(approved.status, 'APPROVE');
  for (const assessment of [...blocked, approved]) {
    await assert.rejects(store.resolveReview(decision(assessment.audit.id)), { code: 'REVIEW_NOT_ALLOWED', status: 409 });
  }
  assert.deepEqual(store.listReviews(), []);
});

test('approval requires the original policy and registry; stale assessments can still be denied', async (t) => {
  const changes = [
    ['policy version, even with unchanged rules', (store) => store.savePolicy(1, store.getPolicy().text)],
    ['policy text', (store) => store.savePolicy(1, 'APPROVE read_file file:demo\n')],
    ['whitelist revocation', (store) => {
      const next = store.getRegistry(); next.agents[0].whitelisted = false; return store.saveRegistry(next);
    }],
    ['grant removal', (store) => {
      const next = store.getRegistry(); next.agents[0].grants = []; return store.saveRegistry(next);
    }],
    ['grant issuer change', (store) => {
      const next = store.getRegistry(); next.agents[0].grants[0].issuedBy = 'operator:replacement'; return store.saveRegistry(next);
    }],
    ['registry identity change', (store) => {
      const next = store.getRegistry(); next.id = 'review.changed'; return store.saveRegistry(next);
    }],
  ];
  for (const [label, change] of changes) await t.test(label, async (sub) => {
    const { store, directory, path, assess } = await setup(sub);
    const assessment = await assess();
    await change(store);
    const before = await readFile(path, 'utf8');
    await assert.rejects(store.resolveReview(decision(assessment.audit.id)), { code: 'REVIEW_STALE', status: 409 });
    assert.equal(await readFile(path, 'utf8'), before);
    assert.equal(store.listReviews()[0].review, null);
    await store.resolveReview(decision(assessment.audit.id, 'DENY', 'Authority changed; a new assessment is required.'));
    assert.equal((await createAuditorStore(directory, registry)).listReviews()[0].review.decision, 'DENY');
  });
});

test('matching registry labels alone cannot substitute for the exact original grant or principal', async (t) => {
  for (const alteration of ['grant', 'principal']) await t.test(alteration, async (sub) => {
    const { store, assess } = await setup(sub);
    const actual = await assess();
    const { audit, ...result } = actual;
    if (alteration === 'grant') result.switchboard.grant.issuedBy = 'operator:forged';
    else {
      result.switchboard.principalId = 'agent.paused';
      result.switchboard.intent.agentId = 'agent.paused';
    }
    const record = await store.assess(() => result, result.switchboard.intent);
    await assert.rejects(store.resolveReview(decision(record.audit.id)), { code: 'REVIEW_STALE', status: 409 });
  });
});

test('concurrent decisions have one winner and cannot be repeated after restart', async (t) => {
  const { store, directory, path, assess } = await setup(t);
  const assessment = await assess();
  const outcomes = await Promise.allSettled([
    store.resolveReview(decision(assessment.audit.id, 'APPROVE')),
    store.resolveReview(decision(assessment.audit.id, 'DENY', 'Do not proceed.')),
  ]);
  assert.equal(outcomes.filter(({ status }) => status === 'fulfilled').length, 1);
  const rejected = outcomes.find(({ status }) => status === 'rejected');
  assert.equal(rejected.reason.code, 'REVIEW_ALREADY_RESOLVED');
  const rows = (await readFile(path, 'utf8')).trim().split('\n').map(JSON.parse);
  assert.equal(rows.filter(({ type }) => type === 'human_review').length, 1);
  const reopened = await createAuditorStore(directory, registry);
  await assert.rejects(reopened.resolveReview(decision(assessment.audit.id, 'DENY')), { code: 'REVIEW_ALREADY_RESOLVED', status: 409 });
});

test('queued revocation is observed before approval at the same serialization point', async (t) => {
  const { store, assess } = await setup(t);
  const assessment = await assess();
  let release;
  const held = new Promise((resolve) => { release = resolve; });
  const blocker = store.assess(async () => { await held; return { status: 'ERROR' }; }, null);
  const next = store.getRegistry();
  next.agents[0].whitelisted = false;
  const revoke = store.saveRegistry(next);
  const attempted = store.resolveReview(decision(assessment.audit.id));
  const rejected = assert.rejects(attempted, { code: 'REVIEW_STALE', status: 409 });
  release();
  await Promise.all([blocker, revoke, rejected]);
  assert.equal(store.listReviews()[0].review, null);
});

test('review list is capped at 100 with oldest pending first and resolved entries afterward', async (t) => {
  const { store, assess } = await setup(t);
  const ids = [];
  for (let index = 0; index < 102; index += 1) ids.push((await assess()).audit.id);
  assert.equal(store.listReviews().length, 100);
  assert.deepEqual(store.listReviews().map(({ assessmentId }) => assessmentId), ids.slice(0, 100));
  for (const id of ids.slice(0, 4)) await store.resolveReview(decision(id, 'DENY', 'Closed during review.'));
  const list = store.listReviews();
  assert.deepEqual(list.slice(0, 98).map(({ assessmentId }) => assessmentId), ids.slice(4));
  assert.equal(list[0].review, null);
  assert.equal(list[98].review.decision, 'DENY');
});

function legacyJournal() {
  const text = 'DENY read_file file:demo\nALLOW delete_file file:demo\nREQUIRE_APPROVAL send_email recipient:demo\n';
  const metadata = () => ({ id: randomUUID(), recordedAt: '2026-09-09T12:00:00.000Z' });
  const oldAssessment = { ...metadata(), type: 'assessment', request: declaration,
    result: { version: '0.0.1', status: 'REQUIRE_APPROVAL', stage: 'GATE', execution: 'NOT_EXECUTED' } };
  const rows = [
    { ...metadata(), type: 'policy_saved', actor: 'operator', policy: { version: 1, text, hash: digest(text) } },
    { ...metadata(), type: 'registry_saved', actor: 'operator', registry, hash: digest(JSON.stringify(registry)) },
    oldAssessment,
  ];
  return { source: rows.map((row) => `${JSON.stringify(row)}\n`).join(''), oldAssessment, rows };
}

test('legacy migration appends a new empty escalation policy without relabeling old decisions or rewriting bytes', async (t) => {
  const directory = await temporary(t);
  const path = join(directory, 'audit.jsonl');
  const legacy = legacyJournal();
  await writeFile(path, legacy.source);
  const store = await createAuditorStore(directory, registry);
  const after = await readFile(path, 'utf8');
  assert.equal(after.startsWith(legacy.source), true);
  assert.equal(after.slice(legacy.source.length).trim().split('\n').length, 1);
  const event = JSON.parse(after.slice(legacy.source.length));
  assert.equal(event.actor, 'operator:model-migration');
  assert.equal(event.type, 'policy_saved');
  assert.equal(store.getPolicy().version, 2);
  assert.equal(store.getPolicy().decisionModel, model);
  assert.deepEqual(parseRules(store.getPolicy().text), []);
  assert.deepEqual(store.listReviews(), []);
  await assert.rejects(store.resolveReview(decision(legacy.oldAssessment.id)), { code: 'REVIEW_NOT_ALLOWED' });
  assert.deepEqual(store.history(100).find(({ id }) => id === legacy.oldAssessment.id), legacy.oldAssessment);
  await createAuditorStore(directory, registry);
  assert.equal(await readFile(path, 'utf8'), after);
  const { version, text } = store.getPolicy();
  const result = createGate({ registry, authenticate: () => 'agent.worker', policy: { version, text } })
    .evaluate({ credential: 'test-only', intent: declaration });
  assert.equal(result.status, 'ESCALATE');
});

test('replay rejects corrupt review linkage, duplicates, actor, context, and unsupported policy models without rewriting', async (t) => {
  const { store, path, assess } = await setup(t);
  const assessment = await assess();
  await store.resolveReview(decision(assessment.audit.id));
  const original = (await readFile(path, 'utf8')).trim().split('\n').map(JSON.parse);
  const variants = [
    ['decision', (rows) => { rows.at(-1).decision = 'ALLOW'; }],
    ['missing context', (rows) => { rows.at(-1).comment = ''; }],
    ['untrimmed context', (rows) => { rows.at(-1).comment = ' Context '; }],
    ['actor', (rows) => { rows.at(-1).actor = 'agent.worker'; }],
    ['reviewer', (rows) => { rows.at(-1).reviewer = 'agent.worker'; }],
    ['different assessment', (rows) => { rows.at(-1).assessmentId = 'not-an-assessment'; }],
    ['different content', (rows) => { rows.find(({ type }) => type === 'assessment').request.target = 'file:changed'; }],
    ['wrong hash', (rows) => { rows.at(-1).assessmentHash = '0'.repeat(64); }],
    ['extra field', (rows) => { rows.at(-1).target = 'file:other'; }],
    ['duplicate review', (rows) => { rows.push({ ...rows.at(-1), id: randomUUID() }); }],
    ['legacy assessment', (rows) => { delete rows.find(({ type }) => type === 'assessment').result.decisionModel; }],
    ['unsupported model', (rows) => { rows[0].policy.decisionModel = 'unknown'; }],
    ['unmarked new policy syntax', (rows) => {
      delete rows[0].policy.decisionModel;
      rows[0].policy.text = 'APPROVE read_file file:demo\n'; rows[0].policy.hash = digest(rows[0].policy.text);
    }],
  ];
  for (const [label, mutate] of variants) await t.test(label, async (sub) => {
    const directory = await temporary(sub);
    const candidate = structuredClone(original);
    mutate(candidate);
    const source = candidate.map((row) => `${JSON.stringify(row)}\n`).join('');
    const candidatePath = join(directory, 'audit.jsonl');
    await writeFile(candidatePath, source);
    await assert.rejects(createAuditorStore(directory, registry), { code: 'AUDIT_CORRUPT', status: 503 });
    assert.equal(await readFile(candidatePath, 'utf8'), source);
  });
});

test('replay rejects approval that was stale at the point it was recorded', async (t) => {
  const { store, path, assess } = await setup(t);
  const assessment = await assess();
  await store.resolveReview(decision(assessment.audit.id));
  await store.savePolicy(1, 'ESCALATE read_file file:demo\n');
  const rows = (await readFile(path, 'utf8')).trim().split('\n').map(JSON.parse);
  const approvalIndex = rows.findIndex(({ type }) => type === 'human_review');
  const [approval] = rows.splice(approvalIndex, 1);
  rows.push(approval);
  const directory = await temporary(t);
  await writeFile(join(directory, 'audit.jsonl'), rows.map((row) => `${JSON.stringify(row)}\n`).join(''));
  await assert.rejects(createAuditorStore(directory, registry), { code: 'AUDIT_CORRUPT' });
});

test('failed review persistence leaves the assessment pending and recoverable after storage repair', async (t) => {
  const { store, directory, path, assess } = await setup(t);
  const assessment = await assess();
  const before = await readFile(path, 'utf8');
  const retained = join(directory, 'retained.jsonl');
  await rename(path, retained);
  await mkdir(path);
  await assert.rejects(store.resolveReview(decision(assessment.audit.id)), { code: 'AUDIT_UNAVAILABLE', status: 503 });
  assert.equal(store.listReviews()[0].review, null);
  assert.equal(store.history(100).some(({ type }) => type === 'human_review'), false);
  await assert.rejects(store.resolveReview(decision(assessment.audit.id, 'DENY')), { code: 'AUDIT_UNAVAILABLE' });
  assert.equal(await readFile(retained, 'utf8'), before);
  await rm(path, { recursive: true });
  await rename(retained, path);
  const reopened = await createAuditorStore(directory, registry);
  assert.equal(reopened.listReviews()[0].review, null);
  await reopened.resolveReview(decision(assessment.audit.id, 'DENY', 'Storage restored; request declined.'));
  assert.equal(reopened.listReviews()[0].review.decision, 'DENY');
});

test('migration fails closed at journal capacity and preserves every legacy byte', async (t) => {
  const directory = await temporary(t);
  const path = join(directory, 'audit.jsonl');
  const { source } = legacyJournal();
  const filler = { id: randomUUID(), recordedAt: '2026-09-09T12:00:00.000Z', type: 'credential_issued',
    principalId: 'agent.worker', ephemeral: true, padding: '' };
  filler.padding = 'x'.repeat(16 * 1024 * 1024 - Buffer.byteLength(source) - Buffer.byteLength(`${JSON.stringify(filler)}\n`));
  const full = `${source}${JSON.stringify(filler)}\n`;
  await writeFile(path, full);
  await assert.rejects(createAuditorStore(directory, registry), { code: 'AUDIT_CAPACITY', status: 503 });
  assert.equal(await readFile(path, 'utf8'), full);
});
