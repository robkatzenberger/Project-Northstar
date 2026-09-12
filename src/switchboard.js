import { createHash, timingSafeEqual } from 'node:crypto';

const namePattern = /^[A-Za-z][A-Za-z0-9._:-]{0,63}$/;
const targetPattern = /^[A-Za-z0-9][A-Za-z0-9._:/-]{0,255}$/;
const tokenPattern = /^[0-9a-f]{64}$/;
const isName = (value) => typeof value === 'string' && value === value.trim() && namePattern.test(value);
const isTarget = (value) => typeof value === 'string' && value === value.trim() && targetPattern.test(value);
const isToken = (value) => typeof value === 'string' && value.length === 64 && tokenPattern.test(value);

// Only explicit data fields are accepted; defaults and extra authority fields
// must not silently change the meaning of a registry or request.
function hasFields(value, fields) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) return false;
  const descriptors = Object.getOwnPropertyDescriptors(value);
  return Reflect.ownKeys(descriptors).length === fields.length
    && fields.every((field) => Object.hasOwn(descriptors, field)
      && Object.hasOwn(descriptors[field], 'value') && descriptors[field].enumerable);
}

function requireConfig(condition, message) {
  if (!condition) throw new TypeError(message);
}

function compileRegistry(config) {
  requireConfig(hasFields(config, ['id', 'version', 'agents'])
    && isName(config.id) && Number.isSafeInteger(config.version)
    && config.version > 0 && Array.isArray(config.agents), 'Invalid Switchboard registry');

  const agents = new Map();
  const grantIds = new Set();
  for (const agent of config.agents) {
    requireConfig(hasFields(agent, ['id', 'whitelisted', 'grants'])
      && isName(agent.id) && typeof agent.whitelisted === 'boolean'
      && Array.isArray(agent.grants), 'Invalid agent entry');
    requireConfig(!agents.has(agent.id), 'Duplicate agent ID');

    const permissions = new Map();
    for (const grant of agent.grants) {
      requireConfig(hasFields(grant, ['id', 'action', 'target', 'issuedBy'])
        && isName(grant.id) && isName(grant.action) && isTarget(grant.target)
        && isName(grant.issuedBy), 'Invalid action-target grant');
      requireConfig(!grantIds.has(grant.id), 'Duplicate grant ID');
      const pair = JSON.stringify([grant.action, grant.target]);
      requireConfig(!permissions.has(pair), 'Duplicate action-target grant');
      grantIds.add(grant.id);
      permissions.set(pair, Object.freeze({ ...grant }));
    }
    agents.set(agent.id, { whitelisted: agent.whitelisted, permissions });
  }
  return { registry: Object.freeze({ id: config.id, version: config.version }), agents };
}

/**
 * A small local credential adapter. Tokens are 32 random bytes encoded as
 * lowercase hex. Supply them from trusted host configuration, never the registry.
 * Possession authenticates a principal; it does not grant action scope.
 */
export function createTokenAuthenticator(entries) {
  requireConfig(Array.isArray(entries), 'Credentials must be an array');
  const principals = new Set();
  const digests = new Set();
  const credentials = [];
  for (const entry of entries) {
    requireConfig(hasFields(entry, ['principalId', 'token'])
      && isName(entry.principalId) && isToken(entry.token), 'Invalid credential entry');
    const digest = createHash('sha256').update(entry.token).digest();
    requireConfig(!principals.has(entry.principalId), 'Duplicate credential principal');
    requireConfig(!digests.has(digest.toString('hex')), 'Shared credentials are not permitted');
    principals.add(entry.principalId);
    digests.add(digest.toString('hex'));
    credentials.push({ principalId: entry.principalId, digest });
  }

  return (token) => {
    if (!isToken(token)) return null;
    const digest = createHash('sha256').update(token).digest();
    let principalId = null;
    for (const credential of credentials) {
      if (timingSafeEqual(digest, credential.digest)) principalId = credential.principalId;
    }
    return principalId;
  };
}

/**
 * Synchronous pre-policy screening. The trusted host supplies authenticate;
 * it returns a verified principal ID or null. PASS is not execution permission.
 */
export function createSwitchboard(config, options) {
  requireConfig(hasFields(options, ['authenticate'])
    && typeof options.authenticate === 'function', 'A trusted authenticator is required');
  const authenticate = options.authenticate;
  const { registry, agents } = compileRegistry(config);

  function screen(request) {
    let principalId = null;
    let intent = null;
    const result = (status, code, reason, grant = null) => Object.freeze({
      stage: 'SWITCHBOARD', status, code, reason, registry, principalId, intent, grant,
      next: status === 'PASS' ? 'CONSTRAINT_SCREENING' : null,
    });

    if (!hasFields(request, ['credential', 'intent'])) {
      return result('ERROR', 'INVALID_REQUEST', 'A credential and intent are required.');
    }
    try {
      const verified = authenticate(request.credential);
      if (verified && (typeof verified === 'object' || typeof verified === 'function')
        && typeof verified.then === 'function') {
        // This API is synchronous. Consume asynchronous rejection so an invalid
        // adapter cannot turn an already-blocked request into a host crash.
        Promise.resolve(verified).catch(() => {});
        return result('ERROR', 'AUTHENTICATION_UNAVAILABLE', 'Identity verification must be synchronous.');
      }
      principalId = verified;
    } catch {
      principalId = null;
      return result('ERROR', 'AUTHENTICATION_UNAVAILABLE', 'Identity verification failed.');
    }
    if (!isName(principalId)) {
      principalId = null;
      return result('DENY', 'UNAUTHENTICATED', 'A verified agent identity is required.');
    }

    const agent = agents.get(principalId);
    if (!agent) {
      return result('DENY', 'UNKNOWN_AGENT', 'The verified agent is not in the registry.');
    }
    if (!agent.whitelisted) {
      return result('DENY', 'NOT_WHITELISTED', 'The verified agent is not whitelisted.');
    }

    const submitted = request.intent;
    if (!hasFields(submitted, ['agentId', 'action', 'target'])
      || !isName(submitted.agentId) || !isName(submitted.action) || !isTarget(submitted.target)) {
      return result('ERROR', 'INVALID_INTENT', 'An exact agent, action, and target are required.');
    }
    intent = Object.freeze({ ...submitted });
    if (submitted.agentId !== principalId) {
      return result('DENY', 'IDENTITY_MISMATCH', 'The declaration names a different agent.');
    }

    const grant = agent.permissions.get(JSON.stringify([intent.action, intent.target]));
    if (!grant) {
      return result('DENY', 'SCOPE_NOT_GRANTED', 'No grant covers this action and target.');
    }
    return result('PASS', 'SCOPE_GRANTED', 'A whitelist grant covers this action and target.', grant);
  }

  return Object.freeze({ screen });
}
