import assert from 'node:assert/strict';
import { randomBytes } from 'node:crypto';
import test from 'node:test';
import { createSwitchboard, createTokenAuthenticator } from '../src/switchboard.js';

const token = () => randomBytes(32).toString('hex');
const intent = () => ({ agentId: 'reader', action: 'report.read', target: 'report:one' });
const grant = (id, action, target) => ({ id, action, target, issuedBy: 'operator:owner' });
function fixture() {
  const registry = {
    id: 'test.registry', version: 1,
    agents: [
      { id: 'reader', whitelisted: true, grants: [
        grant('read-report', 'report.read', 'report:one'),
        grant('export-report', 'report.export', 'destination:one'),
      ] },
      { id: 'paused', whitelisted: false, grants: [grant('paused-read', 'report.read', 'report:one')] },
      { id: 'empty', whitelisted: true, grants: [] },
    ],
  };
  const credentials = ['reader', 'paused', 'empty', 'outsider'].map((principalId) => ({ principalId, token: token() }));
  const authenticate = createTokenAuthenticator(credentials);
  const switchboard = createSwitchboard(registry, { authenticate });
  const screen = (value = intent(), principal = 'reader') => switchboard.screen({
    credential: credentials.find((entry) => entry.principalId === principal).token, intent: value,
  });
  return { registry, credentials, authenticate, switchboard, screen };
}

function blocked(result, code, status = 'DENY') {
  assert.equal(result.status, status);
  assert.equal(result.code, code);
  assert.equal(result.next, null);
  assert.equal(result.grant, null);
}

test('exact whitelisted scope passes with its authority source, without execution permission', () => {
  const { screen } = fixture();
  const result = screen();
  assert.equal(result.stage, 'SWITCHBOARD');
  assert.equal(result.status, 'PASS');
  assert.equal(result.code, 'SCOPE_GRANTED');
  assert.equal(result.next, 'CONSTRAINT_SCREENING');
  assert.equal(result.principalId, 'reader');
  assert.deepEqual(result.registry, { id: 'test.registry', version: 1 });
  assert.deepEqual(result.intent, intent());
  assert.deepEqual(result.grant, grant('read-report', 'report.read', 'report:one'));
  assert.equal(Object.hasOwn(result, 'authorization'), false);
  assert.equal(Object.hasOwn(result, 'executed'), false);
  assert.deepEqual(screen(), result, 'the same inputs have the same decision and explanation');
});

test('verified identity does not itself grant whitelist membership or scope', () => {
  const { screen } = fixture();
  blocked(screen({ ...intent(), agentId: 'outsider' }, 'outsider'), 'UNKNOWN_AGENT');
  blocked(screen({ ...intent(), agentId: 'paused' }, 'paused'), 'NOT_WHITELISTED');
  blocked(screen({ ...intent(), agentId: 'empty' }, 'empty'), 'SCOPE_NOT_GRANTED');
});

test('missing, malformed, or unknown credentials never authenticate', () => {
  const { switchboard, credentials } = fixture();
  for (const credential of [null, undefined, '', 123, {}, [], token(), credentials[0].token + '\n']) {
    const result = switchboard.screen({ credential, intent: intent() });
    blocked(result, 'UNAUTHENTICATED');
    assert.equal(result.principalId, null);
    assert.equal(result.intent, null);
  }
});

test('request-body identity and scope claims cannot impersonate or add authority', () => {
  const { switchboard, credentials, screen } = fixture();
  blocked(screen({ ...intent(), agentId: 'paused' }), 'IDENTITY_MISMATCH');
  blocked(screen(intent(), 'outsider'), 'UNKNOWN_AGENT');
  for (const field of ['whitelisted', 'grants', 'principalId', 'approved', 'issuedBy']) {
    blocked(screen({ ...intent(), [field]: true }), 'INVALID_INTENT', 'ERROR');
    blocked(switchboard.screen({ credential: credentials[0].token, intent: intent(), [field]: true }),
      'INVALID_REQUEST', 'ERROR');
  }
});

test('permissions are exact action-target pairs, never cross-products or prefixes', () => {
  const { screen } = fixture();
  assert.equal(screen({ ...intent(), action: 'report.export', target: 'destination:one' }).status, 'PASS');
  for (const [action, target] of [
    ['report.delete', 'report:one'],
    ['report.read', 'report:two'],
    ['report.read', 'destination:one'],
    ['report.export', 'report:one'],
    ['report.read', 'report:one/child'],
    ['Report.read', 'report:one'],
    ['report.read', 'REPORT:one'],
  ]) blocked(screen({ ...intent(), action, target }), 'SCOPE_NOT_GRANTED');
});

