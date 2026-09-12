import { createHash } from 'node:crypto';
import { open, readFile, rename, rm } from 'node:fs/promises';
import { basename, dirname, join } from 'node:path';
import { createSwitchboard } from './switchboard.js';

// Validation-only: createSwitchboard compiles the registry at construction and
// does not call authenticate unless screen() runs.
const validateOnlyAuthenticator = () => null;

export function validateRegistry(registry) {
  createSwitchboard(registry, { authenticate: validateOnlyAuthenticator });
  return registry;
}

export async function loadRegistry(filePath) {
  let source;
  try {
    source = await readFile(filePath, 'utf8');
  } catch (error) {
    throw new Error(`Could not read registry: ${error.message}`);
  }

  let registry;
  try {
    registry = JSON.parse(source);
  } catch {
    throw new Error('Registry file contains invalid JSON.');
  }

  try {
    validateRegistry(registry);
  } catch (error) {
    throw new Error(`Registry failed validation: ${error.message}`);
  }
  return registry;
}

export function addGrant(registry, agentId, grant) {
  const agent = registry.agents.find((candidate) => candidate.id === agentId);
  if (!agent) throw new Error(`Unknown agent "${agentId}".`);
  agent.grants.push(grant);
  return registry;
}

export async function saveRegistry(filePath, registry) {
  validateRegistry(registry);
  const saved = structuredClone(registry);
  saved.version += 1;
  validateRegistry(saved);

  const json = `${JSON.stringify(saved, null, 2)}\n`;
  const temporaryPath = join(
    dirname(filePath),
    `.${basename(filePath)}.${process.pid}.${Date.now()}.tmp`,
  );

  try {
    const handle = await open(temporaryPath, 'wx', 0o600);
    try {
      await handle.writeFile(json, 'utf8');
      await handle.sync();
    } finally {
      await handle.close();
    }
    await rename(temporaryPath, filePath);
  } catch (error) {
    await rm(temporaryPath, { force: true });
    throw error;
  }

  return {
    registry: saved,
    hash: createHash('sha256').update(json).digest('hex'),
  };
}
