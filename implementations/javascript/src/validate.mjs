/**
 * Lightweight structural validators for TL-PX 0.1 Minimum Profile.
 * Intentionally dependency-free (no Ajv) so the standard ships ready to run.
 */

import { CONTROL_MODE, STANDARD_ID, STANDARD_VERSION, isRecordType } from "./standard.mjs";

function fail(errors, msg) {
  errors.push(msg);
}

function requireParty(errors, party, path, { allowedTypes } = {}) {
  if (!party || typeof party !== "object") {
    fail(errors, `${path} must be an object`);
    return;
  }
  if (!party.id || typeof party.id !== "string") fail(errors, `${path}.id required`);
  if (!party.type || typeof party.type !== "string") fail(errors, `${path}.type required`);
  else if (allowedTypes && !allowedTypes.includes(party.type)) {
    fail(errors, `${path}.type must be one of ${allowedTypes.join("|")}`);
  }
}

export function validateEvaluationRequest(req) {
  const errors = [];
  if (!req || typeof req !== "object") return { ok: false, errors: ["request must be object"] };
  if (!req.intent_id) fail(errors, "intent_id required");
  if (!req.actor) fail(errors, "actor required");
  if (!req.actor_type || !["human", "machine"].includes(req.actor_type)) {
    fail(errors, 'actor_type must be "human" or "machine"');
  }
  if (!req.declared_intent && !req.intent_summary) {
    fail(errors, "declared_intent (or intent_summary) required");
  }
  if (req.risk && !["low", "medium", "high"].includes(req.risk)) {
    fail(errors, "risk must be low|medium|high");
  }
  if (req.data_classes && !Array.isArray(req.data_classes)) {
    fail(errors, "data_classes must be array");
  }
  return { ok: errors.length === 0, errors };
}

export function validateDecisionRecord(rec) {
  const errors = [];
  if (!isRecordType(rec?.record_type, "decision")) {
    fail(errors, "record_type must be tlpx.decision or glass.decision");
  }
  if (rec.standard !== STANDARD_ID) fail(errors, `standard must be ${STANDARD_ID}`);
  if (rec.standard_version !== STANDARD_VERSION) {
    fail(errors, `standard_version must be ${STANDARD_VERSION}`);
  }
  if (!rec.receipt_id) fail(errors, "receipt_id required");
  if (!rec.evaluated_at) fail(errors, "evaluated_at required");
  if (rec.control_mode !== CONTROL_MODE) fail(errors, `control_mode must be ${CONTROL_MODE}`);
  if (!["ALLOW", "REQUIRE_APPROVAL"].includes(rec.decision)) {
    fail(errors, "decision must be ALLOW|REQUIRE_APPROVAL");
  }
  if (typeof rec.reason !== "string") fail(errors, "reason required");
  if (!("policy_id" in (rec || {}))) fail(errors, "policy_id required (string or null)");
  if (
    !["AUTHORIZED", "PENDING_HUMAN_APPROVAL", "DENIED"].includes(rec.authorization_status)
  ) {
    fail(errors, "authorization_status invalid");
  }
  requireParty(errors, rec.parties?.declarer, "parties.declarer", {
    allowedTypes: ["human", "machine"]
  });
  requireParty(errors, rec.parties?.evaluator, "parties.evaluator", {
    allowedTypes: ["machine"]
  });
  if (!rec.original_intent || typeof rec.original_intent !== "object") {
    fail(errors, "original_intent required");
  }

  // Semantic mapping checks
  if (rec.decision === "ALLOW" && rec.authorization_status !== "AUTHORIZED") {
    fail(errors, "ALLOW must map to AUTHORIZED");
  }
  if (
    rec.decision === "REQUIRE_APPROVAL" &&
    rec.authorization_status === "AUTHORIZED" &&
    !rec.parties?.authorizer
  ) {
    // unresolved REQUIRE_APPROVAL must be pending
    fail(errors, "unresolved REQUIRE_APPROVAL must not be AUTHORIZED without authorizer");
  }
  if (
    rec.decision === "REQUIRE_APPROVAL" &&
    !["PENDING_HUMAN_APPROVAL", "AUTHORIZED", "DENIED"].includes(rec.authorization_status)
  ) {
    fail(errors, "REQUIRE_APPROVAL has invalid authorization_status");
  }

  return { ok: errors.length === 0, errors };
}

export function validateOperatorAction(rec) {
  const errors = [];
  if (!isRecordType(rec?.record_type, "operator_action")) {
    fail(errors, "record_type must be tlpx.operator_action or glass.operator_action");
  }
  if (rec.standard !== STANDARD_ID) fail(errors, `standard must be ${STANDARD_ID}`);
  if (rec.standard_version !== STANDARD_VERSION) {
    fail(errors, `standard_version must be ${STANDARD_VERSION}`);
  }
  if (!rec.receipt_id && !rec.linked_receipt_id) {
    fail(errors, "receipt_id or linked_receipt_id required");
  }
  if (!rec.acted_at) fail(errors, "acted_at required");
  requireParty(errors, rec.operator, "operator", { allowedTypes: ["human"] });
  if (!["APPROVE", "REJECT"].includes(rec.outcome)) fail(errors, "outcome must be APPROVE|REJECT");
  if (rec.outcome === "APPROVE" && rec.authorization_status !== "AUTHORIZED") {
    fail(errors, "APPROVE must yield AUTHORIZED");
  }
  if (rec.outcome === "REJECT" && rec.authorization_status !== "DENIED") {
    fail(errors, "REJECT must yield DENIED");
  }
  return { ok: errors.length === 0, errors };
}

export function validateExecutionRecord(rec) {
  const errors = [];
  if (!isRecordType(rec?.record_type, "execution")) {
    fail(errors, "record_type must be tlpx.execution or glass.execution");
  }
  if (rec.standard !== STANDARD_ID) fail(errors, `standard must be ${STANDARD_ID}`);
  if (rec.standard_version !== STANDARD_VERSION) {
    fail(errors, `standard_version must be ${STANDARD_VERSION}`);
  }
  if (!rec.receipt_id && !rec.linked_receipt_id) {
    fail(errors, "receipt_id or linked_receipt_id required");
  }
  if (!rec.executed_at) fail(errors, "executed_at required");
  if (!["EXECUTED", "BLOCKED", "FAILED"].includes(rec.status)) {
    fail(errors, "status must be EXECUTED|BLOCKED|FAILED");
  }
  requireParty(errors, rec.executor, "executor", { allowedTypes: ["human", "machine"] });
  if (
    !["AUTHORIZED", "PENDING_HUMAN_APPROVAL", "DENIED"].includes(rec.authorization_status)
  ) {
    fail(errors, "authorization_status invalid");
  }
  if (rec.status === "EXECUTED" && rec.authorization_status !== "AUTHORIZED") {
    fail(errors, "EXECUTED requires authorization_status AUTHORIZED");
  }
  return { ok: errors.length === 0, errors };
}

export function validateAccountabilityReport(rec) {
  const errors = [];
  if (!isRecordType(rec?.record_type, "accountability_report")) {
    fail(errors, "record_type must be accountability report type");
  }
  if (!rec.parties_involved || typeof rec.parties_involved !== "object") {
    fail(errors, "parties_involved required");
  } else {
    if (!Array.isArray(rec.parties_involved.human)) fail(errors, "parties_involved.human array");
    if (!Array.isArray(rec.parties_involved.machine)) fail(errors, "parties_involved.machine array");
  }
  if (!Array.isArray(rec.findings)) fail(errors, "findings array required");
  return { ok: errors.length === 0, errors };
}
