/**
 * TL-PX 0.2 record/schema conformance. Distinct from the frozen 0.1 suite.
 */
import { validateV02 } from "../src/validate-v02.mjs";
import { loadTlpx02Schemas } from "../src/schema.mjs";
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
  check("authorized-action rejects request_id", !validateV02("authorized-action", { ...authorized, request_id: "x" }).ok);
}

{
  console.log("C4  Claim and execution");
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
  console.log("C5  Reason-code catalog present");
  const catalog = JSON.parse(
    fs.readFileSync(
      path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../schemas/tlpx-0.2/reason-codes.json"),
      "utf8"
    )
  );
  check("catalog version 0.2.0", catalog.standard_version === "0.2.0");
  check("includes POLICY_DENY", catalog.evaluation.includes("POLICY_DENY"));
  check("includes ACTION_MISMATCH", catalog.claim_and_execution.includes("ACTION_MISMATCH"));
}

console.log(`\n────────────────────────────────────────────────`);
console.log(`Conformance 0.2: ${passed} passed, ${failed} failed`);
if (failed) {
  console.log("TL-PX 0.2 record suite: NOT CONFORMING");
  process.exit(1);
}
console.log("TL-PX 0.2 record suite: CONFORMING (schemas + validators; not a 0.2 runtime)");
