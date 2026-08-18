/**
 * TL-PX 0.2 record validators. Distinct from 0.1 validate.mjs.
 * The 0.1 gate must keep using validate.mjs.
 */
import { validateSchemaFile } from "./schema.mjs";
import { validatePolicyBundleManifest } from "./policy-v02.mjs";

const DECISION_STATE = {
  ALLOW: "AUTHORIZED_UNCLAIMED",
  REQUIRE_APPROVAL: "PENDING_APPROVAL",
  DENY: "DENIED"
};

export function validateV02(kind, value) {
  const files = {
    "submitted-intent": "submitted-intent.schema.json",
    "authorized-action": "authorized-action.schema.json",
    "executed-action": "executed-action.schema.json",
    decision: "decision.schema.json",
    "evaluation-error": "evaluation-error.schema.json",
    "operator-action": "operator-action.schema.json",
    authorization: "authorization.schema.json",
    "authorization-claim": "authorization-claim.schema.json",
    execution: "execution.schema.json",
    "policy-bundle": "policy-bundle.schema.json"
  };
  const file = files[kind];
  if (!file) return { ok: false, errors: [`unknown 0.2 kind ${kind}`] };
  if (kind === "policy-bundle") return validatePolicyBundleManifest(value);
  const result = validateSchemaFile(file, value);
  if (!result.ok) return result;

  if (kind === "decision") {
    const expected = DECISION_STATE[value.decision];
    if (value.authorization_state !== expected) {
      result.ok = false;
      result.errors.push(
        `decision ${value.decision} must map to ${expected}, got ${value.authorization_state}`
      );
    }
    if (value.parties?.evaluator?.type !== "machine") {
      result.ok = false;
      result.errors.push("parties.evaluator.type must be machine");
    }
  }
  if (kind === "evaluation-error") {
    if ("authorization_id" in value || "authorization_state" in value) {
      result.ok = false;
      result.errors.push("evaluation_error must not carry authorization");
    }
    const hasRequester = "authenticated_requester" in value;
    const hasRequestId = "request_id" in value;
    if (hasRequester !== hasRequestId) {
      result.ok = false;
      result.errors.push(
        "evaluation_error idempotency scope requires authenticated_requester and request_id together"
      );
    }
    if (("intent_hash" in value || "retry_of_receipt_id" in value) && !hasRequester) {
      result.ok = false;
      result.errors.push(
        "evaluation_error intent/retry linkage requires authenticated idempotency scope"
      );
    }
  }
  if (kind === "submitted-intent") {
    for (const forbidden of [
      "derived_risk",
      "effective_risk",
      "authorization_id",
      "authorization_nonce",
      "action_binding_hash"
    ]) {
      if (forbidden in value) {
        result.ok = false;
        result.errors.push(`submitted intent must not include ${forbidden}`);
      }
    }
  }
  if (
    kind === "authorization-claim" &&
    value.action_binding_hash !== value.executed_action_hash
  ) {
    result.ok = false;
    result.errors.push("ACTION_MISMATCH: presented binding != stored action_binding_hash");
  }
  if (kind === "execution") {
    const started = Date.parse(value.started_at);
    const ended = Date.parse(value.ended_at);
    if (!Number.isFinite(started) || !Number.isFinite(ended) || ended < started) {
      result.ok = false;
      result.errors.push("execution timestamps must be valid and ended_at must not precede started_at");
    }
    const cancelled = value.state === "CANCELLED";
    const terminalCancellation = [
      "CANCELLED_BEFORE_SIDE_EFFECT",
      "CANCELLED_DURING_EXECUTION"
    ].includes(value.cancellation_outcome);
    if (cancelled !== terminalCancellation) {
      result.ok = false;
      result.errors.push(
        "CANCELLED requires a terminal cancellation outcome, and other states must not use one"
      );
    }
  }
  return result;
}
