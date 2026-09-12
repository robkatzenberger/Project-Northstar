import { createHash, randomUUID } from 'node:crypto';
import { open, readFile, rename, rm, mkdir, writeFile } from 'node:fs/promises';
import { basename, dirname, join } from 'node:path';

const PENDING_STATUSES = new Set(['PENDING', 'APPROVED', 'DENIED', 'ERROR']);
const DECISION_STATUSES = new Set(['APPROVED', 'DENIED']);
const DECIDE_STATUSES = new Set(['APPROVE', 'DENY', 'APPROVED', 'DENIED']);

export function hashIntentSummary(intentSummary) {
  return createHash('sha256').update(String(intentSummary ?? ''), 'utf8').digest('hex');
}

function isNonEmptyString(value) {
  return typeof value === 'string' && value.trim().length > 0;
}

export function validatePendingRecord(record, lineNumber) {
  const where = lineNumber == null ? 'pending record' : `pending.jsonl line ${lineNumber}`;
  if (!record || typeof record !== 'object' || Array.isArray(record)) {
    throw new Error(`${where}: expected an object.`);
  }
  if (!isNonEmptyString(record.requestId)) {
    throw new Error(`${where}: requestId is required.`);
  }
  if (!PENDING_STATUSES.has(record.status)) {
    throw new Error(`${where}: status must be PENDING, APPROVED, DENIED, or ERROR.`);
  }
  if (!isNonEmptyString(record.at)) {
    throw new Error(`${where}: at is required.`);
  }
  if (!isNonEmptyString(record.principalId)) {
    throw new Error(`${where}: principalId is required.`);
  }
  if (!isNonEmptyString(record.action)) {
    throw new Error(`${where}: action is required.`);
  }
  if (!isNonEmptyString(record.target)) {
    throw new Error(`${where}: target is required.`);
  }
  if (!isNonEmptyString(record.intentSummary)) {
    throw new Error(`${where}: intentSummary is required.`);
  }
  if (record.notScreened != null && typeof record.notScreened !== 'boolean') {
    throw new Error(`${where}: notScreened must be a boolean when present.`);
  }
  return {
    requestId: record.requestId.trim(),
    status: record.status,
    at: record.at.trim(),
    principalId: record.principalId.trim(),
    action: record.action.trim(),
    target: record.target.trim(),
    intentSummary: record.intentSummary.trim(),
    ...(record.notScreened === true ? { notScreened: true } : {}),
  };
}

export function validateDecisionReceipt(record, lineNumber) {
  const where = lineNumber == null ? 'decision receipt' : `decisions.jsonl line ${lineNumber}`;
  if (!record || typeof record !== 'object' || Array.isArray(record)) {
    throw new Error(`${where}: expected an object.`);
  }
  if (!isNonEmptyString(record.id)) {
    throw new Error(`${where}: id is required.`);
  }
  if (!isNonEmptyString(record.requestId)) {
    throw new Error(`${where}: requestId is required.`);
  }
  if (!DECISION_STATUSES.has(record.status)) {
    throw new Error(`${where}: status must be APPROVED or DENIED.`);
  }
  if (!isNonEmptyString(record.at)) {
    throw new Error(`${where}: at is required.`);
  }
  if (!isNonEmptyString(record.operatorId)) {
    throw new Error(`${where}: operatorId is required.`);
  }
  if (!isNonEmptyString(record.intentSummary)) {
    throw new Error(`${where}: intentSummary is required.`);
  }
  if (record.notScreened != null && typeof record.notScreened !== 'boolean') {
    throw new Error(`${where}: notScreened must be a boolean when present.`);
  }
  const receipt = {
    id: record.id.trim(),
    requestId: record.requestId.trim(),
    status: record.status,
    at: record.at.trim(),
    operatorId: record.operatorId.trim(),
    intentSummary: record.intentSummary.trim(),
  };
  if (isNonEmptyString(record.principalId)) receipt.principalId = record.principalId.trim();
  if (isNonEmptyString(record.action)) receipt.action = record.action.trim();
  if (isNonEmptyString(record.target)) receipt.target = record.target.trim();
  if (record.notScreened === true) receipt.notScreened = true;
  return receipt;
}

async function readJsonl(filePath, validate) {
  let source;
  try {
    source = await readFile(filePath, 'utf8');
  } catch (error) {
    if (error && error.code === 'ENOENT') return [];
    throw new Error(`Could not read ${basename(filePath)}: ${error.message}`);
  }

  if (source.length === 0) return [];

  const lines = source.split(/\n/);
  // Allow a trailing newline without treating it as an empty record.
  if (lines.length && lines[lines.length - 1] === '') lines.pop();

  const records = [];
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() === '') {
      throw new Error(`${basename(filePath)} line ${index + 1}: empty line is not allowed.`);
    }
    let parsed;
    try {
      parsed = JSON.parse(line);
    } catch {
      throw new Error(`${basename(filePath)} line ${index + 1}: invalid JSON.`);
    }
    records.push(validate(parsed, index + 1));
  }
  return records;
}

function serializeJsonl(records) {
  if (records.length === 0) return '';
  return `${records.map((record) => JSON.stringify(record)).join('\n')}\n`;
}

