import { randomUUID } from "node:crypto";

export function uuid() {
  return randomUUID();
}

export function receiptId(intentId, at = new Date()) {
  const stamp = at.toISOString().replace(/[-:]/g, "").replace(/\.\d{3}Z$/, "Z");
  const safe = String(intentId || "unknown").replace(/[^A-Za-z0-9_-]/g, "_").slice(0, 40);
  return `rcpt_${safe}_${stamp}`;
}

export function nowIso() {
  return new Date().toISOString();
}
