/**
 * Append-only JSONL audit with hash-chain integrity + HMAC seal (A9).
 *
 * Each line:
 *   { ...record fields, prev_hash, audit_hash, seal }
 *
 * - prev_hash: sha256 of previous line's audit_hash (or genesis)
 * - audit_hash: sha256 of canonical JSON (excluding audit_hash + seal)
 * - seal: HMAC-SHA256(audit_hash, seal key) — stops raw forgery without the key
 *
 * Seal key: env TLPX_AUDIT_SEAL, or file next to the log (.<basename>.seal),
 * auto-created on first write (mode 0600).
 */

import fs from "node:fs";
import path from "node:path";
import { createHash, createHmac, randomBytes } from "node:crypto";

export const GENESIS_HASH = "0".repeat(64);

export function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function sha256Hex(s) {
  return createHash("sha256").update(s, "utf8").digest("hex");
}

function hmacHex(key, data) {
  return createHmac("sha256", key).update(data, "utf8").digest("hex");
}

/** Deterministic JSON for hashing (sorted keys; drops integrity fields). */
export function canonicalJson(value) {
  if (value === null || typeof value !== "object") {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((v) => canonicalJson(v)).join(",")}]`;
  }
  const keys = Object.keys(value)
    .filter((k) => k !== "audit_hash" && k !== "seal")
    .sort();
  return `{${keys.map((k) => `${JSON.stringify(k)}:${canonicalJson(value[k])}`).join(",")}}`;
}

export function sealKeyPathForLog(logPath) {
  const dir = path.dirname(path.resolve(logPath));
  const base = path.basename(logPath);
  return path.join(dir, `.${base}.seal`);
}

/**
 * Resolve the HMAC seal key for a log file.
 * @param {{ sealKey?: string|Buffer, sealKeyPath?: string, createIfMissing?: boolean }} opts
 */
export function resolveSealKey(logPath, opts = {}) {
  if (opts.sealKey) {
    return Buffer.isBuffer(opts.sealKey)
      ? opts.sealKey
      : Buffer.from(String(opts.sealKey), "utf8");
  }
  if (process.env.TLPX_AUDIT_SEAL) {
    return Buffer.from(process.env.TLPX_AUDIT_SEAL, "utf8");
  }
  const keyPath = opts.sealKeyPath || sealKeyPathForLog(logPath);
  if (fs.existsSync(keyPath)) {
    return fs.readFileSync(keyPath);
  }
  if (opts.createIfMissing === false) {
    throw new Error(`Missing seal key file: ${keyPath}`);
  }
  ensureDir(path.dirname(keyPath));
  const key = randomBytes(32);
  fs.writeFileSync(keyPath, key, { mode: 0o600 });
  try {
    fs.chmodSync(keyPath, 0o600);
  } catch {
    /* best effort */
  }
  return key;
}

function lastAuditHash(logPath) {
  if (!fs.existsSync(logPath)) return GENESIS_HASH;
  const text = fs.readFileSync(logPath, "utf8").trim();
  if (!text) return GENESIS_HASH;
  const lines = text.split(/\r?\n/).filter(Boolean);
  const last = JSON.parse(lines[lines.length - 1]);
  return last.audit_hash || GENESIS_HASH;
}

/**
 * Seal a record (does not write). Used by append and tests.
 */
export function sealRecord(record, prevHash, sealKey) {
  const withPrev = { ...record, prev_hash: prevHash };
  const audit_hash = sha256Hex(canonicalJson(withPrev));
  const seal = hmacHex(sealKey, audit_hash);
  return { ...withPrev, audit_hash, seal };
}

export function appendAudit(logPath, record, opts = {}) {
  ensureDir(path.dirname(logPath));
  const sealKey = resolveSealKey(logPath, { ...opts, createIfMissing: true });
  const prev = lastAuditHash(logPath);
  const sealed = sealRecord(record, prev, sealKey);
  fs.appendFileSync(logPath, `${JSON.stringify(sealed)}\n`, "utf8");
  return sealed;
}

/**
 * Verify hash-chain + seals from genesis.
 * @returns {{ ok: boolean, lines: number, errors: string[] }}
 */
export function verifyAudit(logPath, opts = {}) {
  const errors = [];
  if (!fs.existsSync(logPath)) {
    return { ok: true, lines: 0, errors: [], empty: true };
  }
  const text = fs.readFileSync(logPath, "utf8").trim();
  if (!text) return { ok: true, lines: 0, errors: [], empty: true };

  let sealKey;
  try {
    sealKey = resolveSealKey(logPath, { ...opts, createIfMissing: false });
  } catch (e) {
    return { ok: false, lines: 0, errors: [e.message] };
  }

  const lines = text.split(/\r?\n/).filter(Boolean);
  let expectedPrev = GENESIS_HASH;

  for (let i = 0; i < lines.length; i++) {
    const lineNo = i + 1;
    let rec;
    try {
      rec = JSON.parse(lines[i]);
    } catch {
      errors.push(`line ${lineNo}: corrupt JSON`);
      break;
    }

    if (!rec.audit_hash || !rec.seal || !rec.prev_hash) {
      errors.push(
        `line ${lineNo}: missing integrity fields (prev_hash/audit_hash/seal) — unsealed or forged`
      );
      continue;
    }

    if (rec.prev_hash !== expectedPrev) {
      errors.push(
        `line ${lineNo}: prev_hash mismatch (chain broken or history rewritten)`
      );
    }

    const body = { ...rec };
    const claimedHash = body.audit_hash;
    const claimedSeal = body.seal;
    delete body.audit_hash;
    delete body.seal;
    // body still has prev_hash
    const recomputed = sha256Hex(canonicalJson(body));
    if (recomputed !== claimedHash) {
      errors.push(`line ${lineNo}: audit_hash mismatch (payload tampered)`);
    }

    const expectedSeal = hmacHex(sealKey, claimedHash);
    if (expectedSeal !== claimedSeal) {
      errors.push(`line ${lineNo}: seal mismatch (forged or wrong seal key)`);
    }

    expectedPrev = claimedHash;
  }

  return { ok: errors.length === 0, lines: lines.length, errors };
}

/**
 * Read audit records. By default verifies integrity first (fail-closed).
 * Pass { skipVerify: true } only for recovery/debug.
 */
export function readAudit(logPath, opts = {}) {
  if (!fs.existsSync(logPath)) return [];
  const text = fs.readFileSync(logPath, "utf8").trim();
  if (!text) return [];

  if (!opts.skipVerify) {
    const v = verifyAudit(logPath, opts);
    if (!v.ok) {
      throw new Error(
        `Audit integrity check failed (${logPath}): ${v.errors.join("; ")}`
      );
    }
  }

  return text.split(/\r?\n/).filter(Boolean).map((line, i) => {
    try {
      return JSON.parse(line);
    } catch {
      throw new Error(`Corrupt audit line ${i + 1} in ${logPath}`);
    }
  });
}

export function findByReceiptId(records, receiptId) {
  return records.filter((r) => r.receipt_id === receiptId || r.linked_receipt_id === receiptId);
}

function isDecisionType(t) {
  return t === "tlpx.decision" || t === "glass.decision";
}

export function chainForReceipt(records, receiptId) {
  const primary = records.find((r) => isDecisionType(r.record_type) && r.receipt_id === receiptId);
  if (!primary) {
    return records.filter((r) => r.receipt_id === receiptId || r.linked_receipt_id === receiptId);
  }

  const intentId = primary.original_intent?.intent_id;
  return records.filter((r) => {
    if (r.receipt_id === receiptId || r.linked_receipt_id === receiptId) return true;
    if (intentId && r.original_intent?.intent_id === intentId) return true;
    if (intentId && r.intent_id === intentId) return true;
    return false;
  });
}