async function writeFileAtomic(filePath, contents) {
  await mkdir(dirname(filePath), { recursive: true });
  const temporaryPath = join(
    dirname(filePath),
    `.${basename(filePath)}.${process.pid}.${Date.now()}.tmp`,
  );
  try {
    const handle = await open(temporaryPath, 'wx', 0o600);
    try {
      await handle.writeFile(contents, 'utf8');
      await handle.sync();
    } finally {
      await handle.close();
    }
    await rename(temporaryPath, filePath);
  } catch (error) {
    await rm(temporaryPath, { force: true });
    throw error;
  }
}

async function appendLineAtomic(filePath, line) {
  await mkdir(dirname(filePath), { recursive: true });
  const handle = await open(filePath, 'a', 0o600);
  try {
    await handle.writeFile(line.endsWith('\n') ? line : `${line}\n`, 'utf8');
    await handle.sync();
  } finally {
    await handle.close();
  }
}

export function resolveAuditPaths(dataDir) {
  return {
    pendingPath: join(dataDir, 'pending.jsonl'),
    decisionsPath: join(dataDir, 'decisions.jsonl'),
  };
}

export async function loadPending(dataDir) {
  const { pendingPath } = resolveAuditPaths(dataDir);
  return readJsonl(pendingPath, validatePendingRecord);
}

export async function loadDecisions(dataDir) {
  const { decisionsPath } = resolveAuditPaths(dataDir);
  return readJsonl(decisionsPath, validateDecisionReceipt);
}

export async function loadAuditState(dataDir) {
  const [pending, decisions] = await Promise.all([
    loadPending(dataDir),
    loadDecisions(dataDir),
  ]);
  return { pending, decisions };
}

const STATUS_RANK = {
  PENDING: 0,
  ERROR: 1,
  DENIED: 2,
  APPROVED: 3,
};

/** Pending-first ordering; Denied stays visible under the default filter set. */
export function sortPendingFirst(records) {
  return [...records].sort((left, right) => {
    const rankDelta = (STATUS_RANK[left.status] ?? 99) - (STATUS_RANK[right.status] ?? 99);
    if (rankDelta !== 0) return rankDelta;
    return String(right.at).localeCompare(String(left.at));
  });
}

/**
 * Default filter keeps Pending first and never hides Denied.
 * status: undefined | null | 'ALL' | specific status string | string[]
 */
export function filterRecords(records, filters = {}) {
  const {
    status,
    agentId,
    principalId,
    action,
    requestId,
    since,
    hideDenied = false,
  } = filters;

  const statusFilter = normalizeStatusFilter(status);
  const agent = (agentId ?? principalId ?? '').trim().toLowerCase();
  const actionNeedle = (action ?? '').trim().toLowerCase();
  const requestNeedle = (requestId ?? '').trim().toLowerCase();
  const sinceMs = since ? Date.parse(since) : NaN;

  const filtered = records.filter((record) => {
    if (statusFilter && !statusFilter.has(record.status)) return false;
    // Denied never hidden by default; only when hideDenied is explicitly true
    // and the caller is not filtering to DENIED specifically.
    if (
      hideDenied === true &&
      record.status === 'DENIED' &&
      !(statusFilter && statusFilter.has('DENIED'))
    ) {
      return false;
    }
    if (agent && record.principalId.toLowerCase() !== agent) return false;
    if (actionNeedle && !record.action.toLowerCase().includes(actionNeedle)) return false;
    if (requestNeedle && !record.requestId.toLowerCase().includes(requestNeedle)) return false;
    if (Number.isFinite(sinceMs)) {
      const atMs = Date.parse(record.at);
      if (!Number.isFinite(atMs) || atMs < sinceMs) return false;
    }
    return true;
  });

  return sortPendingFirst(filtered);
}

function normalizeStatusFilter(status) {
  if (status == null || status === '' || status === 'ALL') return null;
  if (Array.isArray(status)) {
    const set = new Set(status.map((value) => String(value).toUpperCase()));
    return set.size ? set : null;
  }
  return new Set([String(status).toUpperCase()]);
}

/** Default list view: Pending-first; Denied remains visible. */
export function defaultListView(records) {
  return filterRecords(records, { hideDenied: false });
}

function normalizeDecisionStatus(decision) {
  const raw = String(decision || '').toUpperCase();
  if (!DECIDE_STATUSES.has(raw)) {
    throw new Error('Decision must be APPROVE or DENY.');
  }
  return raw === 'APPROVE' || raw === 'APPROVED' ? 'APPROVED' : 'DENIED';
}

export async function appendDecision(dataDir, receipt) {
  const validated = validateDecisionReceipt(receipt);
  const { decisionsPath } = resolveAuditPaths(dataDir);
  await appendLineAtomic(decisionsPath, JSON.stringify(validated));
  return validated;
}

/**
 * Validate → append decision receipt → rewrite pending atomically.
 * Fail closed: validation errors write nothing; append happens only after
 * validation; pending rewrite uses temp+rename. Double-decide is rejected.
 */
