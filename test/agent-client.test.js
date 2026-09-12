import assert from 'node:assert/strict';
import { createHash, randomBytes, randomUUID } from 'node:crypto';
import { execFile } from 'node:child_process';
import { mkdtemp, writeFile, chmod, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { submitIntent, observeAssessment } from '../src/agent-client.js';
import { createGate } from '../src/gate.js';
import { createTokenAuthenticator } from '../src/switchboard.js';

const declared = { agentId: 'agent.client', action: 'read_file', target: 'file:demo' };
const json = (value, status = 200) => new Response(JSON.stringify(value), { status, headers: { 'content-type': 'application/json' } });
function fixture(decision = 'APPROVE') {
  const credential = randomBytes(32).toString('hex');
  const registry = { id: 'client.test', version: 1, agents: [{ id: declared.agentId, whitelisted: true, grants: [
    { id: 'grant.read', action: declared.action, target: declared.target, issuedBy: 'operator' },
  ] }] };
  const gate = createGate({ registry, authenticate: createTokenAuthenticator([{ principalId: declared.agentId, token: credential }]),
    policy: { version: 1, text: `${decision} read_file file:demo` } });
  const result = gate.evaluate({ credential, intent: declared });
  const audit = { id: randomUUID(), recordedAt: new Date().toISOString() };
  return { credential, receipt: structuredClone({ ...result, audit }), record: {
    assessment: { ...audit, request: { ...declared }, result: structuredClone(result) }, review: null, current: true,
  } };
}
function reviewed(record, decision = 'APPROVE') {
  const { assessment } = record;
  record.review = { id: randomUUID(), recordedAt: new Date().toISOString(), type: 'human_review',
    assessmentId: assessment.id,
    assessmentHash: createHash('sha256').update(JSON.stringify({ request: assessment.request, result: assessment.result })).digest('hex'),
    decisionModel: 'approve_escalate_v1', decision, comment: 'Checked this exact request with its owner.',
    reviewer: 'operator', actor: 'operator', execution: 'NOT_EXECUTED' };
  return record;
}

test('submit uses the real gate contract, keeps credentials in intake, forbids redirects, and returns immutable receipts', async (t) => {
  const { credential, receipt } = fixture();
  t.mock.method(globalThis, 'fetch', async (url, options) => {
    assert.equal(url, 'http://127.0.0.1:43129/api/gate/evaluate');
    assert.equal(options.method, 'POST');
    assert.equal(options.redirect, 'error');
    assert.equal(options.headers.authorization, undefined);
    assert.deepEqual(JSON.parse(options.body), { credential, intent: declared });
    return json(receipt);
  });
  const result = await submitIntent({ credential, intent: declared });
  assert.deepEqual(result, receipt);
  assert.equal(JSON.stringify(result).includes(credential), false);
  assert.throws(() => { result.policy.version = 2; }, TypeError);
});

test('wrong identity, action, target, receipt, model, or invented permission all fail closed', async (t) => {
  const { credential, receipt } = fixture();
  const mutate = [
    (r) => { r.switchboard.principalId = 'agent.other'; },
    (r) => { r.switchboard.intent.target = 'file:other'; },
    (r) => { r.switchboard.intent.action = 'delete_file'; },
    (r) => { r.switchboard.grant.target = 'file:other'; },
    (r) => { r.audit.id = ''; },
    (r) => { r.audit.recordedAt = 'not-a-time'; },
    (r) => { delete r.audit; },
    (r) => { r.decisionModel = 'legacy'; },
    (r) => { r.version = '0.0.1'; },
    (r) => { r.status = 'ALLOW'; },
    (r) => { r.execution = 'AUTHORIZED'; },
    (r) => { r.authorization = 'invented'; },
    (r) => { r.matchedRules[0].decision = 'ESCALATE'; },
    (r) => { r.reason = credential; },
  ];
  let response;
  t.mock.method(globalThis, 'fetch', async () => json(response));
  for (const change of mutate) {
    response = structuredClone(receipt); change(response);
    await assert.rejects(submitIntent({ credential, intent: declared }), { code: 'INVALID_RESPONSE' });
  }
});

test('invalid caller scope, unsafe origins, and credentials never initiate a request', async (t) => {
  const { credential } = fixture();
  t.mock.method(globalThis, 'fetch', () => assert.fail('Invalid configuration reached the network.'));
  for (const intent of [null, {}, { ...declared, approved: true }, { ...declared, target: '*' }]) {
    await assert.rejects(submitIntent({ credential, intent }), { code: 'INVALID_INTENT' });
  }
  for (const url of ['https://example.com', 'http://example.com', 'http://127.0.0.1:43129/path',
    'http://user:password@localhost', 'http://localhost/?key=private', 'not-a-url']) {
    await assert.rejects(submitIntent({ url, credential, intent: declared }), { code: 'INVALID_URL' });
  }
  await assert.rejects(submitIntent({ credential: 'bad', intent: declared }), { code: 'INVALID_CREDENTIAL' });
});

test('non-success responses, malformed JSON, redirects, and oversized streams are blocking errors without server secrets', async (t) => {
  const { credential } = fixture();
  let response;
  let signal;
  t.mock.method(globalThis, 'fetch', async (_url, options) => { signal = options.signal; return response; });
  for (const status of [401, 403, 404, 409, 500, 503]) {
    response = json({ error: credential }, status);
    await assert.rejects(submitIntent({ credential, intent: declared }), (error) => error.code === 'HTTP_ERROR' && !String(error).includes(credential));
    assert.equal(signal.aborted, true, 'an unconsumed error response cannot keep the connection alive');
  }
  response = new Response('{invalid', { headers: { 'content-type': 'application/json' } });
  await assert.rejects(submitIntent({ credential, intent: declared }), { code: 'INVALID_RESPONSE' });
  response = json({}); Object.defineProperty(response, 'redirected', { value: true });
  await assert.rejects(submitIntent({ credential, intent: declared }), { code: 'INVALID_RESPONSE' });
  response = new Response(' '.repeat(32769), { headers: { 'content-type': 'application/json' } });
  await assert.rejects(submitIntent({ credential, intent: declared }), { code: 'RESPONSE_TOO_LARGE' });
});

test('requests time out even when the transport stalls and caller cancellation is honored', async (t) => {
  const { credential } = fixture();
  t.mock.timers.enable({ apis: ['setTimeout'] });
  t.mock.method(globalThis, 'fetch', () => new Promise(() => {}));
  const timed = assert.rejects(submitIntent({ credential, intent: declared }), { code: 'REQUEST_TIMEOUT' });
  t.mock.timers.tick(10000);
  await timed;
  const controller = new AbortController();
  const aborted = assert.rejects(submitIntent({ credential, intent: declared, signal: controller.signal }), { code: 'REQUEST_ABORTED' });
  controller.abort();
  await aborted;
});

test('status uses agent Bearer auth and verifies the exact linked human outcome hash', async (t) => {
  const { credential, record } = fixture('ESCALATE');
  reviewed(record);
  t.mock.method(globalThis, 'fetch', async (url, options) => {
    assert.equal(url, `http://127.0.0.1:43129/api/agent/assessments/${record.assessment.id}`);
    assert.equal(options.method, 'GET');
    assert.equal(options.headers.authorization, `Bearer ${credential}`);
    assert.equal(options.body, undefined);
    return json(record);
  });
  const result = await observeAssessment({ credential, assessmentId: record.assessment.id });
  assert.deepEqual(result, record);
  assert.throws(() => { result.review.decision = 'DENY'; }, TypeError);
});

test('status rejects substituted records, broken human binding, actor/comment changes, and missing current state', async (t) => {
  const { credential, record } = fixture('ESCALATE'); reviewed(record);
  const changes = [
    (r) => { r.assessment.id = randomUUID(); },
    (r) => { r.assessment.request.target = 'file:other'; },
    (r) => { r.review.assessmentId = randomUUID(); },
    (r) => { r.review.assessmentHash = 'a'.repeat(64); },
    (r) => { r.review.reviewer = 'agent'; },
    (r) => { r.review.comment = ''; },
    (r) => { r.review.comment = 'x'.repeat(2001); },
    (r) => { r.review.decision = 'ALLOW'; },
    (r) => { delete r.current; },
  ];
  let response;
  t.mock.method(globalThis, 'fetch', async () => json(response));
  for (const change of changes) {
    response = structuredClone(record); change(response);
    await assert.rejects(observeAssessment({ credential, assessmentId: record.assessment.id }), { code: 'INVALID_RESPONSE' });
  }
});

function runCli(args, options, input = '') {
  return new Promise((resolve) => {
    const child = execFile(process.execPath, args, options, (error, stdout, stderr) => {
      resolve({ code: error?.code ?? 0, stdout, stderr });
    });
    child.stdin.end(input);
  });
}

test('CLI status never approves stale records, keeps human denial, and rejects public credential files', async (t) => {
  const directory = await mkdtemp(join(tmpdir(), 'apex-agent-client-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const { credential, record } = fixture('ESCALATE'); reviewed(record);
  const credentialFile = join(directory, 'agent.key');
  const preload = join(directory, 'transport.mjs');
  await writeFile(credentialFile, `${credential}\n`, { mode: 0o600 });
  const cli = fileURLToPath(new URL('../bin/apex-agent.mjs', import.meta.url));
  const options = { env: { ...process.env, APEX_AGENT_ID: declared.agentId, APEX_CREDENTIAL_FILE: credentialFile,
    APEX_URL: 'http://127.0.0.1:43129' }, timeout: 5000 };
  for (const [current, choice, expectedCode, expectedStatus] of [
    [true, 'APPROVE', 0, 'APPROVE'], [false, 'APPROVE', 3, 'REASSESSMENT_REQUIRED'],
    [false, 'DENY', 3, 'DENY'],
  ]) {
    record.current = current; record.review.decision = choice;
    await writeFile(preload, `globalThis.fetch = async () => new Response(${JSON.stringify(JSON.stringify(record))}, {headers:{'content-type':'application/json'}});\n`);
    const result = await runCli(['--import', preload, cli, 'status', record.assessment.id], options);
    assert.equal(result.code, expectedCode, result.stderr);
    assert.equal(JSON.parse(result.stdout).client.status, expectedStatus);
    assert.equal((result.stdout + result.stderr).includes(credential), false);
  }
  await chmod(credentialFile, 0o644);
  const rejected = await runCli([cli, 'status', record.assessment.id], options);
  assert.equal(rejected.code, 1);
  assert.equal(JSON.parse(rejected.stderr).code, 'UNSAFE_CREDENTIAL_FILE');
  assert.equal((rejected.stdout + rejected.stderr).includes(credential), false);
});

test('CLI submits declared intent from stdin and stops for escalation instead of executing', async (t) => {
  const directory = await mkdtemp(join(tmpdir(), 'apex-agent-submit-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const { credential, receipt } = fixture('ESCALATE');
  const credentialFile = join(directory, 'agent.key');
  const preload = join(directory, 'transport.mjs');
  await writeFile(credentialFile, credential, { mode: 0o600 });
  await writeFile(preload, `globalThis.fetch = async (_url, options) => { const input=JSON.parse(options.body); if(input.intent.agentId !== 'agent.client') throw new Error('Unexpected input'); return new Response(${JSON.stringify(JSON.stringify(receipt))}, {headers:{'content-type':'application/json'}}); };\n`);
  const cli = fileURLToPath(new URL('../bin/apex-agent.mjs', import.meta.url));
  const result = await runCli(['--import', preload, cli, 'submit'], { env: { ...process.env,
    APEX_AGENT_ID: declared.agentId, APEX_CREDENTIAL_FILE: credentialFile, APEX_URL: 'http://localhost:43129' }, timeout: 5000 }, JSON.stringify(declared));
  assert.equal(result.code, 2, result.stderr);
  assert.equal(JSON.parse(result.stdout).status, 'ESCALATE');
  assert.equal(JSON.parse(result.stdout).execution, 'NOT_EXECUTED');
  assert.equal((result.stdout + result.stderr).includes(credential), false);
});
