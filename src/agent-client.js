import { createHash } from 'node:crypto';

const DEFAULT_URL = 'http://127.0.0.1:43129';
const TIMEOUT_MS = 10000;
const MAX_RESPONSE_BYTES = 32768;
const model = 'approve_escalate_v1';
const names = /^[A-Za-z][A-Za-z0-9._:-]{0,63}$/;
const targets = /^[A-Za-z0-9][A-Za-z0-9._:/-]{0,255}$/;
const hashes = /^[a-f0-9]{64}$/;
const ids = /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/;
const isName = (value) => typeof value === 'string' && value === value.trim() && names.test(value);
const isTarget = (value) => typeof value === 'string' && value === value.trim() && targets.test(value);
const isHash = (value) => typeof value === 'string' && value.length === 64 && hashes.test(value);
const isId = (value) => typeof value === 'string' && value === value.trim() && ids.test(value);
const version = (value) => Number.isSafeInteger(value) && value > 0;
const text = (value) => typeof value === 'string' && value.isWellFormed() && value.length > 0 && value.length <= 2048;
const date = (value) => typeof value === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(value)
  && Number.isFinite(Date.parse(value)) && new Date(value).toISOString() === value;

function fields(value, keys) {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    && Object.keys(value).length === keys.length && keys.every((key) => Object.hasOwn(value, key));
}
function fail(code, message) {
  return Object.assign(new Error(message), { name: 'AgentClientError', code });
}
function valid(condition) {
  if (!condition) throw fail('INVALID_RESPONSE', 'The auditor returned an invalid or mismatched record.');
}
function intent(value) {
  return fields(value, ['agentId', 'action', 'target']) && isName(value.agentId)
    && isName(value.action) && isTarget(value.target);
}
const sameIntent = (left, right) => intent(left) && intent(right)
  && ['agentId', 'action', 'target'].every((key) => left[key] === right[key]);
function immutable(value) {
  if (value && typeof value === 'object') {
    Object.values(value).forEach(immutable);
    Object.freeze(value);
  }
  return value;
}

function validateResult(result, expected, withAudit) {
  valid(fields(result, ['version', 'decisionModel', 'status', 'code', 'reason', 'stage', 'policy',
    'switchboard', 'matchedRules', 'execution', ...(withAudit ? ['audit'] : [])]));
  valid(result.version === '0.0.2' && result.decisionModel === model && result.execution === 'NOT_EXECUTED'
    && text(result.reason) && fields(result.policy, ['version', 'hash'])
    && version(result.policy.version) && isHash(result.policy.hash));
  if (withAudit) valid(fields(result.audit, ['id', 'recordedAt']) && isId(result.audit.id) && date(result.audit.recordedAt));
  const screening = result.switchboard;
  valid(fields(screening, ['stage', 'status', 'code', 'reason', 'registry', 'principalId', 'intent', 'grant', 'next'])
    && screening.stage === 'SWITCHBOARD' && text(screening.reason)
    && fields(screening.registry, ['id', 'version']) && isName(screening.registry.id) && version(screening.registry.version)
    && (screening.principalId === null || isName(screening.principalId))
    && (screening.intent === null || sameIntent(screening.intent, expected)));
  valid(Array.isArray(result.matchedRules) && result.matchedRules.length <= 2);
  const seen = new Set();
  for (const rule of result.matchedRules) {
    valid(fields(rule, ['line', 'decision', 'action', 'target']) && version(rule.line)
      && ['APPROVE', 'ESCALATE'].includes(rule.decision) && rule.action === expected.action && rule.target === expected.target
      && !seen.has(rule.decision));
    seen.add(rule.decision);
  }
  if (result.stage === 'SWITCHBOARD') {
    valid(screening.next === null && screening.grant === null && result.matchedRules.length === 0
      && result.code === screening.code && result.reason === screening.reason);
    if (result.status === 'BLOCKED') {
      valid(screening.status === 'DENY' && ['UNAUTHENTICATED', 'UNKNOWN_AGENT', 'NOT_WHITELISTED',
        'IDENTITY_MISMATCH', 'SCOPE_NOT_GRANTED'].includes(result.code));
    } else {
      valid(result.status === 'ERROR' && screening.status === 'ERROR'
        && ['INVALID_REQUEST', 'AUTHENTICATION_UNAVAILABLE', 'INVALID_INTENT'].includes(result.code));
    }
    return;
  }
  valid(result.stage === 'GATE' && screening.status === 'PASS' && screening.code === 'SCOPE_GRANTED'
    && screening.next === 'CONSTRAINT_SCREENING' && screening.principalId === expected.agentId
    && sameIntent(screening.intent, expected) && fields(screening.grant, ['id', 'action', 'target', 'issuedBy'])
    && isName(screening.grant.id) && isName(screening.grant.issuedBy)
    && screening.grant.action === expected.action && screening.grant.target === expected.target);
  if (result.status === 'APPROVE') {
    valid(result.code === 'RULE_APPROVE' && seen.size === 1 && seen.has('APPROVE'));
  } else if (result.status === 'ESCALATE') {
    valid((result.code === 'NO_MATCHING_RULE' && seen.size === 0)
      || (result.code === 'HUMAN_VERIFICATION_REQUIRED' && seen.has('ESCALATE')));
  } else valid(result.status === 'ERROR' && result.code === 'UNKNOWN_ACTION' && seen.size === 0);
}

