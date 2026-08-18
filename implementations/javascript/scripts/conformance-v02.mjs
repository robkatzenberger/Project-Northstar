/**
 * TL-PX 0.2 record/schema conformance. Distinct from the frozen 0.1 suite.
 */
import { validateV02 } from "../src/validate-v02.mjs";
import { loadTlpx02Schemas } from "../src/schema.mjs";
import {
  POLICY_PRECEDENCE,
  validatedPolicyBundleHash,
  resolvePolicyDecision,
  selectActivePolicyBundle,
  verifyCompleteAuthoritySequence
} from "../src/policy-v02.mjs";
import {
  actionBindingsMatchV02,
  authorizedActionBindingHashV02,
  authorizedActionHashV02,
  executedActionHashV02,
  submittedIntentHashV02
} from "../src/action-v02.mjs";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const H = `sha256:${"a".repeat(64)}`;
const H2 = `sha256:${"b".repeat(64)}`;
const T = "2026-08-14T12:00:00.000Z";
const adapter = { id: "adapter.mailer", version: "1.0.0" };

loadTlpx02Schemas();

let passed = 0;
let failed = 0;
function check(name, cond, detail = "") {
  if (cond) {
    passed += 1;
    console.log(`  PASS  ${name}`);
  } else {
    failed += 1;
    console.log(`  FAIL  ${name}${detail ? ` — ${detail}` : ""}`);
  }
}

const intent = {
  requesting_principal: "agent.a",
  executing_principal: "agent.a",
  action: "send_email",
  intent_class: "external_communication",
  target: "customer:123",
  arguments: { template: "invoice-ready" },
  environment: "production",
  tenant: "tenant_abc",
  declared_risk: "medium",
  data_classes: ["PII"],
  requested_capability: "mailer.send",
  resource_scope: ["customer:123"],
  payload_hash: null,
  artifact_hash: null,
  adapter,
  request_id: "req-1"
};

const authorized = {
  requesting_principal: "agent.a",
  executing_principal: "agent.a",
  action: "send_email",
  target: "customer:123",
  arguments: { template: "invoice-ready" },
  environment: "production",
  tenant: "tenant_abc",
  derived_risk: "high",
  effective_risk: "high",
  risk_reasons: ["policy.high_data_class"],
  risk_source: "policy",
  data_classes: ["PII"],
  capability: "mailer.send",
  resource_scope: ["customer:123"],
  payload_hash: null,
  artifact_hash: null,
  policy_bundle_hash: H,
  adapter
};

const executed = {
  executing_principal: "agent.a",
  action: "send_email",
  target: "customer:123",
  arguments: { template: "invoice-ready" },
  environment: "production",
  tenant: "tenant_abc",
  payload_hash: null,
  artifact_hash: null,
  adapter
};

function decision(over = {}) {
  return {
    record_type: "tlpx.decision",
    standard: "TL-PX",
    standard_version: "0.2.0",
    control_mode: "ALLOW_ESCALATE_OR_DENY",
    receipt_id: "rcpt_1",
    request_id: "req-1",
    evaluated_at: T,
    decision: "ALLOW",
    authorization_state: "AUTHORIZED_UNCLAIMED",
    reason: "ok",
    reason_code: "POLICY_ALLOW",
    policy_id: "rule_ok",
    policy_bundle_id: "pack",
    policy_bundle_version: "1.0.0",
    policy_bundle_hash: H,
    intent_hash: H,
    parties: {
      requester: { id: "agent.a", type: "machine" },
      evaluator: { id: "tlpx-authority", type: "machine" }
    },
    sequence: 1,
    ...over
  };
}

console.log("TL-PX 0.2.0 record/schema conformance\n");

{
  console.log("C1  Accept valid 0.2 objects");
  check("submitted-intent", validateV02("submitted-intent", intent).ok);
  check("authorized-action", validateV02("authorized-action", authorized).ok);
  check("executed-action", validateV02("executed-action", executed).ok);
  check("ALLOW maps to AUTHORIZED_UNCLAIMED", validateV02("decision", decision()).ok);
  check(
    "DENY is a first-class decision",
    validateV02("decision", decision({ decision: "DENY", authorization_state: "DENIED", reason_code: "POLICY_DENY" })).ok
  );
  check(
    "REQUIRE_APPROVAL pending",
    validateV02(
      "decision",
      decision({
        decision: "REQUIRE_APPROVAL",
        authorization_state: "PENDING_APPROVAL",
        reason_code: "POLICY_REQUIRE_APPROVAL"
      })
    ).ok
  );
  const err = {
    record_type: "tlpx.evaluation_error",
    standard: "TL-PX",
    standard_version: "0.2.0",
    receipt_id: "rcpt_e",
    occurred_at: T,
    stage: "compile",
    error_code: "POLICY_COMPILE_FAILED",
    retryability: "NEVER",
    reason: "bad pack",
    sequence: 2
  };
  check("evaluation_error", validateV02("evaluation-error", err).ok);
  check(
    "scoped evaluation_error",
    validateV02("evaluation-error", {
      ...err,
      receipt_id: "rcpt_scoped",
      authenticated_requester: "agent.a",
      request_id: "req-error",
      intent_hash: H
    }).ok
  );
}

