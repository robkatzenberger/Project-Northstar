import assert from 'node:assert/strict';
import { createHash, randomBytes } from 'node:crypto';
import test from 'node:test';
import { createGate, parseRules, parseLegacyRules } from '../src/gate.js';
import { createTokenAuthenticator } from '../src/switchboard.js';

const actionIds = ['send_email', 'delete_file', 'execute_file', 'write_file', 'read_file'];
const defaultRules = 'APPROVE read_file file:demo\nESCALATE delete_file file:demo\nESCALATE send_email recipient:demo';
const grant = (id, action, target) => ({ id, action, target, issuedBy: 'operator:owner' });

function fixture(text = defaultRules) {
  const registry = {
    id: 'gate.test', version: 1,
    agents: [
      { id: 'agent.demo', whitelisted: true, grants: [
        ...actionIds.map((action) => grant(`grant.${action}`, action,
          action === 'send_email' ? 'recipient:demo' : 'file:demo')),
        grant('grant.unknown', 'unknown_action', 'file:demo'),
        grant('grant.other', 'read_file', 'file:other'),
      ] },
      { id: 'agent.paused', whitelisted: false, grants: [grant('grant.paused', 'read_file', 'file:demo')] },
      { id: 'agent.empty', whitelisted: true, grants: [] },
    ],
  };
  const credentials = ['agent.demo', 'agent.paused', 'agent.empty', 'agent.unknown'].map((principalId) => ({
    principalId, token: randomBytes(32).toString('hex'),
  }));
  const authenticate = createTokenAuthenticator(credentials);
  const policy = { version: 1, text };
  const gate = createGate({ registry, authenticate, policy });
  const request = (action = 'read_file', target = 'file:demo', agentId = 'agent.demo') => ({
    credential: credentials.find((entry) => entry.principalId === agentId).token,
    intent: { agentId, action, target },
  });
  return { gate, registry, authenticate, policy, credentials, request };
}

function expectOutcome(result, status, code, stage = 'GATE') {
  assert.equal(result.status, status);
  assert.equal(result.code, code);
  assert.equal(result.stage, stage);
  assert.equal(result.version, '0.0.2');
  assert.equal(result.decisionModel, 'approve_escalate_v1');
  assert.equal(result.execution, 'NOT_EXECUTED');
  assert.equal(Object.hasOwn(result, 'authorization'), false);
  assert.equal(Object.hasOwn(result, 'approval'), false);
}

test('human rule text has line references, exact vocabulary, comments, and immutable output', () => {
  const rules = parseRules('# Test policy\r\n\r\n \tAPPROVE\tread_file  file:demo \t\r\nESCALATE delete_file file:demo\n');
  assert.deepEqual(rules, [
    { line: 3, decision: 'APPROVE', action: 'read_file', target: 'file:demo' },
    { line: 4, decision: 'ESCALATE', action: 'delete_file', target: 'file:demo' },
  ]);
  assert.throws(() => rules.push({}), TypeError);
  assert.throws(() => { rules[0].decision = 'ESCALATE'; }, TypeError);
});

test('all five catalog actions can be screened by explicit rules', () => {
  for (const action of actionIds) {
    const target = action === 'send_email' ? 'recipient:demo' : 'file:demo';
    const { gate, request } = fixture(`APPROVE ${action} ${target}`);
    expectOutcome(gate.evaluate(request(action, target)), 'APPROVE', 'RULE_APPROVE');
  }
});

test('exact matching rules produce APPROVE or ESCALATE without a machine denial', () => {
  const { gate, policy, request } = fixture();
  const read = gate.evaluate(request());
  expectOutcome(read, 'APPROVE', 'RULE_APPROVE');
  assert.equal(read.switchboard.status, 'PASS');
  assert.match(read.reason, /not execution permission/);
  assert.deepEqual(read.policy, {
    version: 1, hash: createHash('sha256').update(policy.text).digest('hex'),
  });
  assert.deepEqual(read.matchedRules, [{ line: 1, decision: 'APPROVE', action: 'read_file', target: 'file:demo' }]);
  expectOutcome(gate.evaluate(request('delete_file')), 'ESCALATE', 'HUMAN_VERIFICATION_REQUIRED');
  const review = gate.evaluate(request('send_email', 'recipient:demo'));
  expectOutcome(review, 'ESCALATE', 'HUMAN_VERIFICATION_REQUIRED');
  assert.match(review.reason, /human decision/);
  assert.deepEqual(gate.evaluate(request()), read, 'same input yields the same complete result');
});