test('malformed or incomplete intents block instead of receiving defaults', () => {
  const { switchboard, credentials } = fixture();
  for (const value of [null, undefined, [], 'report.read', {},
    { agentId: 'reader', target: 'report:one' },
    { ...intent(), target: '' }, { ...intent(), action: '*' },
    { ...intent(), agentId: 'reader\n' }, { ...intent(), action: 'report.read ' },
    { ...intent(), target: 'report:one\n' }, { ...intent(), target: 'report:*' },
    { ...intent(), target: 'x'.repeat(257) },
  ]) blocked(switchboard.screen({ credential: credentials[0].token, intent: value }), 'INVALID_INTENT', 'ERROR');
});

test('malformed requests and accessor properties are rejected without executing them', () => {
  const { switchboard, screen } = fixture();
  for (const value of [null, undefined, [], '', {}, { intent: intent() }]) {
    blocked(switchboard.screen(value), 'INVALID_REQUEST', 'ERROR');
  }
  const accessorIntent = { ...intent() };
  Object.defineProperty(accessorIntent, 'action', { enumerable: true, get() { assert.fail('getter ran'); } });
  blocked(screen(accessorIntent), 'INVALID_INTENT', 'ERROR');
});

test('authentication precedes registry/intent screening and failures reveal no verifier detail', () => {
  const { registry } = fixture();
  const secret = token();
  const switchboard = createSwitchboard(registry, { authenticate(credential) {
    assert.equal(credential, secret);
    throw new Error(`private verifier failure: ${secret}`);
  } });
  const result = switchboard.screen({ credential: secret, intent: { agentId: 'reader', whitelisted: true } });
  blocked(result, 'AUTHENTICATION_UNAVAILABLE', 'ERROR');
  assert.equal(JSON.stringify(result).includes(secret), false);
  assert.equal(JSON.stringify(result).includes('private verifier'), false);
  const invalidVerifier = createSwitchboard(registry, { authenticate: () => ({ principalId: 'reader' }) });
  blocked(invalidVerifier.screen({ credential: secret, intent: intent() }), 'UNAUTHENTICATED');
});

test('async and rejected-Promise verifiers block without an unhandled rejection', async () => {
  const { registry, credentials } = fixture();
  const verifiers = [
    async () => 'reader',
    async () => { throw new Error('async verifier failed'); },
    () => Promise.reject(new Error('promise verifier failed')),
    () => ({ then(_resolve, reject) { reject(new Error('thenable verifier failed')); } }),
  ];
  for (const authenticate of verifiers) {
    const switchboard = createSwitchboard(registry, { authenticate });
    const result = switchboard.screen({ credential: credentials[0].token, intent: intent() });
    blocked(result, 'AUTHENTICATION_UNAVAILABLE', 'ERROR');
    assert.equal(result.principalId, null);
  }
  // Yield beyond Promise rejection handling; node:test fails on an unhandled
  // rejection even if the synchronous screening assertions already passed.
  await new Promise((resolve) => setImmediate(resolve));
});

test('configuration changes cannot mutate an existing registry, credential map, or returned grant', () => {
  const { registry, credentials, switchboard, screen, authenticate } = fixture();
  const originalToken = credentials[0].token;
  const originalResult = screen();
  registry.id = 'other.registry';
  registry.version = 2;
  registry.agents[0].grants[0].target = 'report:private';
  registry.agents[1].whitelisted = true;
  registry.agents[2].grants.push(grant('new-grant', 'report.read', 'report:one'));
  credentials[0].token = token();
  credentials[0].principalId = 'admin';
  assert.equal(authenticate(originalToken), 'reader');
  assert.equal(authenticate(credentials[0].token), null);
  const result = switchboard.screen({ credential: originalToken, intent: intent() });
  assert.deepEqual(result, originalResult);
  blocked(switchboard.screen({ credential: originalToken, intent: { ...intent(), target: 'report:private' } }), 'SCOPE_NOT_GRANTED');
  blocked(screen({ ...intent(), agentId: 'paused' }, 'paused'), 'NOT_WHITELISTED');
  blocked(screen({ ...intent(), agentId: 'empty' }, 'empty'), 'SCOPE_NOT_GRANTED');
  assert.throws(() => { result.grant.target = 'report:private'; }, TypeError);
  assert.throws(() => { result.registry.version = 99; }, TypeError);
  assert.throws(() => { result.intent.target = 'report:private'; }, TypeError);
  assert.throws(() => { result.status = 'ALLOW'; }, TypeError);
});