{
  console.log("C2  Reject 0.1 shapes and mixes");
  const v01 = {
    record_type: "glass.decision",
    standard: "TL-PX",
    standard_version: "0.1.0",
    control_mode: "ALLOW_OR_ESCALATE",
    receipt_id: "x",
    evaluated_at: T,
    decision: "ALLOW",
    reason: "ok",
    policy_id: null,
    authorization_status: "AUTHORIZED",
    parties: { declarer: { id: "a", type: "machine" }, evaluator: { id: "e", type: "machine" } },
    original_intent: {}
  };
  check("0.1 decision fails 0.2 schema", !validateV02("decision", v01).ok);
  check(
    "ALLOW + AUTHORIZED (0.1 mapping) fails",
    !validateV02("decision", decision({ authorization_state: "AUTHORIZED" })).ok
  );
  check(
    "evaluation_error is not a decision",
    !validateV02("decision", decision({ record_type: "tlpx.evaluation_error" })).ok
  );
}

{
  console.log("C3  Distinct objects and forbidden fields");
  check(
    "intent cannot carry derived_risk",
    !validateV02("submitted-intent", { ...intent, derived_risk: "low" }).ok
  );
  check(
    "intent cannot carry authorization_nonce",
    !validateV02("submitted-intent", { ...intent, authorization_nonce: "n" }).ok
  );
  check(
    "intent cannot carry action_binding_hash",
    !validateV02("submitted-intent", { ...intent, action_binding_hash: H }).ok
  );
  check("authorized-action rejects request_id", !validateV02("authorized-action", { ...authorized, request_id: "x" }).ok);
  check(
    "retry linkage accepted on successor intent",
    validateV02("submitted-intent", { ...intent, request_id: "req-2", retry_of_receipt_id: "rcpt_e" }).ok
  );
  check(
    "retry linkage accepted on successor decision",
    validateV02("decision", decision({ request_id: "req-2", retry_of_receipt_id: "rcpt_e" })).ok
  );
  check(
    "effective risk below derived risk rejected",
    !validateV02("authorized-action", {
      ...authorized,
      derived_risk: "high",
      effective_risk: "low"
    }).ok
  );
  check("intent hash is schema-bound", /^sha256:[0-9a-f]{64}$/.test(submittedIntentHashV02(intent)));
  check(
    "authorized full hash differs from Action Binding hash",
    authorizedActionHashV02(authorized) !== authorizedActionBindingHashV02(authorized)
  );
  check(
    "matching executed action uses the Action Binding hash",
    actionBindingsMatchV02(authorized, executed) &&
      authorizedActionBindingHashV02(authorized) === executedActionHashV02(executed)
  );
  check(
    "binding mutation is rejected",
    !actionBindingsMatchV02(authorized, { ...executed, target: "customer:999" })
  );
}