test('real credentials, whitelist, identity, and scope are screened before policy', () => {
  const { gate, request } = fixture('ESCALATE read_file file:demo');
  const cases = [
    [{ ...request(), credential: null }, 'BLOCKED', 'UNAUTHENTICATED'],
    [request('read_file', 'file:demo', 'agent.paused'), 'BLOCKED', 'NOT_WHITELISTED'],
    [request('read_file', 'file:demo', 'agent.unknown'), 'BLOCKED', 'UNKNOWN_AGENT'],
    [request('read_file', 'file:demo', 'agent.empty'), 'BLOCKED', 'SCOPE_NOT_GRANTED'],
    [{ ...request(), intent: { ...request().intent, agentId: 'agent.paused' } }, 'BLOCKED', 'IDENTITY_MISMATCH'],
    [request('read_file', 'file:demo/child'), 'BLOCKED', 'SCOPE_NOT_GRANTED'],
    [{ ...request(), intent: null }, 'ERROR', 'INVALID_INTENT'],
    [{ ...request(), approved: true }, 'ERROR', 'INVALID_REQUEST'],
  ];
  for (const [input, status, code] of cases) {
    const result = gate.evaluate(input);
    expectOutcome(result, status, code, 'SWITCHBOARD');
    assert.deepEqual(result.matchedRules, []);
    assert.equal(result.reason, result.switchboard.reason);
    assert.equal(result.switchboard.status, status === 'BLOCKED' ? 'DENY' : 'ERROR');
    assert.equal(result.switchboard.next, null, 'an authority failure cannot progress to human policy review');
  }
  const paused = request('unknown_action', 'file:demo', 'agent.paused');
  paused.intent = null;
  expectOutcome(gate.evaluate(paused), 'BLOCKED', 'NOT_WHITELISTED', 'SWITCHBOARD');
});

test('empty and unmatched policy escalates exact scoped requests', () => {
  for (const text of ['', '# Nothing approved\n \t', 'APPROVE read_file file:other',
    'APPROVE read_file File:demo', 'APPROVE read_file file:demo/child', 'APPROVE write_file file:demo']) {
    const { gate, request } = fixture(text);
    expectOutcome(gate.evaluate(request()), 'ESCALATE', 'NO_MATCHING_RULE');
  }
});

test('ESCALATE dominates APPROVE independent of rule order', () => {
  for (const order of [['APPROVE', 'ESCALATE'], ['ESCALATE', 'APPROVE']]) {
    const { gate, request } = fixture(order.map((decision) => `${decision} read_file file:demo`).join('\n'));
    const result = gate.evaluate(request());
    expectOutcome(result, 'ESCALATE', 'HUMAN_VERIFICATION_REQUIRED');
    assert.equal(result.matchedRules.length, order.length, 'retain all matching rules as evidence');
  }
});

test('unknown action blocks only after the real Switchboard grants its exact scope', () => {
  const { gate, request } = fixture();
  const result = gate.evaluate(request('unknown_action'));
  expectOutcome(result, 'ERROR', 'UNKNOWN_ACTION');
  assert.equal(result.switchboard.status, 'PASS');
  assert.deepEqual(result.matchedRules, []);
  expectOutcome(gate.evaluate(request('unknown_action', 'file:other')), 'BLOCKED', 'SCOPE_NOT_GRANTED', 'SWITCHBOARD');
});

test('one bad line prevents the whole policy from being activated with a precise line error', () => {
  for (const line of ['APPROVE read_file', 'APPROVE read_file file:demo extra',
    'APPROVE read_file file:demo # inline comment', 'approve read_file file:demo',
    'DENY read_file file:demo', 'APPROVE read_only file:demo', 'APPROVE write file:demo',
    'APPROVE READ_FILE file:demo', 'APPROVE unknown_action file:demo', 'APPROVE read_file *',
    'APPROVE read_file file:*', 'APPROVE read_file /file/demo', 'APPROVE read_file file:demo?',
    'APPROVE read_file file:demo\u0000', 'APPROVE\u00a0read_file file:demo',
    'APPROVE read_file file:demo\r', 'APPROVE read_file file:demo\u2028', 'APPROVE read_file file:demo\u2029',
    'APPROVE read_file file:demo\rESCALATE delete_file file:demo', 'APPROVE read_file ' + 'a'.repeat(257),
  ]) {
    const text = `# Policy\nAPPROVE write_file file:demo\n${line}`;
    assert.throws(() => fixture(text), (error) => error instanceof TypeError
      && error.code === 'INVALID_RULES' && error.line === 3, line);
  }
});

test('identical semantic duplicate rules reject even when whitespace differs', () => {
  assert.throws(() => parseRules('APPROVE read_file file:demo\n APPROVE\tread_file file:demo '),
    (error) => error.code === 'INVALID_RULES' && error.line === 2 && /Duplicate/.test(error.message));
});