test('credentials are never included in screening results', () => {
  const { screen, switchboard, credentials } = fixture();
  const input = intent();
  const results = [screen(input), screen({ ...input, target: 'report:private' }),
    switchboard.screen({ credential: credentials[0].token, intent: null })];
  for (const result of results) {
    for (const credential of credentials) assert.equal(JSON.stringify(result).includes(credential.token), false);
  }
  assert.deepEqual(input, intent(), 'screening does not rewrite the declaration');
});

test('malformed and ambiguous registry configuration prevents startup', async (t) => {
  const mutations = [
    ['missing whitelist flag', (c) => { delete c.agents[0].whitelisted; }],
    ['string whitelist flag', (c) => { c.agents[0].whitelisted = 'true'; }],
    ['missing grants', (c) => { delete c.agents[0].grants; }],
    ['unknown config field', (c) => { c.default = 'ALLOW'; }],
    ['unknown grant field', (c) => { c.agents[0].grants[0].allowAll = true; }],
    ['duplicate agent', (c) => { c.agents.push(structuredClone(c.agents[0])); }],
    ['duplicate grant ID across agents', (c) => { c.agents[1].grants[0].id = 'read-report'; }],
    ['duplicate pair', (c) => { c.agents[0].grants.push({ ...c.agents[0].grants[0], id: 'another-id' }); }],
    ['missing grant issuer', (c) => { delete c.agents[0].grants[0].issuedBy; }],
    ['wildcard target', (c) => { c.agents[0].grants[0].target = '*'; }],
    ['whitespace identity', (c) => { c.agents[0].id += '\n'; }],
    ['invalid version', (c) => { c.version = 1.5; }],
    ['null grant', (c) => { c.agents[0].grants.push(null); }],
  ];
  for (const [name, mutate] of mutations) await t.test(name, () => {
    const { registry, authenticate } = fixture();
    mutate(registry);
    assert.throws(() => createSwitchboard(registry, { authenticate }), TypeError);
  });
});

test('authenticator and credentials must be explicit and unambiguous', () => {
  const { registry } = fixture();
  for (const options of [undefined, {}, { authenticate: null }]) {
    assert.throws(() => createSwitchboard(registry, options), TypeError);
  }
  const first = { principalId: 'reader', token: token() };
  for (const entries of [undefined, {}, [null], new Array(1),
    [{ ...first, token: '' }], [{ ...first, token: first.token + '\n' }],
    [{ ...first, token: 'A' + first.token.slice(1) }],
    [first, { principalId: 'other', token: first.token }],
    [first, { principalId: 'reader', token: token() }],
    [{ ...first, role: 'admin' }],
  ]) assert.throws(() => createTokenAuthenticator(entries), TypeError);
});

test('empty registries and credential sets permit nothing', () => {
  const { registry, credentials, authenticate } = fixture();
  const emptyRegistry = createSwitchboard({ ...registry, agents: [] }, { authenticate });
  blocked(emptyRegistry.screen({ credential: credentials[0].token, intent: intent() }), 'UNKNOWN_AGENT');
  const noCredentials = createSwitchboard(registry, { authenticate: createTokenAuthenticator([]) });
  blocked(noCredentials.screen({ credential: credentials[0].token, intent: intent() }), 'UNAUTHENTICATED');
});

test('replacing the active instance applies explicit whitelist removal', () => {
  const { registry, credentials, authenticate, screen } = fixture();
  assert.equal(screen().status, 'PASS');
  registry.version = 2;
  registry.agents[0].whitelisted = false;
  const replacement = createSwitchboard(registry, { authenticate });
  const result = replacement.screen({ credential: credentials[0].token, intent: intent() });
  blocked(result, 'NOT_WHITELISTED');
  assert.equal(result.registry.version, 2);
  assert.equal(screen().status, 'PASS', 'the host must replace old instances explicitly');
});
