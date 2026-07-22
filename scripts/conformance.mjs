/**
 * TL-PX 0.1 Minimum Profile — conformance suite
 *
 * Any implementation claiming TL-PX 0.1 conformance for this reference surface
 * must pass these checks. Alternate runtimes can port the fixtures + assertions.
 */
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { fileURLToPath } from "node:url";
import {
  createPrismSignal,
  toEvaluationIntent,
  evaluateIntent,
  resolveEscalation,
  recordExecution,
  readPolicyFile,
  readAudit,
  buildChain,
  analyzeAccountability
} from "../src/index.mjs";
import {
  validateDecisionRecord,
  validateOperatorAction,
  validateExecutionRecord,
  validateAccountabilityReport,
  validateEvaluationRequest
} from "../src/validate.mjs";
import { STANDARD_ID, STANDARD_VERSION, CONTROL_MODE } from "../src/standard.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const policy = readPolicyFile(path.join(root, "config", "policy.yaml"));

let passed = 0;
let failed = 0;

function check(name, cond, detail = "") {
  if (cond) {
    passed += 1;
    console.log(`  PASS  ${name}`);
  } else {
    failed += 1;
    console.error(`  FAIL  ${name}${detail ? ` — ${detail}` : ""}`);
  }
}

function tmpLog() {
  return path.join(
    os.tmpdir(),
    `tlpx-conf-${Date.now()}-${Math.random().toString(16).slice(2)}.jsonl`
  );
}

function loadExample(name) {
  return JSON.parse(fs.readFileSync(path.join(root, "examples", name), "utf8"));
}

console.log(`TL-PX ${STANDARD_VERSION} Minimum Profile conformance\n`);

// ---------------------------------------------------------------------------
// C1 Deterministic decision fixtures
// ---------------------------------------------------------------------------
console.log("C1  Decision determinism");
{
  const cases = [
    {
      file: "intent-safe.json",
      decision: "ALLOW",
      policy_id: null,
      auth: "AUTHORIZED"
    },
    {
      file: "intent-pii-email.json",
      decision: "REQUIRE_APPROVAL",
      policy_id: "rule_pii_email",
      auth: "PENDING_HUMAN_APPROVAL"
    },
    {
      file: "intent-funds.json",
      decision: "REQUIRE_APPROVAL",
      policy_id: "rule_funds",
      auth: "PENDING_HUMAN_APPROVAL"
    },
    {
      file: "intent-human-deploy.json",
      decision: "REQUIRE_APPROVAL",
      policy_id: "rule_prod_deploy",
      auth: "PENDING_HUMAN_APPROVAL"
    }
  ];

  for (const c of cases) {
    const intent = toEvaluationIntent(createPrismSignal(loadExample(c.file)));
    const d1 = evaluateIntent(intent, policy);
    const d2 = evaluateIntent(intent, policy);
    check(
      `${c.file} → ${c.decision}`,
      d1.decision === c.decision && d1.policy_id === c.policy_id,
      `got ${d1.decision}/${d1.policy_id}`
    );
    check(
      `${c.file} auth status`,
      d1.authorization_status === c.auth,
      d1.authorization_status
    );
    check(
      `${c.file} deterministic decision+policy`,
      d1.decision === d2.decision && d1.policy_id === d2.policy_id
    );
    const v = validateDecisionRecord(d1);
    check(`${c.file} decision record validates`, v.ok, v.errors.join("; "));
    check(
      `${c.file} standard stamp`,
      d1.standard === STANDARD_ID &&
        d1.standard_version === STANDARD_VERSION &&
        d1.control_mode === CONTROL_MODE
    );
  }
}

// ---------------------------------------------------------------------------
// C2 Authorization mapping + operator
// ---------------------------------------------------------------------------
console.log("\nC2  Authorization + operator");
{
  const intent = toEvaluationIntent(createPrismSignal(loadExample("intent-pii-email.json")));
  const d = evaluateIntent(intent, policy);
  check("escalation pending", d.authorization_status === "PENDING_HUMAN_APPROVAL");

  const approve = resolveEscalation(d, {
    operator_id: "human.ops.alex",
    outcome: "APPROVE"
  });
  check("approve → AUTHORIZED", approve.authorization_status === "AUTHORIZED");
  check("operator is human", approve.operator.type === "human");
  const va = validateOperatorAction(approve);
  check("operator action validates", va.ok, va.errors.join("; "));

  const d2 = evaluateIntent(
    toEvaluationIntent(createPrismSignal(loadExample("intent-funds.json"))),
    policy
  );
  const reject = resolveEscalation(d2, {
    operator_id: "human.finance.sam",
    outcome: "REJECT"
  });
  check("reject → DENIED", reject.authorization_status === "DENIED");
  const vr = validateOperatorAction(reject);
  check("reject action validates", vr.ok, vr.errors.join("; "));
}

