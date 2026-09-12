import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { addGrant, loadRegistry, saveRegistry } from '../src/registry-store.js';
import { createSwitchboard, createTokenAuthenticator } from '../src/switchboard.js';

const validRegistry = () => ({
  id: 'apex.test',
  version: 3,
  agents: [
    {
      id: 'agent-one',
      whitelisted: true,
      grants: [
        {
          id: 'grant-read',
          action: 'documents.read',
          target: 'report:demo',
          issuedBy: 'operator:test',
        },
      ],
    },
  ],
});

async function temporaryRegistry(contents = 'unchanged\n') {
  const directory = await mkdtemp(join(tmpdir(), 'apex-test-'));
  const path = join(directory, 'agents.json');
  await writeFile(path, contents);
  return path;
}

test('valid registry creates a switchboard and saved file round-trips', async () => {
  const authenticate = () => null;
  const switchboard = createSwitchboard(validRegistry(), { authenticate });
  assert.equal(typeof switchboard.screen, 'function');

  const path = await temporaryRegistry();
  const { registry: saved, hash } = await saveRegistry(path, validRegistry());
  assert.equal(saved.version, 4);
  assert.match(hash, /^[a-f0-9]{64}$/);
  assert.deepEqual(await loadRegistry(path), saved);
  assert.deepEqual(JSON.parse(await readFile(path, 'utf8')), saved);
});

test('invalid registries write nothing', async (t) => {
  const cases = [
    ['empty agent id', (registry) => { registry.agents[0].id = ''; }],
    ['duplicate agent id', (registry) => { registry.agents.push({ ...registry.agents[0], grants: [] }); }],
    ['duplicate grant id', (registry) => {
      registry.agents.push({
        id: 'agent-two',
        whitelisted: false,
        grants: [{ ...registry.agents[0].grants[0] }],
      });
    }],
    ['malformed action', (registry) => { registry.agents[0].grants[0].action = 'Read Files'; }],
    ['malformed target', (registry) => { registry.agents[0].grants[0].target = 'contains spaces'; }],
    ['credential field in registry', (registry) => { registry.credential = 'a'.repeat(64); }],
    ['credential field on agent', (registry) => { registry.agents[0].credential = 'a'.repeat(64); }],
    ['credential field on grant', (registry) => { registry.agents[0].grants[0].credential = 'a'.repeat(64); }],
  ];

  for (const [name, mutate] of cases) {
    await t.test(name, async () => {
      const original = 'sentinel: do not replace\n';
      const path = await temporaryRegistry(original);
      const registry = validRegistry();
      mutate(registry);
      await assert.rejects(saveRegistry(path, registry));
      assert.equal(await readFile(path, 'utf8'), original);
    });
  }
});

test('a grant cannot be added to an unknown agent', () => {
  assert.throws(
    () => addGrant(validRegistry(), 'missing-agent', {
      id: 'grant-new',
      action: 'read',
      target: 'report:notes',
      issuedBy: 'operator:test',
    }),
    /Unknown agent/,
  );
});

test('saved JSON contains schema fields only and no credentials', async () => {
  const path = await temporaryRegistry();
  const { registry: saved } = await saveRegistry(path, validRegistry());
  const raw = JSON.parse(await readFile(path, 'utf8'));
  assert.deepEqual(Object.keys(raw).sort(), ['agents', 'id', 'version']);
  assert.deepEqual(Object.keys(raw.agents[0]).sort(), ['grants', 'id', 'whitelisted']);
  assert.deepEqual(Object.keys(raw.agents[0].grants[0]).sort(), ['action', 'id', 'issuedBy', 'target']);
  assert.equal(JSON.stringify(raw).includes('credential'), false);
  assert.equal(saved.version, 4);
});

test('corrupt or schema-invalid load reports an error without rewriting', async (t) => {
  await t.test('corrupt JSON', async () => {
    const original = '{not-json';
    const path = await temporaryRegistry(original);
    await assert.rejects(loadRegistry(path), /invalid JSON/);
    assert.equal(await readFile(path, 'utf8'), original);
  });

  await t.test('schema-invalid JSON', async () => {
    const original = `${JSON.stringify({ id: 'apex.test', version: 1, agents: [{ id: 'x', whitelisted: true, grants: [], credential: 'nope' }] }, null, 2)}\n`;
    const path = await temporaryRegistry(original);
    await assert.rejects(loadRegistry(path), /validation/);
    assert.equal(await readFile(path, 'utf8'), original);
  });
});

test('token authenticator accepts only a configured 64-hex credential', () => {
  const token = 'a'.repeat(64);
  const authenticate = createTokenAuthenticator([{ principalId: 'agent-one', token }]);
  assert.equal(authenticate(token), 'agent-one');
  assert.equal(authenticate('b'.repeat(64)), null);
});
