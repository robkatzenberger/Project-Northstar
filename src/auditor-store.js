import { randomUUID, createHash } from 'node:crypto';
import { mkdir, open, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { parseRules, parseLegacyRules } from './gate.js';
import { validateRegistry } from './registry-store.js';

const MAX_JOURNAL_BYTES = 16 * 1024 * 1024;
const DECISION_MODEL = 'approve_escalate_v1';
const hash = (text) => createHash('sha256').update(text).digest('hex');
const policyView = (policy) => ({ ...policy, hash: hash(policy.text) });
const assessmentHash = (assessment) => hash(JSON.stringify({ request: assessment.request, result: assessment.result }));

function hasFields(value, fields) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) return false;
  const descriptors = Object.getOwnPropertyDescriptors(value);
  return Reflect.ownKeys(descriptors).length === fields.length && fields.every((field) =>
    Object.hasOwn(descriptors, field) && Object.hasOwn(descriptors[field], 'value') && descriptors[field].enumerable);
}

function reviewable(assessment) {
  const result = assessment?.result;
  const screening = result?.switchboard;
  const intent = screening?.intent;
  const grant = screening?.grant;
  return result?.version === '0.0.2' && result.decisionModel === DECISION_MODEL
    && result.stage === 'GATE' && result.status === 'ESCALATE' && result.execution === 'NOT_EXECUTED'
    && screening?.stage === 'SWITCHBOARD' && screening.status === 'PASS' && screening.code === 'SCOPE_GRANTED'
    && hasFields(intent, ['agentId', 'action', 'target'])
    && hasFields(assessment.request, ['agentId', 'action', 'target'])
    && ['agentId', 'action', 'target'].every((field) => typeof intent[field] === 'string' && intent[field] === assessment.request[field])
    && screening.principalId === intent.agentId
    && hasFields(grant, ['id', 'action', 'target', 'issuedBy'])
    && grant.action === intent.action && grant.target === intent.target
    && hasFields(screening.registry, ['id', 'version'])
    && Number.isSafeInteger(result.policy?.version) && result.policy.version > 0
    && typeof result.policy.hash === 'string' && /^[a-f0-9]{64}$/.test(result.policy.hash);
}

function stale(assessment, policy, registry) {
  const result = assessment.result;
  const screening = result.switchboard;
  const agent = registry?.agents.find(({ id }) => id === screening.principalId);
  return policy.decisionModel !== DECISION_MODEL
    || result.policy.version !== policy.version || result.policy.hash !== hash(policy.text)
    || screening.registry.id !== registry?.id || screening.registry.version !== registry?.version
    || agent?.whitelisted !== true
    || !agent.grants.some((grant) => ['id', 'action', 'target', 'issuedBy'].every((field) => grant[field] === screening.grant[field]));
}

function validateReview(input) {
  if (!hasFields(input, ['assessmentId', 'decision', 'comment'])
    || typeof input.assessmentId !== 'string' || !input.assessmentId.isWellFormed()
    || input.assessmentId !== input.assessmentId.trim() || !input.assessmentId || input.assessmentId.length > 128
    || !['APPROVE', 'DENY'].includes(input.decision)
    || typeof input.comment !== 'string' || !input.comment.isWellFormed()
    || !input.comment.trim() || Buffer.byteLength(input.comment.trim(), 'utf8') > 2000) {
    throw fault('INVALID_REVIEW', 'Provide an assessment ID, APPROVE or DENY, and a required context comment of at most 2000 UTF-8 bytes.');
  }
  return { assessmentId: input.assessmentId, decision: input.decision, comment: input.comment.trim() };
}
export function fault(code, message, status = 400) {
  return Object.assign(new Error(message), { code, status });
}