// ---------------------------------------------------------------------------
// C3 Execution guard
// ---------------------------------------------------------------------------
console.log("\nC3  Execution guard");
{
  const intent = toEvaluationIntent(createPrismSignal(loadExample("intent-pii-email.json")));
  const d = evaluateIntent(intent, policy);
  let blocked = false;
  try {
    recordExecution({ decision: d, executor_id: "runtime", status: "EXECUTED" });
  } catch {
    blocked = true;
  }
  check("cannot EXECUTED while PENDING", blocked);

  const op = resolveEscalation(d, { operator_id: "human.ops.alex", outcome: "APPROVE" });
  const exec = recordExecution({
    decision: d,
    operator_action: op,
    executor_id: "runtime.mailer",
    executor_type: "machine",
    status: "EXECUTED"
  });
  check("EXECUTED after APPROVE", exec.status === "EXECUTED");
  const ve = validateExecutionRecord(exec);
  check("execution record validates", ve.ok, ve.errors.join("; "));

  const safe = evaluateIntent(
    toEvaluationIntent(createPrismSignal(loadExample("intent-safe.json"))),
    policy
  );
  const safeExec = recordExecution({
    decision: safe,
    executor_id: "runtime",
    status: "EXECUTED"
  });
  check("ALLOW path EXECUTED", safeExec.status === "EXECUTED");
  check(
    "EXECUTED requires AUTHORIZED (record)",
    safeExec.authorization_status === "AUTHORIZED"
  );
}

// ---------------------------------------------------------------------------
// C4 Party model (human + machine)
// ---------------------------------------------------------------------------
console.log("\nC4  Party model");
{
  const humanIntent = toEvaluationIntent(
    createPrismSignal(loadExample("intent-human-deploy.json"))
  );
  check(
    "human declarer on request",
    humanIntent.actor_type === "human",
    humanIntent.actor_type
  );
  const d = evaluateIntent(humanIntent, policy);
  check("declarer type human", d.parties.declarer.type === "human");
  check("evaluator type machine", d.parties.evaluator.type === "machine");

  const machineIntent = toEvaluationIntent(
    createPrismSignal(loadExample("intent-safe.json"))
  );
  const d2 = evaluateIntent(machineIntent, policy);
  check("machine declarer", d2.parties.declarer.type === "machine");
}

// ---------------------------------------------------------------------------
// C5 Chain integrity
// ---------------------------------------------------------------------------
console.log("\nC5  Chain integrity");
{
  const log = tmpLog();
  const intent = toEvaluationIntent(createPrismSignal(loadExample("intent-pii-email.json")));
  const d = evaluateIntent(intent, policy, { auditPath: log });
  const op = resolveEscalation(
    d,
    { operator_id: "human.ops.alex", outcome: "APPROVE" },
    { auditPath: log }
  );
  recordExecution(
    {
      decision: d,
      operator_action: op,
      executor_id: "runtime.mailer",
      status: "EXECUTED"
    },
    { auditPath: log }
  );
  const chain = buildChain(readAudit(log), d.receipt_id);
  check("chain length >= 3", chain.length >= 3, `len=${chain.length}`);
  check(
    "chain has decision",
    chain.some((r) => r.record_type === "tlpx.decision" || r.record_type === "glass.decision")
  );
  check(
    "chain has operator",
    chain.some(
      (r) =>
        r.record_type === "tlpx.operator_action" || r.record_type === "glass.operator_action"
    )
  );
  check(
    "chain has execution",
    chain.some((r) => r.record_type === "tlpx.execution" || r.record_type === "glass.execution")
  );
  check(
    "all chain rows share receipt",
    chain.every((r) => r.receipt_id === d.receipt_id || r.linked_receipt_id === d.receipt_id)
  );
  fs.unlinkSync(log);
}

// ---------------------------------------------------------------------------
// C6 Accountability (human + machine surfaces)
// ---------------------------------------------------------------------------
console.log("\nC6  Accountability");
{
  const log = tmpLog();
  const intent = toEvaluationIntent(createPrismSignal(loadExample("intent-pii-email.json")));
  const d = evaluateIntent(intent, policy, { auditPath: log });
  const op = resolveEscalation(
    d,
    { operator_id: "human.ops.alex", outcome: "APPROVE" },
    { auditPath: log }
  );
  recordExecution(
    {
      decision: d,
      operator_action: op,
      executor_id: "runtime.mailer",
      status: "EXECUTED"
    },
    { auditPath: log }
  );
  const chain = buildChain(readAudit(log), d.receipt_id);
  const report = analyzeAccountability(chain, {
    what_went_wrong: "Sensitive attachment not disclosed",
    observed_action: "send_email",
    severity: "critical"
  });
  const vr = validateAccountabilityReport(report);
  check("accountability report validates", vr.ok, vr.errors.join("; "));
  check(
    "human party listed",
    report.parties_involved.human.includes("human.ops.alex"),
    JSON.stringify(report.parties_involved.human)
  );
  check(
    "machine parties listed",
    report.parties_involved.machine.length >= 1,
    JSON.stringify(report.parties_involved.machine)
  );
  check(
    "findings non-empty",
    Array.isArray(report.findings) && report.findings.length > 0
  );
  fs.unlinkSync(log);
}

// ---------------------------------------------------------------------------
// C7 Evaluation request validation
// ---------------------------------------------------------------------------
console.log("\nC7  Evaluation request shape");
{
  const ok = validateEvaluationRequest({
    intent_id: "i1",
    actor: "a",
    actor_type: "machine",
    declared_intent: "x"
  });
  check("minimal request ok", ok.ok);
  const bad = validateEvaluationRequest({ actor: "a" });
  check("incomplete request fails", !bad.ok);
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------
console.log(`\n${"─".repeat(48)}`);
console.log(`Conformance: ${passed} passed, ${failed} failed`);
if (failed > 0) {
  console.error("TL-PX 0.1 Minimum Profile: NON-CONFORMING");
  process.exit(1);
}
console.log("TL-PX 0.1 Minimum Profile: CONFORMING (reference implementation)");
process.exit(0);
