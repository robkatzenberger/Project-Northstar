function buildReceipt({ intent, policy, decision, evaluatedAt, apexVersion }) {
  const receipt = {
    receipt_type: "apex-lite.receipt",
    apex_version: apexVersion,
    evaluated_at: evaluatedAt,
    receipt_id: buildReceiptId(intent, evaluatedAt),
    control_mode: "ALLOW_OR_ESCALATE",
    blocking: false,
    decision: decision.decision,
    reason: decision.reason,
    policy_id: decision.policy_id,
    reward_signal: decision.decision === "ALLOW" ? null : "TRANSPARENCY_REWARDED",
    original_intent: intent
  };

  return receipt;
}

function buildReceiptId(intent, evaluatedAt) {
  const intentId = sanitizeReceiptPart(intent.intent_id || "intent");
  const stamp = sanitizeReceiptPart(String(evaluatedAt || "local")).slice(0, 24) || "local";
  return `rcpt_${intentId}_${stamp}`;
}

function sanitizeReceiptPart(value) {
  return String(value)
    .replace(/[^A-Za-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .slice(0, 48) || "local";
}

module.exports = {
  buildReceipt
};