// A single-server local journal owns saved policy and assessments. It is not
// a tamper-proof ledger. Success is returned only after the event is synced.
export async function createAuditorStore(dataDir, initialRegistry) {
  await mkdir(dataDir, { recursive: true, mode: 0o700 });
  const path = join(dataDir, 'audit.jsonl');
  let source = '';
  try { source = await readFile(path, 'utf8'); } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
  if (Buffer.byteLength(source) > MAX_JOURNAL_BYTES) throw fault('AUDIT_CAPACITY', 'Audit storage is full.', 503);
  let policy = { version: 0, text: '' };
  let registry = null;
  const records = [];
  const ids = new Set();
  const assessments = new Map();
  const reviews = new Map();
  if (source && !source.endsWith('\n')) throw fault('AUDIT_CORRUPT', 'Incomplete audit record; recover the journal before starting.', 503);
  for (const line of source.split('\n').filter(Boolean)) {
    let record;
    try { record = JSON.parse(line); } catch {
      throw fault('AUDIT_CORRUPT', 'Invalid audit JSON.', 503);
    }
    if (!record || typeof record.id !== 'string' || !record.id || ids.has(record.id) || !record.recordedAt
      || !['policy_saved', 'assessment', 'credential_issued', 'registry_saved', 'human_review'].includes(record.type)) {
      throw fault('AUDIT_CORRUPT', 'Invalid audit record.', 503);
    }
    ids.add(record.id);
    if (record.type === 'policy_saved') {
      try {
        if (!record.policy || (record.policy.decisionModel !== undefined && record.policy.decisionModel !== DECISION_MODEL)) throw new Error();
        (record.policy.decisionModel === DECISION_MODEL ? parseRules : parseLegacyRules)(record.policy.text);
      } catch {
        throw fault('AUDIT_CORRUPT', 'Invalid saved policy.', 503);
      }
      if (record.policy.version !== policy.version + 1 || record.policy.hash !== hash(record.policy.text)) {
        throw fault('AUDIT_CORRUPT', 'Invalid policy history.', 503);
      }
      policy = { version: record.policy.version, text: record.policy.text,
        ...(record.policy.decisionModel === DECISION_MODEL ? { decisionModel: DECISION_MODEL } : {}) };
    }
    if (record.type === 'registry_saved') {
      try { validateRegistry(record.registry); } catch {
        throw fault('AUDIT_CORRUPT', 'Invalid saved registry.', 503);
      }
      if ((registry && record.registry.version !== registry.version + 1)
        || record.hash !== hash(JSON.stringify(record.registry))) throw fault('AUDIT_CORRUPT', 'Invalid registry history.', 503);
      registry = structuredClone(record.registry);
    }
    if (record.type === 'assessment') assessments.set(record.id, record);
    if (record.type === 'human_review') {
      const assessment = assessments.get(record.assessmentId);
      let submitted;
      try { submitted = validateReview({ assessmentId: record.assessmentId, decision: record.decision, comment: record.comment }); } catch {
        throw fault('AUDIT_CORRUPT', 'Invalid saved human review.', 503);
      }
      if (!hasFields(record, ['id', 'recordedAt', 'type', 'assessmentId', 'assessmentHash', 'decisionModel', 'decision', 'comment', 'reviewer', 'actor', 'execution'])
        || record.decisionModel !== DECISION_MODEL || record.reviewer !== 'operator' || record.actor !== 'operator'
        || record.execution !== 'NOT_EXECUTED' || submitted.comment !== record.comment
        || !reviewable(assessment) || reviews.has(record.assessmentId)
        || record.assessmentHash !== assessmentHash(assessment)
        || (record.decision === 'APPROVE' && stale(assessment, policy, registry))) {
        throw fault('AUDIT_CORRUPT', 'Invalid human review linkage or authority state.', 503);
      }
      reviews.set(record.assessmentId, record);
    }
    records.push(record);
  }
  let bytes = Buffer.byteLength(source);
  let failed = false;
  let queue = Promise.resolve();
  function serial(task) {
    const next = queue.then(task);
    queue = next.catch(() => {});
    return next;
  }
  async function append(type, fields) {
    if (failed) throw fault('AUDIT_UNAVAILABLE', 'Audit writing failed; restart after repairing storage.', 503);
    const record = { id: randomUUID(), recordedAt: new Date().toISOString(), type, ...structuredClone(fields) };
    const line = `${JSON.stringify(record)}\n`;
    if (bytes + Buffer.byteLength(line) > MAX_JOURNAL_BYTES) throw fault('AUDIT_CAPACITY', 'Audit storage is full.', 503);
    let handle;
    try {
      handle = await open(path, 'a', 0o600);
      await handle.writeFile(line, 'utf8');
      await handle.sync();
      await handle.close();
      handle = null;
    } catch {
      failed = true;
      await handle?.close().catch(() => {});
      throw fault('AUDIT_UNAVAILABLE', 'Could not record the result; no successful assessment was returned.', 503);
    }
    bytes += Buffer.byteLength(line);
    records.push(record);
    return structuredClone(record);
  }
  if (!policy.version) {
    const initial = { version: 1, text: '# Enter APPROVE or ESCALATE rules. No matching rule means human escalation after authority screening.\n', decisionModel: DECISION_MODEL };
    await append('policy_saved', { policy: policyView(initial), actor: 'operator:initialization' });
    policy = initial;
  }
  if (policy.decisionModel !== DECISION_MODEL) {
    const migrated = { version: policy.version + 1, decisionModel: DECISION_MODEL,
      text: '# Decision model migrated to approve_escalate_v1. Historical rules remain in the journal.\n# Enter reviewed APPROVE or ESCALATE rules. No matching rule means human escalation after authority screening.\n' };
    await append('policy_saved', { policy: policyView(migrated), actor: 'operator:model-migration' });
    policy = migrated;
  }
  if (!registry) {
    validateRegistry(initialRegistry);
    const initial = structuredClone(initialRegistry);
    await append('registry_saved', { registry: initial, hash: hash(JSON.stringify(initial)), actor: 'operator:initialization' });
    registry = initial;
  }
  return {
    getPolicy: () => policyView(policy),
    getRegistry: () => structuredClone(registry),
    getAgentAssessment: (assessmentId, principalId) => {
      const saved = assessments.get(assessmentId);
      // Read access follows verified identity, never the declaration's agentId.
      if (!principalId || saved?.result?.switchboard?.principalId !== principalId) {
        throw fault('ASSESSMENT_NOT_FOUND', 'Assessment was not found.', 404);
      }
      const result = saved.result;
      const current = result.version === '0.0.2' && result.decisionModel === DECISION_MODEL
        && result.stage === 'GATE' && ['APPROVE', 'ESCALATE'].includes(result.status)
        && result.switchboard.status === 'PASS' && !stale(saved, policy, registry);
      return structuredClone({
        assessment: { id: saved.id, recordedAt: saved.recordedAt, request: saved.request, result },
        review: reviews.get(saved.id) ?? null,
        current,
      });
    },
    history: (limit = 20) => structuredClone(records.slice(-Math.min(100, Math.max(1, limit))).reverse()),
    listReviews: () => {
      const eligible = [...assessments.values()].filter(reviewable);
      // Journal order is the accepted order, even if the wall clock moves.
      const pending = eligible.filter((assessment) => !reviews.has(assessment.id));
      const resolved = eligible.filter((assessment) => reviews.has(assessment.id)).reverse();
      return structuredClone([...pending, ...resolved].slice(0, 100).map((assessment) => ({
        assessmentId: assessment.id, recordedAt: assessment.recordedAt, request: assessment.request,
        result: assessment.result, review: reviews.get(assessment.id) ?? null,
      })));
    },
    resolveReview: async (input) => {
      const submitted = validateReview(input);
      return serial(async () => {
        const assessment = assessments.get(submitted.assessmentId);
        if (!assessment) throw fault('ASSESSMENT_NOT_FOUND', 'Assessment was not found.', 404);
        if (!reviewable(assessment)) throw fault('REVIEW_NOT_ALLOWED', 'Only current gate escalations can receive a human review.', 409);
        if (reviews.has(assessment.id)) throw fault('REVIEW_ALREADY_RESOLVED', 'This assessment already has a human decision.', 409);
        if (submitted.decision === 'APPROVE' && stale(assessment, policy, registry)) {
          throw fault('REVIEW_STALE', 'Policy or agent authority changed. Obtain a fresh assessment before approval.', 409);
        }
        const review = await append('human_review', { ...submitted, assessmentHash: assessmentHash(assessment),
          decisionModel: DECISION_MODEL, reviewer: 'operator', actor: 'operator', execution: 'NOT_EXECUTED' });
        reviews.set(assessment.id, review);
        return structuredClone(review);
      });
    },
    savePolicy: (version, text) => serial(async () => {
      if (version !== policy.version) throw fault('VERSION_CONFLICT', 'Rules changed since you loaded them. Reload before saving.', 409);
      parseRules(text);
      const next = { version: policy.version + 1, text, decisionModel: DECISION_MODEL };
      await append('policy_saved', { policy: policyView(next), actor: 'operator' });
      policy = next;
      return policyView(policy);
    }),
    saveRegistry: (submitted) => serial(async () => {
      if (submitted?.version !== registry.version) throw fault('VERSION_CONFLICT', 'Registry changed; reload before saving.', 409);
      validateRegistry(submitted);
      const next = structuredClone(submitted);
      next.version += 1;
      validateRegistry(next);
      const digest = hash(JSON.stringify(next));
      await append('registry_saved', { registry: next, hash: digest, actor: 'operator' });
      registry = next;
      return { registry: structuredClone(registry), hash: digest };
    }),
    assess: (evaluate, request) => {
      const declaration = structuredClone(request);
      return serial(async () => {
        const result = structuredClone(await evaluate(policyView(policy), structuredClone(registry)));
        const record = await append('assessment', { request: declaration, result });
        assessments.set(record.id, record);
        return { ...result, audit: { id: record.id, recordedAt: record.recordedAt } };
      });
    },
    issueCredential: (principalId, activate) => serial(async () => {
      if (!registry.agents.some((agent) => agent.id === principalId)) throw fault('UNKNOWN_AGENT', 'Enroll that agent in the registry first.');
      await append('credential_issued', { principalId, ephemeral: true });
      activate();
    }),
  };
}
