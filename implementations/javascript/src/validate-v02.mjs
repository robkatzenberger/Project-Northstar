/**
 * TL-PX 0.2 record validators. Distinct from 0.1 validate.mjs.
 * The 0.1 gate must keep using validate.mjs.
 */
import { validateSchemaFile } from "./schema.mjs";

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
    execution: "execution.schema.json"
  };
  const file = files[kind];
  if (!file) return { ok: false, errors: [`unknown 0.2 kind ${kind}`] };
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
  }
  if (kind === "submitted-intent") {
    for (const forbidden of [
      "derived_risk",
      "effective_risk",
      "authorization_id",
      "authorization_nonce"
    ]) {
      if (forbidden in value) {
        result.ok = false;
        result.errors.push(`submitted intent must not include ${forbidden}`);
      }
    }
  }
  return result;
}
