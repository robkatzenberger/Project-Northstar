import assert from 'node:assert/strict';
import { randomBytes } from 'node:crypto';
import { mkdtemp, readFile, writeFile, rm, rename, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { createAuditorServer } from '../src/server.js';
import { createAuditorStore } from '../src/auditor-store.js';

const registry = JSON.parse(await readFile(new URL('../examples/auditor-agents.json', import.meta.url), 'utf8'));
const rules = 'APPROVE read_file file:demo\nESCALATE delete_file file:demo\nESCALATE send_email recipient:demo\n';

async function setup(t) {
  const directory = await mkdtemp(join(tmpdir(), 'apex-auditor-test-'));
  const registryPath = join(directory, 'seed.json');
  const dataDir = join(directory, 'data');
  const operatorKey = randomBytes(32).toString('hex');
  await writeFile(registryPath, JSON.stringify(registry));
  const running = new Set();
  async function start() {
    const app = await createAuditorServer({ dataDir, registryPath, operatorKey });
    running.add(app.server);
    await new Promise((resolve, reject) => {
      app.server.once('error', reject);
      app.server.listen(0, '127.0.0.1', resolve);
    });
    const base = `http://127.0.0.1:${app.server.address().port}`;
    async function request(path, { method = 'GET', input, operator = false, headers = {} } = {}) {
      const response = await fetch(base + path, { method,
        headers: { ...(input !== undefined ? { 'content-type': 'application/json' } : {}),
          ...(operator ? { authorization: `Bearer ${operatorKey}` } : {}), ...headers },
        ...(input !== undefined ? { body: JSON.stringify(input) } : {}) });
      return { status: response.status, headers: response.headers, body: await response.json() };
    }
    return { ...app, base, request, close: async () => {
      app.server.closeAllConnections();
      await new Promise((resolve, reject) => app.server.close((error) => error ? reject(error) : resolve()));
      running.delete(app.server);
    } };
  }
  t.after(async () => {
    for (const server of running) {
      server.closeAllConnections();
      await new Promise((resolve) => server.close(resolve));
    }
    await rm(directory, { recursive: true, force: true });
  });
  return { start, directory, dataDir, operatorKey };
}

test('auditor HTTP separates operator authority, checks real agents, and records every returned assessment', async (t) => {
  const env = await setup(t);
  const app = await env.start();
  const request = app.request;
  assert.equal((await request('/api/gate-rules')).status, 401);
  assert.equal((await request('/api/audit')).status, 401);
  assert.equal((await request('/api/gate-rules', { method: 'PUT', input: { version: 1, text: rules } })).status, 401);
  assert.equal((await request('/api/operator/session', { method: 'POST', input: { key: 'wrong' } })).status, 401);
  const login = await request('/api/operator/session', { method: 'POST', input: { key: env.operatorKey } });
  assert.equal(login.status, 200);
  assert.match(login.headers.get('set-cookie'), /HttpOnly; SameSite=Strict/);
  const cookie = login.headers.get('set-cookie').split(';')[0];
  assert.equal((await request('/api/gate-rules', { headers: { cookie } })).status, 200);
  assert.equal((await request('/api/gate-rules', { operator: true, headers: { origin: 'https://untrusted.example' } })).status, 403);
  const page = await fetch(app.base);
  assert.equal(page.status, 200);
  assert.match(await page.text(), /Apex Auditor/);

  const issued = await request('/api/gate/credentials', { operator: true, method: 'POST', input: { agentId: 'demo-agent' } });
  assert.equal(issued.status, 201);
  const credential = issued.body.credential;
  assert.equal((await request('/api/gate-rules', { headers: { authorization: `Bearer ${credential}` } })).status, 401);
  const evaluate = (action, target = 'file:demo', cred = credential) => request('/api/gate/evaluate', {
    method: 'POST', input: { credential: cred, intent: { agentId: 'demo-agent', action, target } },
  });
  const initial = await evaluate('read_file');
  assert.equal(initial.body.code, 'NO_MATCHING_RULE');
  const saved = await request('/api/gate-rules', { operator: true, method: 'PUT', input: { version: 1, text: rules } });
  assert.equal(saved.status, 200);
  assert.equal(saved.body.version, 2);
  const outcomes = [
    await evaluate('read_file'), await evaluate('delete_file'), await evaluate('send_email', 'recipient:demo'),
    await evaluate('write_file'), await evaluate('read_file', 'file:other'), await evaluate('read_file', 'file:demo', ''),
  ];
  assert.deepEqual(outcomes.map((r) => [r.status, r.body.status, r.body.code]), [
    [200, 'APPROVE', 'RULE_APPROVE'], [200, 'ESCALATE', 'HUMAN_VERIFICATION_REQUIRED'], [200, 'ESCALATE', 'HUMAN_VERIFICATION_REQUIRED'],
    [200, 'ESCALATE', 'NO_MATCHING_RULE'], [200, 'BLOCKED', 'SCOPE_NOT_GRANTED'], [200, 'BLOCKED', 'UNAUTHENTICATED'],
  ]);
  for (const result of outcomes) {
    assert.equal(result.body.execution, 'NOT_EXECUTED');
    assert.equal(result.body.policy.version, 2);
    assert.ok(result.body.audit.id);
  }
  const overlap = await evaluate('read_file', credential);
  assert.equal(overlap.body.status, 'ERROR');
  assert.equal(JSON.stringify(overlap.body).includes(credential), false);
  const invalid = await request('/api/gate-rules', { operator: true, method: 'PUT', input: { version: 2, text: 'APPROVE read_file *\n' } });
  assert.equal(invalid.status, 400);
  assert.equal(invalid.body.line, 1);
  assert.equal((await request('/api/gate-rules', { operator: true })).body.text, rules);
  const history = (await request('/api/audit?limit=100', { operator: true })).body.records;
  for (const result of [initial, ...outcomes, overlap]) {
    const record = history.find((r) => r.id === result.body.audit.id);
    assert.ok(record);
    const { audit, ...assessment } = result.body;
    assert.deepEqual(record.result, assessment);
  }
  const raw = await readFile(join(env.dataDir, 'audit.jsonl'), 'utf8');
  assert.equal(raw.includes(credential), false);
  assert.equal(raw.includes(env.operatorKey), false);
});

test('rule edits conflict safely, registry revocation activates atomically, and journal survives restart', async (t) => {
  const env = await setup(t);
  let app = await env.start();
  const writes = await Promise.all(['APPROVE read_file file:demo\n', 'ESCALATE read_file file:demo\n'].map((text) =>
    app.request('/api/gate-rules', { method: 'PUT', operator: true, input: { version: 1, text } })));
  assert.deepEqual(writes.map((r) => r.status).sort(), [200, 409]);
  const current = (await app.request('/api/gate-rules', { operator: true })).body;
  await app.request('/api/gate-rules', { method: 'PUT', operator: true, input: { version: current.version, text: rules } });
  const credential = (await app.request('/api/gate/credentials', { method: 'POST', operator: true, input: { agentId: 'demo-agent' } })).body.credential;
  const input = { credential, intent: { agentId: 'demo-agent', action: 'read_file', target: 'file:demo' } };
  assert.equal((await app.request('/api/gate/evaluate', { method: 'POST', input })).body.status, 'APPROVE');
  const change = (await app.request('/api/registry', { operator: true })).body.registry;
  change.agents[0].whitelisted = false;
  assert.equal((await app.request('/api/registry', { operator: true, method: 'PUT', input: { registry: change } })).status, 200);
  assert.equal((await app.request('/api/gate/evaluate', { method: 'POST', input })).body.code, 'NOT_WHITELISTED');
  assert.equal((await app.request('/api/registry', { operator: true, method: 'PUT', input: { registry: change } })).status, 409);
  const before = (await app.request('/api/audit?limit=100', { operator: true })).body.records;
  await app.close();
  app = await env.start();
  assert.equal((await app.request('/api/gate-rules', { operator: true })).body.text, rules);
  assert.equal((await app.request('/api/registry', { operator: true })).body.registry.agents[0].whitelisted, false);
  assert.deepEqual((await app.request('/api/audit?limit=100', { operator: true })).body.records, before);
  assert.equal((await app.request('/api/gate/evaluate', { method: 'POST', input })).body.code, 'UNAUTHENTICATED');
});

test('HTTP human review is operator-only, binds a saved escalation, and keeps context across restart', async (t) => {
  const env = await setup(t);
  let app = await env.start();
  const credential = (await app.request('/api/gate/credentials', { method: 'POST', operator: true,
    input: { agentId: 'demo-agent' } })).body.credential;
  const evaluate = (target = 'file:demo') => app.request('/api/gate/evaluate', { method: 'POST',
    input: { credential, intent: { agentId: 'demo-agent', action: 'read_file', target } } });
  const escalation = (await evaluate()).body;
  assert.equal(escalation.status, 'ESCALATE');
  const blocked = (await evaluate('file:other')).body;
  assert.equal(blocked.status, 'BLOCKED');
  assert.equal((await app.request('/api/reviews')).status, 401);
  assert.equal((await app.request('/api/reviews', { headers: { authorization: `Bearer ${credential}` } })).status, 401);
  const input = { assessmentId: escalation.audit.id, decision: 'APPROVE', comment: 'I verified this exact demo file scope.' };
  assert.equal((await app.request('/api/reviews/decision', { method: 'POST', input,
    headers: { authorization: `Bearer ${credential}` } })).status, 401);
  const pending = (await app.request('/api/reviews', { operator: true })).body.reviews;
  assert.deepEqual(pending.map((review) => review.assessmentId), [escalation.audit.id]);
  assert.equal(pending[0].review, null);
  assert.equal((await app.request('/api/reviews/decision', { method: 'POST', operator: true,
    input: { ...input, comment: '  ' } })).status, 400);
  assert.equal((await app.request('/api/reviews/decision', { method: 'POST', operator: true,
    input: { ...input, reviewer: 'agent:pretending-to-be-human' } })).status, 400);
  assert.equal((await app.request('/api/reviews/decision', { method: 'POST', operator: true,
    input: { ...input, assessmentId: blocked.audit.id } })).status, 409);
  const decision = await app.request('/api/reviews/decision', { method: 'POST', operator: true, input });
  assert.equal(decision.status, 201);
  assert.equal(decision.body.review.decision, 'APPROVE');
  assert.equal(decision.body.review.comment, input.comment);
  assert.equal(decision.body.review.reviewer, 'operator');
  assert.equal(decision.body.review.execution, 'NOT_EXECUTED');
  assert.equal((await app.request('/api/reviews/decision', { method: 'POST', operator: true, input })).status, 409);
  const denial = (await evaluate()).body;
  const denialInput = { assessmentId: denial.audit.id, decision: 'DENY', comment: '<script>context only</script>' };
  const denied = await app.request('/api/reviews/decision', { method: 'POST', operator: true, input: denialInput });
  assert.equal(denied.status, 201);
  assert.equal(denied.body.review.decision, 'DENY');
  assert.equal(denied.body.review.comment, denialInput.comment);
  const records = (await app.request('/api/audit?limit=100', { operator: true })).body.records;
  const { audit, ...original } = escalation;
  assert.deepEqual(records.find((record) => record.id === audit.id).result, original);
  assert.equal(records.find((record) => record.id === decision.body.review.id).assessmentId, audit.id);
  const before = (await app.request('/api/reviews', { operator: true })).body.reviews;
  await app.close();
  app = await env.start();
  assert.deepEqual((await app.request('/api/reviews', { operator: true })).body.reviews, before);
});

test('HTTP human approval cannot use an escalation from a changed policy or revoked authority', async (t) => {
  const env = await setup(t);
  const app = await env.start();
  const credential = (await app.request('/api/gate/credentials', { method: 'POST', operator: true,
    input: { agentId: 'demo-agent' } })).body.credential;
  const input = { credential, intent: { agentId: 'demo-agent', action: 'read_file', target: 'file:demo' } };
  const escalation = (await app.request('/api/gate/evaluate', { method: 'POST', input })).body;
  await app.request('/api/gate-rules', { operator: true, method: 'PUT', input: { version: 1, text: 'ESCALATE read_file file:demo' } });
  const review = { assessmentId: escalation.audit.id, decision: 'APPROVE', comment: 'Attempt to reuse stale policy.' };
  const stale = await app.request('/api/reviews/decision', { method: 'POST', operator: true, input: review });
  assert.equal(stale.status, 409);
  assert.equal(stale.body.code, 'REVIEW_STALE');
  const fresh = (await app.request('/api/gate/evaluate', { method: 'POST', input })).body;
  const changed = (await app.request('/api/registry', { operator: true })).body.registry;
  changed.agents[0].whitelisted = false;
  await app.request('/api/registry', { operator: true, method: 'PUT', input: { registry: changed } });
  const revoked = await app.request('/api/reviews/decision', { method: 'POST', operator: true,
    input: { ...review, assessmentId: fresh.audit.id } });
  assert.equal(revoked.status, 409);
  assert.equal(revoked.body.code, 'REVIEW_STALE');
  assert.equal((await app.request('/api/reviews/decision', { method: 'POST', operator: true,
    input: { ...review, assessmentId: fresh.audit.id, decision: 'DENY', comment: 'Authority was revoked.' } })).status, 201);
});

test('agents can observe only their own recorded requests and human context, with stale approval identified', async (t) => {
  const env = await setup(t);
  const app = await env.start();
  const change = (await app.request('/api/registry', { operator: true })).body.registry;
  const other = structuredClone(change.agents[0]);
  other.id = 'other-agent';
  other.grants.forEach((grant) => { grant.id += '.other'; });
  change.agents.push(other);
  assert.equal((await app.request('/api/registry', { operator: true, method: 'PUT', input: { registry: change } })).status, 200);
  const issue = async (agentId) => (await app.request('/api/gate/credentials', {
    operator: true, method: 'POST', input: { agentId },
  })).body.credential;
  const credential = await issue('demo-agent');
  const otherCredential = await issue('other-agent');
  const intent = { agentId: 'demo-agent', action: 'read_file', target: 'file:demo' };
  const submitted = (await app.request('/api/gate/evaluate', { method: 'POST', input: { credential, intent } })).body;
  const endpoint = `/api/agent/assessments/${submitted.audit.id}`;
  const headers = { authorization: `Bearer ${credential}` };
  assert.equal((await app.request(endpoint)).status, 401);
  assert.equal((await app.request(endpoint, { operator: true })).status, 401);
  assert.equal((await app.request(endpoint, { headers: { authorization: `Bearer ${otherCredential}` } })).status, 404);
  assert.equal((await app.request('/api/agent/assessments/missing', { headers })).status, 404);
  const pending = await app.request(endpoint, { headers });
  assert.equal(pending.status, 200);
  assert.equal(pending.body.current, true);
  assert.equal(pending.body.review, null);
  assert.deepEqual(pending.body.assessment.request, intent);
  const { audit, ...gateResult } = submitted;
  assert.deepEqual(pending.body.assessment.result, gateResult);
  assert.equal(pending.body.assessment.id, audit.id);
  assert.equal(pending.body.assessment.recordedAt, audit.recordedAt);
  const input = { assessmentId: audit.id, decision: 'APPROVE', comment: 'Verified this request for the agent.' };
  const review = (await app.request('/api/reviews/decision', { operator: true, method: 'POST', input })).body.review;
  const approved = (await app.request(endpoint, { headers })).body;
  assert.deepEqual(approved.review, review);
  assert.equal(approved.current, true);
  await app.request('/api/gate-rules', { operator: true, method: 'PUT', input: { version: 1, text: 'ESCALATE read_file file:demo' } });
  const stale = (await app.request(endpoint, { headers })).body;
  assert.equal(stale.current, false);
  assert.deepEqual(stale.review, review);
  const replacement = await issue('demo-agent');
  assert.equal((await app.request(endpoint, { headers })).status, 401);
  assert.equal((await app.request(endpoint, { headers: { authorization: `Bearer ${replacement}` } })).status, 200);
});

test('failed journal writes cannot activate policy or registry changes or return an unaudited assessment', async (t) => {
  const directory = await mkdtemp(join(tmpdir(), 'apex-auditor-failure-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const store = await createAuditorStore(directory, registry);
  await rename(join(directory, 'audit.jsonl'), join(directory, 'retained.jsonl'));
  await mkdir(join(directory, 'audit.jsonl'));
  await assert.rejects(store.savePolicy(1, rules), { code: 'AUDIT_UNAVAILABLE' });
  assert.equal(store.getPolicy().version, 1);
  const changed = structuredClone(registry);
  changed.agents[0].whitelisted = false;
  await assert.rejects(store.saveRegistry(changed), { code: 'AUDIT_UNAVAILABLE' });
  assert.equal(store.getRegistry().agents[0].whitelisted, true);
  await assert.rejects(store.assess(() => ({ status: 'APPROVE' }), null), { code: 'AUDIT_UNAVAILABLE' });
});

test('registry revocation and assessments share the same serialization point', async (t) => {
  const directory = await mkdtemp(join(tmpdir(), 'apex-auditor-order-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const store = await createAuditorStore(directory, registry);
  let release;
  const held = new Promise((resolve) => { release = resolve; });
  const first = store.assess(async () => { await held; return { status: 'DENY' }; }, null);
  const changed = structuredClone(registry);
  changed.agents[0].whitelisted = false;
  const revoked = store.saveRegistry(changed);
  const after = store.assess((policy, snapshot) => ({ version: snapshot.version, whitelisted: snapshot.agents[0].whitelisted }), null);
  release();
  await first;
  await revoked;
  const observed = await after;
  assert.equal(observed.version, 2);
  assert.equal(observed.whitelisted, false);
});

test('truncated journal blocks restart instead of silently discarding evidence', async (t) => {
  const directory = await mkdtemp(join(tmpdir(), 'apex-auditor-corrupt-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  await writeFile(join(directory, 'audit.jsonl'), '{"type":');
  await assert.rejects(createAuditorStore(directory, registry), { code: 'AUDIT_CORRUPT' });
});