async function requestJson({ url = DEFAULT_URL, credential, signal }, path, body) {
  if (!isHash(credential)) throw fail('INVALID_CREDENTIAL', 'A valid agent credential is required.');
  let origin;
  try { origin = new URL(url); } catch { throw fail('INVALID_URL', 'Use a local auditor HTTP origin.'); }
  if (origin.protocol !== 'http:' || !['127.0.0.1', 'localhost', '[::1]'].includes(origin.hostname)
    || origin.username || origin.password || origin.pathname !== '/' || origin.search || origin.hash) {
    throw fail('INVALID_URL', 'Use a local auditor HTTP origin.');
  }
  if (signal !== undefined && !(signal instanceof AbortSignal)) throw fail('INVALID_SIGNAL', 'Use an AbortSignal for cancellation.');
  if (signal?.aborted) throw fail('REQUEST_ABORTED', 'The auditor request was cancelled.');
  const controller = new AbortController();
  let rejectCancellation;
  const cancelled = new Promise((_, reject) => { rejectCancellation = reject; });
  const cancel = () => { controller.abort(); rejectCancellation(fail('REQUEST_ABORTED', 'The auditor request was cancelled.')); };
  signal?.addEventListener('abort', cancel, { once: true });
  const timer = setTimeout(() => {
    controller.abort();
    rejectCancellation(fail('REQUEST_TIMEOUT', 'The auditor did not respond within 10 seconds.'));
  }, TIMEOUT_MS);
  const endpoint = new URL(path, origin).href;
  async function fetchRecord() {
    const response = await fetch(endpoint, { method: body === undefined ? 'GET' : 'POST',
      redirect: 'error', signal: controller.signal,
      headers: body === undefined ? { accept: 'application/json', authorization: `Bearer ${credential}` }
        : { accept: 'application/json', 'content-type': 'application/json' },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
    if (!response.ok) throw fail('HTTP_ERROR', `The auditor rejected the request (HTTP ${response.status}).`);
    valid(!response.redirected && (!response.url || response.url === endpoint)
      && response.headers.get('content-type')?.split(';')[0].trim() === 'application/json' && response.body);
    const reader = response.body.getReader();
    const chunks = [];
    let bytes = 0;
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      bytes += value.byteLength;
      if (bytes > MAX_RESPONSE_BYTES) {
        await reader.cancel();
        throw fail('RESPONSE_TOO_LARGE', 'The auditor response exceeded 32 KiB.');
      }
      chunks.push(value);
    }
    const serialized = Buffer.concat(chunks).toString('utf8');
    valid(!serialized.includes(credential));
    try { return JSON.parse(serialized); } catch { throw fail('INVALID_RESPONSE', 'The auditor returned invalid JSON.'); }
  }
  try {
    return await Promise.race([fetchRecord(), cancelled]);
  } catch (error) {
    if (error?.name === 'AgentClientError') throw error;
    throw fail('AUDITOR_UNAVAILABLE', 'The auditor request failed; no valid assessment was received.');
  } finally {
    controller.abort();
    clearTimeout(timer);
    signal?.removeEventListener('abort', cancel);
  }
}

/** Submit declared scope before work. A returned APPROVE is an assessment, never execution authorization. */
export async function submitIntent({ url, credential, intent: submitted, signal } = {}) {
  if (!intent(submitted)) throw fail('INVALID_INTENT', 'Submit exactly agentId, action, and target.');
  const expected = { ...submitted };
  const result = await requestJson({ url, credential, signal }, '/api/gate/evaluate', { credential, intent: expected });
  validateResult(result, expected, true);
  return immutable(result);
}

/** Read only the authenticated agent's saved assessment and separate human outcome. */
export async function observeAssessment({ url, credential, assessmentId, signal } = {}) {
  if (!isId(assessmentId)) throw fail('INVALID_ASSESSMENT_ID', 'A saved assessment ID is required.');
  const record = await requestJson({ url, credential, signal }, `/api/agent/assessments/${encodeURIComponent(assessmentId)}`);
  valid(fields(record, ['assessment', 'review', 'current']) && typeof record.current === 'boolean');
  const assessment = record.assessment;
  valid(fields(assessment, ['id', 'recordedAt', 'request', 'result']) && assessment.id === assessmentId
    && date(assessment.recordedAt) && intent(assessment.request));
  validateResult(assessment.result, assessment.request, false);
  if (record.review !== null) {
    const review = record.review;
    valid(fields(review, ['id', 'recordedAt', 'type', 'assessmentId', 'assessmentHash', 'decisionModel',
      'decision', 'comment', 'reviewer', 'actor', 'execution'])
      && isId(review.id) && date(review.recordedAt) && review.type === 'human_review'
      && review.assessmentId === assessmentId && review.decisionModel === model
      && ['APPROVE', 'DENY'].includes(review.decision) && typeof review.comment === 'string' && review.comment.isWellFormed()
      && review.comment === review.comment.trim() && review.comment.length > 0 && Buffer.byteLength(review.comment) <= 2000
      && review.reviewer === 'operator' && review.actor === 'operator' && review.execution === 'NOT_EXECUTED'
      && assessment.result.status === 'ESCALATE' && assessment.result.stage === 'GATE'
      && review.assessmentHash === createHash('sha256').update(JSON.stringify({ request: assessment.request, result: assessment.result })).digest('hex'));
  }
  if (record.current) valid(assessment.result.switchboard.status === 'PASS' && assessment.result.stage === 'GATE');
  return immutable(record);
}