export async function decide(dataDir, requestId, decision, operatorId, intentSummaryHash) {
  if (!isNonEmptyString(requestId)) {
    throw new Error('requestId is required.');
  }
  if (!isNonEmptyString(operatorId)) {
    throw new Error('operatorId is required.');
  }
  if (!isNonEmptyString(intentSummaryHash)) {
    throw new Error('intentSummaryHash is required.');
  }

  const status = normalizeDecisionStatus(decision);
  const { pendingPath, decisionsPath } = resolveAuditPaths(dataDir);

  // Load and validate fully before any write.
  const pending = await loadPending(dataDir);
  const decisions = await loadDecisions(dataDir);

  const priorDecisionsBytes = await readFile(decisionsPath, 'utf8').catch((error) => {
    if (error && error.code === 'ENOENT') return '';
    throw error;
  });

  const targetId = requestId.trim();
  const index = pending.findIndex((record) => record.requestId === targetId);
  if (index < 0) {
    throw new Error(`Unknown requestId "${targetId}".`);
  }

  const current = pending[index];
  if (current.status !== 'PENDING') {
    throw new Error(`Request "${targetId}" is already ${current.status}; second decide rejected.`);
  }

  const existingDecision = decisions.find((record) => record.requestId === targetId);
  if (existingDecision) {
    throw new Error(`Request "${targetId}" already has a decision; second decide rejected.`);
  }

  const expectedHash = hashIntentSummary(current.intentSummary);
  if (expectedHash !== intentSummaryHash.trim().toLowerCase()) {
    throw new Error('intentSummaryHash does not match the pending record.');
  }

  const receipt = {
    id: randomUUID(),
    requestId: current.requestId,
    status,
    at: new Date().toISOString(),
    operatorId: operatorId.trim(),
    intentSummary: current.intentSummary,
    principalId: current.principalId,
    action: current.action,
    target: current.target,
    ...(current.notScreened === true ? { notScreened: true } : {}),
  };
  validateDecisionReceipt(receipt);

  const nextPending = pending.map((record, recordIndex) =>
    recordIndex === index ? { ...record, status } : record,
  );
  // Re-validate the whole pending set before writing.
  nextPending.forEach((record, recordIndex) => validatePendingRecord(record, recordIndex + 1));
  const pendingBody = serializeJsonl(nextPending);

  // Append decision first (durable), then atomically replace pending.
  await appendDecision(dataDir, receipt);

  try {
    await writeFileAtomic(pendingPath, pendingBody);
  } catch (error) {
    // Fail closed for callers: pending was not updated. Decision line may exist;
    // reload + double-decide guard prevents a second successful decide.
    throw new Error(`Decision recorded but pending update failed: ${error.message}`);
  }

  return {
    receipt,
    pending: nextPending,
    priorDecisionsBytes,
  };
}

export async function seedPendingIfMissing(dataDir, records) {
  const { pendingPath, decisionsPath } = resolveAuditPaths(dataDir);
  await mkdir(dataDir, { recursive: true });
  try {
    await readFile(pendingPath, 'utf8');
  } catch (error) {
    if (error && error.code === 'ENOENT') {
      const validated = records.map((record, index) => validatePendingRecord(record, index + 1));
      await writeFileAtomic(pendingPath, serializeJsonl(validated));
    } else {
      throw error;
    }
  }
  try {
    await readFile(decisionsPath, 'utf8');
  } catch (error) {
    if (error && error.code === 'ENOENT') {
      await writeFile(decisionsPath, '', { mode: 0o600 });
    } else {
      throw error;
    }
  }
}

export function createSamplePending() {
  const now = Date.now();
  return [
    {
      requestId: 'req-pending-docs-read',
      status: 'PENDING',
      at: new Date(now - 60_000).toISOString(),
      principalId: 'agent-docs',
      action: 'documents.read',
      target: 'northstar://documents/demo',
      intentSummary: 'Read the demo document for operator review.',
      notScreened: true,
    },
    {
      requestId: 'req-pending-mail-draft',
      status: 'PENDING',
      at: new Date(now - 120_000).toISOString(),
      principalId: 'agent-mail',
      action: 'mail.draft',
      target: 'northstar://mail/drafts',
      intentSummary: 'Draft a reply summarizing the weekly status.',
    },
    {
      requestId: 'req-denied-export',
      status: 'DENIED',
      at: new Date(now - 3600_000).toISOString(),
      principalId: 'agent-export',
      action: 'documents.export',
      target: 'northstar://documents/secret',
      intentSummary: 'Export the secret document set off-host.',
    },
    {
      requestId: 'req-approved-list',
      status: 'APPROVED',
      at: new Date(now - 7200_000).toISOString(),
      principalId: 'agent-docs',
      action: 'documents.list',
      target: 'northstar://documents',
      intentSummary: 'List document titles in the shared folder.',
    },
    {
      requestId: 'req-error-unknown-action',
      status: 'ERROR',
      at: new Date(now - 1800_000).toISOString(),
      principalId: 'agent-ops',
      action: 'system.reboot',
      target: 'northstar://hosts/edge-1',
      intentSummary: 'Reboot edge host after maintenance window.',
      notScreened: true,
    },
  ];
}
