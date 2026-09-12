const path = require("node:path");
const {
  appendAuditLog,
  findOperatorActionByReceiptIdInEntries,
  findReceiptByIdInEntries,
  readAuditLog
} = require("./audit");

const { version } = require(path.join(__dirname, "..", "package.json"));

function recordOperatorAction(payload, auditPath) {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    throw new Error("Request body must be a JSON object.");
  }

  const action = normalizeAction(payload.action);
  const receipt = normalizeReceipt(payload.receipt);
  const auditEntries = readAuditLog(auditPath);
  const recordedReceipt = verifyRecordedReceipt(receipt, auditEntries);
  const operator = normalizeOperator(payload.operator);
  const entry = {
    receipt_type: "apex-lite.operator_action",
    apex_version: version,
    action_at: new Date().toISOString(),
    action,
    operator,
    receipt_id: recordedReceipt.receipt_id,
    intent_id: recordedReceipt.original_intent.intent_id || null,
    policy_id: recordedReceipt.policy_id || null,
    original_decision: recordedReceipt.decision,
    original_reason: recordedReceipt.reason,
    outcome: action === "APPROVE" ? "ALLOW" : "ESCALATED_TO_HUMAN",
    reward_signal: action === "ESCALATE" ? "TRANSPARENCY_REWARDED" : "ESCALATION_RESOLVED"
  };

  appendAuditLog(auditPath, entry);
  return entry;
}

function normalizeAction(value) {
  if (value === "approve") {
    return "APPROVE";
  }

  if (value === "escalate") {
    return "ESCALATE";
  }

  throw new Error("Action must be 'approve' or 'escalate'.");
}

function normalizeReceipt(receipt) {
  if (!receipt || typeof receipt !== "object" || Array.isArray(receipt)) {
    throw new Error("A receipt is required for operator actions.");
  }

  if (!receipt.receipt_id) {
    throw new Error("Operator actions require a valid evaluation receipt.");
  }

  if (!receipt.original_intent || typeof receipt.original_intent !== "object") {
    throw new Error("Operator actions require the original intent.");
  }

  return receipt;
}

function normalizeOperator(value) {
  const operator = typeof value === "string" ? value.trim() : "";

  if (!operator) {
    return "local_operator";
  }

  if (!/^[A-Za-z0-9_.:-]{1,64}$/.test(operator)) {
    throw new Error("Operator must use 1-64 characters from A-Z, a-z, 0-9, ., _, :, or -.");
  }

  return operator;
}

function verifyRecordedReceipt(receipt, auditEntries) {
  const recordedReceipt = findReceiptByIdInEntries(auditEntries, receipt.receipt_id);

  if (!recordedReceipt) {
    throw new Error("Operator actions require a recorded evaluation receipt.");
  }

  if (recordedReceipt.decision !== "REQUIRE_APPROVAL") {
    throw new Error("Operator actions are only valid for escalated evaluations.");
  }

  if ((recordedReceipt.original_intent || {}).intent_id !== (receipt.original_intent || {}).intent_id) {
    throw new Error("Operator action receipt does not match the recorded evaluation.");
  }

  if (recordedReceipt.evaluated_at !== receipt.evaluated_at) {
    throw new Error("Operator action receipt does not match the recorded evaluation.");
  }

  if (findOperatorActionByReceiptIdInEntries(auditEntries, receipt.receipt_id)) {
    throw new Error("This escalated evaluation already has a recorded operator outcome.");
  }

  return recordedReceipt;
}

module.exports = {
  recordOperatorAction
};
