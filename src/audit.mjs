import fs from "node:fs";
import path from "node:path";

export function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

export function appendAudit(logPath, record) {
  ensureDir(path.dirname(logPath));
  fs.appendFileSync(logPath, `${JSON.stringify(record)}\n`, "utf8");
  return record;
}

export function readAudit(logPath) {
  if (!fs.existsSync(logPath)) return [];
  const text = fs.readFileSync(logPath, "utf8").trim();
  if (!text) return [];
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

export function chainForReceipt(records, receiptId) {
  const chain = records.filter(
    (r) =>
      r.receipt_id === receiptId ||
      r.linked_receipt_id === receiptId ||
      (r.original_intent && r.original_intent.intent_id && records.some(
        (x) => x.receipt_id === receiptId && x.original_intent?.intent_id === r.original_intent?.intent_id
      ))
  );

  // Prefer explicit linkage via receipt_id / linked_receipt_id
  const primary = records.find((r) => r.record_type === "glass.decision" && r.receipt_id === receiptId);
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
