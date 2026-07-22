import { randomBytes, randomUUID } from "node:crypto";

export function uuid() {
  return randomUUID();
}

/**
 * Receipt ids include time + short entropy so concurrent evaluates never collide.
 */
export function receiptId(intentId, at = new Date()) {
  const stamp = at.toISOString().replace(/[-:]/g, "").replace(/\.\d{3}Z$/, "Z");
  const safe = String(intentId || "unknown").replace(/[^A-Za-z0-9_-]/g, "_").slice(0, 40);
  const entropy = randomBytes(3).toString("hex");
  return `rcpt_${safe}_${stamp}_${entropy}`;
}

export function nowIso() {
  return new Date().toISOString();
}
