const fs = require("node:fs");
const path = require("node:path");

function appendAuditLog(filePath, receipt) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.appendFileSync(filePath, JSON.stringify(receipt) + "\n", "utf8");
}

function readAuditLog(filePath) {
  if (!fs.existsSync(filePath)) {
    return [];
  }

  return fs
    .readFileSync(filePath, "utf8")
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

function findReceiptById(filePath, receiptId) {
  return findReceiptByIdInEntries(readAuditLog(filePath), receiptId);
}

function findOperatorActionByReceiptId(filePath, receiptId) {
  return findOperatorActionByReceiptIdInEntries(readAuditLog(filePath), receiptId);
}

function findReceiptByIdInEntries(entries, receiptId) {
  return entries.find((entry) => {
    return entry.receipt_type === "apex-lite.receipt" && entry.receipt_id === receiptId;
  }) || null;
}

function findOperatorActionByReceiptIdInEntries(entries, receiptId) {
  return entries.find((entry) => {
    return entry.receipt_type === "apex-lite.operator_action" && entry.receipt_id === receiptId;
  }) || null;
}

module.exports = {
  appendAuditLog,
  findOperatorActionByReceiptId,
  findOperatorActionByReceiptIdInEntries,
  findReceiptById,
  findReceiptByIdInEntries,
  readAuditLog
};
