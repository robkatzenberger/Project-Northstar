/**
 * TL-PX 0.1 Minimum Profile — conformance suite (air-gapped audit required).
 * Frozen historical evidence. Do not rewrite fixtures to match TL-PX 0.2.
 * A distinct 0.2 suite belongs in a later slice.
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

function ev(file, log) {
  return evaluateIntent(
    toEvaluationIntent(createPrismSignal(loadExample(file))),
    policy,
    { auditPath: log }
  );
}

console.log(`TL-PX ${STANDARD_VERSION} Minimum Profile conformance\n`);

console.log("C1  Decision determinism");
{
  const cases = [
    { file: "intent-safe.json", decision: "ALLOW", policy_id: null, auth: "AUTHORIZED" },
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
    const log = tmpLog();
    const d1 = ev(c.file, log);
    const d2 = ev(c.file, log);
    check(
      `${c.file} → ${c.decision}`,
      d1.decision === c.decision && d1.policy_id === c.policy_id,
      `got ${d1.decision}/${d1.policy_id}`
    );
    check(`${c.file} auth status`, d1.authorization_status === c.auth, d1.authorization_status);
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
    fs.unlinkSync(log);
  }
}

console.log("\nC2  Authorization + operator");
{
  const log = tmpLog();
  const d = ev("intent-pii-email.json", log);
  check("escalation pending", d.authorization_status === "PENDING_HUMAN_APPROVAL");

  const approve = resolveEscalation(
    d,
    { operator_id: "human.ops.alex", outcome: "APPROVE" },
    { auditPath: log }
  );
  check("approve → AUTHORIZED", approve.authorization_status === "AUTHORIZED");
  check("operator is human", approve.operator.type === "human");
  const va = validateOperatorAction(approve);
  check("operator action validates", va.ok, va.errors.join("; "));

  const log2 = tmpLog();
  const d2 = ev("intent-funds.json", log2);
  const reject = resolveEscalation(
    d2,
    { operator_id: "human.finance.sam", outcome: "REJECT" },
    { auditPath: log2 }
  );
  check("reject → DENIED", reject.authorization_status === "DENIED");
  const vr = validateOperatorAction(reject);
  check("reject action validates", vr.ok, vr.errors.join("; "));
  fs.unlinkSync(log);
  fs.unlinkSync(log2);
}

console.log("\nC3  Execution guard");
{
  const log = tmpLog();
  const d = ev("intent-pii-email.json", log);
  let blocked = false;
  try {
    recordExecution(
      { receipt_id: d.receipt_id, executor_id: "runtime", status: "EXECUTED" },
      { auditPath: log }
    );
  } catch {
    blocked = true;
  }
  check("cannot EXECUTED while PENDING", blocked);

  const op = resolveEscalation(
    d,
    { operator_id: "human.ops.alex", outcome: "APPROVE" },
    { auditPath: log }
  );
  check("approve → AUTHORIZED", op.authorization_status === "AUTHORIZED");

  const exec = recordExecution(
    {
      receipt_id: d.receipt_id,
      executor_id: "runtime.mailer",
      executor_type: "machine",
      status: "EXECUTED"
    },
    { auditPath: log }
  );
  check("EXECUTED after APPROVE", exec.status === "EXECUTED");
  const ve = validateExecutionRecord(exec);
  check("execution record validates", ve.ok, ve.errors.join("; "));

  const logSafe = tmpLog();
  const safe = ev("intent-safe.json", logSafe);
  const safeExec = recordExecution(
    { receipt_id: safe.receipt_id, executor_id: "runtime", status: "EXECUTED" },
    { auditPath: logSafe }
  );
  check("ALLOW path EXECUTED", safeExec.status === "EXECUTED");
  check(
    "EXECUTED requires AUTHORIZED (record)",
    safeExec.authorization_status === "AUTHORIZED"
  );
  fs.unlinkSync(log);
  fs.unlinkSync(logSafe);
}

console.log("\nC4  Party model");
{
  const log = tmpLog();
  const humanIntent = toEvaluationIntent(
    createPrismSignal(loadExample("intent-human-deploy.json"))
  );
  check("human declarer on request", humanIntent.actor_type === "human", humanIntent.actor_type);
  const d = evaluateIntent(humanIntent, policy, { auditPath: log });
  check("declarer type human", d.parties.declarer.type === "human");
  check("evaluator type machine", d.parties.evaluator.type === "machine");

  const d2 = ev("intent-safe.json", log);
  check("machine declarer", d2.parties.declarer.type === "machine");
  fs.unlinkSync(log);
}

console.log("\nC5  Chain integrity");
{
  const log = tmpLog();
  const d = ev("intent-pii-email.json", log);
  const op = resolveEscalation(
    d,
    { operator_id: "human.ops.alex", outcome: "APPROVE" },
    { auditPath: log }
  );
  recordExecution(
    { receipt_id: d.receipt_id, executor_id: "runtime.mailer", status: "EXECUTED" },
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
  void op;
  fs.unlinkSync(log);
}

console.log("\nC6  Accountability");
{
  const log = tmpLog();
  const d = ev("intent-pii-email.json", log);
  resolveEscalation(
    d,
    { operator_id: "human.ops.alex", outcome: "APPROVE" },
    { auditPath: log }
  );
  recordExecution(
    { receipt_id: d.receipt_id, executor_id: "runtime.mailer", status: "EXECUTED" },
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
  check("findings non-empty", Array.isArray(report.findings) && report.findings.length > 0);
  fs.unlinkSync(log);
}

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

console.log(`\n${"─".repeat(48)}`);
console.log(`Conformance: ${passed} passed, ${failed} failed`);
if (failed > 0) {
  console.error("TL-PX 0.1 Minimum Profile: NON-CONFORMING");
  process.exit(1);
}
console.log("TL-PX 0.1 Minimum Profile: CONFORMING (reference implementation)");
process.exit(0);