{
  console.log("C4  Evaluation-error and operator invariants");
  const baseError = {
    record_type: "tlpx.evaluation_error",
    standard: "TL-PX",
    standard_version: "0.2.0",
    receipt_id: "rcpt_error",
    occurred_at: T,
    stage: "authentication",
    error_code: "AUTHENTICATION_FAILED",
    retryability: "AFTER_CONDITION",
    required_condition: "authenticated requester matches proposed requester",
    reason: "identity mismatch",
    sequence: 5,
    authenticated_requester: "agent.a",
    request_id: "req-error"
  };
  check("AFTER_CONDITION requires condition", validateV02("evaluation-error", baseError).ok);
  const { required_condition: _condition, ...missingCondition } = baseError;
  check(
    "AFTER_CONDITION without condition rejected",
    !validateV02("evaluation-error", missingCondition).ok
  );
  check(
    "NEVER with required_condition rejected",
    !validateV02("evaluation-error", { ...baseError, retryability: "NEVER" }).ok
  );
  const { request_id: _requestId, ...halfScoped } = baseError;
  check(
    "half-scoped evaluation_error rejected",
    !validateV02("evaluation-error", halfScoped).ok
  );
  const { authenticated_requester: _requesterOnly, ...requestIdOnly } = baseError;
  check(
    "request_id-only evaluation_error rejected",
    !validateV02("evaluation-error", requestIdOnly).ok
  );
  const {
    authenticated_requester: _authenticatedRequester,
    request_id: _scopedRequestId,
    ...unscopedError
  } = missingCondition;
  check(
    "retry link requires authenticated scope",
    !validateV02("evaluation-error", {
      ...unscopedError,
      retryability: "NEVER",
      retry_of_receipt_id: "rcpt_prior"
    }).ok
  );
  check(
    "intent hash requires authenticated scope",
    !validateV02("evaluation-error", {
      ...unscopedError,
      retryability: "NEVER",
      intent_hash: H
    }).ok
  );

  const operator = {
    record_type: "tlpx.operator_action",
    standard: "TL-PX",
    standard_version: "0.2.0",
    receipt_id: "rcpt_pending",
    acted_at: T,
    outcome: "APPROVE",
    operator: { id: "human.ops.alex", type: "human" },
    authorized_action_hash: H,
    policy_bundle_hash: H2,
    approval_route: ["human.ops.alex"],
    renderer_id: "glass.approval.v1",
    renderer_version: "1.0.0",
    sequence: 6
  };
  check("APPROVE carries rendered action context", validateV02("operator-action", operator).ok);
  check(
    "APPROVE requires a human authorizer",
    !validateV02("operator-action", {
      ...operator,
      operator: { id: "agent.approver", type: "machine" }
    }).ok
  );
  check(
    "REJECT requires a human authorizer",
    !validateV02("operator-action", {
      ...operator,
      outcome: "REJECT",
      operator: { id: "agent.rejector", type: "machine" }
    }).ok
  );
  for (const field of [
    "authorized_action_hash",
    "approval_route",
    "renderer_id",
    "renderer_version"
  ]) {
    const incomplete = { ...operator };
    delete incomplete[field];
    check(`APPROVE without ${field} rejected`, !validateV02("operator-action", incomplete).ok);
  }
  check(
    "CANCEL is valid without renderer context",
    validateV02("operator-action", {
      record_type: "tlpx.operator_action",
      standard: "TL-PX",
      standard_version: "0.2.0",
      receipt_id: "rcpt_pending",
      acted_at: T,
      outcome: "CANCEL",
      operator: { id: "agent.a", type: "machine" },
      policy_bundle_hash: H2,
      sequence: 7
    }).ok
  );
}

{
  console.log("C5  Claim and execution");
  const authz = {
    record_type: "tlpx.authorization",
    standard: "TL-PX",
    standard_version: "0.2.0",
    authorization_id: "authz_1",
    receipt_id: "rcpt_1",
    requesting_principal: "agent.a",
    executing_principal: "agent.a",
    action: "send_email",
    target: "customer:123",
    authorized_action_hash: H,
    action_binding_hash: H2,
    intent_hash: H,
    environment: "production",
    tenant: "tenant_abc",
    adapter,
    issued_at: T,
    claim_expires_at: T,
    execution_lease_seconds: 30,
    authorization_nonce: "nonce-authority",
    idempotency_key: "idemp-1",
    state: "AUTHORIZED_UNCLAIMED"
  };
  check("authorization", validateV02("authorization", authz).ok);
  const claim = {
    record_type: "tlpx.authorization_claim",
    standard: "TL-PX",
    standard_version: "0.2.0",
    claim_id: "claim_1",
    authorization_id: "authz_1",
    receipt_id: "rcpt_1",
    executing_principal: "agent.a",
    authorized_action_hash: H,
    action_binding_hash: H2,
    executed_action_hash: H2,
    adapter,
    claimed_at: T,
    lease_expires_at: T,
    sequence: 3,
    state: "CLAIMED"
  };
  check("authorization_claim", validateV02("authorization-claim", claim).ok);
  check(
    "claim ACTION_MISMATCH when presented binding differs",
    !validateV02("authorization-claim", { ...claim, executed_action_hash: H }).ok
  );
  const execOk = {
    record_type: "tlpx.execution",
    standard: "TL-PX",
    standard_version: "0.2.0",
    execution_id: "ex1",
    claim_id: "claim_1",
    authorization_id: "authz_1",
    receipt_id: "rcpt_1",
    requesting_principal: "agent.a",
    executing_principal: "agent.a",
    intent_hash: H,
    authorized_action_hash: H,
    executed_action_hash: H,
    target: "customer:123",
    policy_bundle_id: "pack",
    policy_bundle_version: "1.0.0",
    policy_bundle_hash: H,
    adapter,
    sequence: 4,
    started_at: T,
    ended_at: T,
    state: "COMPLETED"
  };
  check("execution receipt", validateV02("execution", execOk).ok);
  check(
    "receipt may store distinct authorized vs executed hashes",
    validateV02("execution", { ...execOk, executed_action_hash: H2 }).ok
  );
  check("bad hash string", !validateV02("authorization", { ...authz, intent_hash: "SHA256:" + "A".repeat(64) }).ok);
}