test('legacy rules validate for historical replay but can never activate the current gate', () => {
  const legacy = '# Historical policy\r\nALLOW read_file file:demo\r\nDENY delete_file file:demo\r\nREQUIRE_APPROVAL send_email recipient:demo\r\n';
  const parsed = parseLegacyRules(legacy);
  assert.deepEqual(parsed, [
    { line: 2, decision: 'ALLOW', action: 'read_file', target: 'file:demo' },
    { line: 3, decision: 'DENY', action: 'delete_file', target: 'file:demo' },
    { line: 4, decision: 'REQUIRE_APPROVAL', action: 'send_email', target: 'recipient:demo' },
  ]);
  assert.throws(() => { parsed[0].decision = 'APPROVE'; }, TypeError);
  assert.throws(() => { parsed.push({}); }, TypeError);
  for (const decision of ['ALLOW', 'DENY', 'REQUIRE_APPROVAL']) {
    const text = `${decision} read_file file:demo`;
    assert.throws(() => parseRules(text), { code: 'INVALID_RULES', line: 1 });
    assert.throws(() => fixture(text), { code: 'INVALID_RULES', line: 1 });
  }
  assert.deepEqual(parseLegacyRules(''), []);
  for (const text of ['APPROVE read_file file:demo', 'ESCALATE read_file file:demo',
    'ALLOW read_file *', 'ALLOW read_only file:demo', 'ALLOW read_file file:demo extra',
    'ALLOW read_file file:demo\n ALLOW\tread_file file:demo ', '#\ud800', '#' + 'a'.repeat(32768)]) {
    assert.throws(() => parseLegacyRules(text), { code: 'INVALID_RULES' });
  }
  const legacyLimit = Array.from({ length: 200 }, (_, i) => `ALLOW read_file file:${i}`);
  assert.equal(parseLegacyRules(legacyLimit.join('\n')).length, 200);
  assert.throws(() => parseLegacyRules([...legacyLimit, 'DENY read_file file:overflow'].join('\n')),
    { code: 'INVALID_RULES', line: 201 });
});

test('policy size and rule count are bounded without accepting a valid prefix', () => {
  const rules = Array.from({ length: 200 }, (_, i) => `APPROVE read_file file:${i}`);
  assert.equal(parseRules(rules.join('\n')).length, 200);
  assert.throws(() => parseRules([...rules, 'ESCALATE read_file file:overflow'].join('\n')),
    (error) => error.code === 'INVALID_RULES' && error.line === 201);
  assert.deepEqual(parseRules('#' + 'a'.repeat(32767)), []);
  for (const text of ['#' + 'a'.repeat(32768), '#' + 'é'.repeat(16384), null, undefined, {}, [], 1, '#\ud800']) {
    assert.throws(() => parseRules(text), (error) => error instanceof TypeError && error.code === 'INVALID_RULES');
  }
});

test('invalid policy configuration and accessors cannot create a gate or invoke getters', () => {
  const { registry, authenticate } = fixture();
  for (const policy of [null, {}, { version: 0, text: '' }, { version: 1.5, text: '' },
    { version: Number.MAX_SAFE_INTEGER + 1, text: '' }, { version: 1, text: null },
    { version: 1, text: '', default: 'APPROVE' }]) {
    assert.throws(() => createGate({ registry, authenticate, policy }),
      (error) => error instanceof TypeError && error.code === 'INVALID_RULES');
  }
  const policy = { version: 1, get text() { assert.fail('getter executed'); } };
  assert.throws(() => createGate({ registry, authenticate, policy }), TypeError);
});

test('input and result mutation cannot change a gate; replacement activates new policy', () => {
  const { gate, registry, authenticate, policy, request } = fixture();
  const original = gate.evaluate(request());
  const input = request();
  const oldResult = gate.evaluate(input);
  input.intent.action = 'delete_file';
  policy.text = 'ESCALATE read_file file:demo';
  policy.version = 2;
  registry.agents[0].whitelisted = false;
  assert.deepEqual(gate.evaluate(request()), original);
  assert.deepEqual(oldResult, original);
  for (const mutate of [
    () => { original.status = 'ESCALATE'; },
    () => { original.policy.version = 9; },
    () => { original.matchedRules[0].decision = 'ESCALATE'; },
    () => { original.matchedRules.push({}); },
    () => { original.switchboard.intent.target = 'file:other'; },
  ]) assert.throws(mutate, TypeError);
  registry.agents[0].whitelisted = true;
  const replacement = createGate({ registry, authenticate, policy });
  const updated = replacement.evaluate(request());
  expectOutcome(updated, 'ESCALATE', 'HUMAN_VERIFICATION_REQUIRED');
  assert.equal(updated.policy.version, 2);
  assert.notEqual(updated.policy.hash, original.policy.hash);
});

test('credentials and verifier failures remain out of all gate result paths', () => {
  const { gate, credentials, registry, policy, request } = fixture();
  const values = [request(), request('delete_file'), request('send_email', 'recipient:demo'),
    request('write_file'), request('unknown_action'), { ...request(), intent: null },
    request('read_file', 'file:demo', 'agent.paused'), { ...request(), credential: '' }];
  const failing = createGate({ registry, policy, authenticate() { throw new Error(credentials[0].token); } });
  const failed = failing.evaluate(request());
  expectOutcome(failed, 'ERROR', 'AUTHENTICATION_UNAVAILABLE', 'SWITCHBOARD');
  for (const result of [...values.map((value) => gate.evaluate(value)), failed]) {
    for (const { token } of credentials) assert.equal(JSON.stringify(result).includes(token), false);
  }
});