{
  console.log("C6  Reason-code catalog present");
  const catalog = JSON.parse(
    fs.readFileSync(
      path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../schemas/tlpx-0.2/reason-codes.json"),
      "utf8"
    )
  );
  check("catalog version 0.2.0", catalog.standard_version === "0.2.0");
  check("includes POLICY_DENY", catalog.evaluation.includes("POLICY_DENY"));
  check("includes policy scope refusal", catalog.evaluation.includes("POLICY_TARGET_OUT_OF_SCOPE"));
  check("includes inactive principal", catalog.evaluation.includes("SWITCHBOARD_PRINCIPAL_INACTIVE"));
  check("includes IDEMPOTENCY_CONFLICT", catalog.evaluation_error.includes("IDEMPOTENCY_CONFLICT"));
  check("includes ACTION_MISMATCH", catalog.claim_and_execution.includes("ACTION_MISMATCH"));
  check("includes POLICY_INACTIVE", catalog.claim_and_execution.includes("POLICY_INACTIVE"));
  check("includes POLICY_PROVENANCE_INVALID", catalog.evaluation_error.includes("POLICY_PROVENANCE_INVALID"));
  check("includes POLICY_PRECEDENCE_AMBIGUOUS", catalog.evaluation_error.includes("POLICY_PRECEDENCE_AMBIGUOUS"));
  check("includes TRUSTED_SEQUENCE_INVALID", catalog.evaluation_error.includes("TRUSTED_SEQUENCE_INVALID"));
  check(
    "includes CANCELLATION_UNAUTHORIZED",
    catalog.operator_action.includes("CANCELLATION_UNAUTHORIZED")
  );
  check("includes APPROVAL_TERMINAL", catalog.operator_action.includes("APPROVAL_TERMINAL"));
}

{
  console.log("C7  Slice 2.4 policy provenance, precedence, and ordering");
  const bundle = {
    manifest_type: "tlpx.policy_bundle",
    standard: "TL-PX",
    standard_version: "0.2.0",
    policy_bundle_id: "pack",
    policy_bundle_version: "1.0.0",
    issuer: { id: "security.platform", type: "human" },
    content_type: "application/vnd.tlpx.policy+json;version=1",
    content_hash: H,
    activated_at: T,
    retired_at: null,
    environment: "production",
    tenant: "tenant_abc",
    precedence: [...POLICY_PRECEDENCE],
    default_decision: "DENY"
  };
  check("policy bundle manifest", validateV02("policy-bundle", bundle).ok);
  check("policy bundle hash", /^sha256:[0-9a-f]{64}$/.test(validatedPolicyBundleHash(bundle)));
  check(
    "policy provenance required",
    !validateV02("policy-bundle", { ...bundle, issuer: undefined }).ok
  );
  check(
    "precedence order fixed",
    !validateV02("policy-bundle", { ...bundle, precedence: [...POLICY_PRECEDENCE].reverse() }).ok
  );
  check(
    "single active exact-scope bundle",
    selectActivePolicyBundle([bundle], {
      environment: "production",
      tenant: "tenant_abc",
      at: "2026-08-14T12:00:01.000Z"
    }).manifest === bundle
  );
  check(
    "Switchboard DENY remains final",
    resolvePolicyDecision({ SWITCHBOARD_SCOPE: "DENY", BASE_POLICY: "ALLOW" }).decision === "DENY"
  );
  check(
    "approval floor cannot be lowered",
    resolvePolicyDecision({
      TENANT_ENVIRONMENT_RESTRICTION: "REQUIRE_APPROVAL",
      BASE_POLICY: "ALLOW",
      ACTION_POLICY: "ALLOW"
    }).decision === "REQUIRE_APPROVAL"
  );
  check(
    "trusted sequence uses authority order",
    verifyCompleteAuthoritySequence([{ sequence: 9 }, { sequence: 10 }, { sequence: 11 }], 9)
  );
  let gapRejected = false;
  try {
    verifyCompleteAuthoritySequence([{ sequence: 9 }, { sequence: 11 }], 9);
  } catch (error) {
    gapRejected = error.code === "TRUSTED_SEQUENCE_INVALID";
  }
  check("complete-stream gap rejected", gapRejected);
}

console.log(`\n────────────────────────────────────────────────`);
console.log(`Conformance 0.2: ${passed} passed, ${failed} failed`);
if (failed) {
  console.log("TL-PX 0.2 record suite: NOT CONFORMING");
  process.exit(1);
}
console.log("TL-PX 0.2 contract suite: CONFORMING (schemas + oracles; not a 0.2 runtime)");
