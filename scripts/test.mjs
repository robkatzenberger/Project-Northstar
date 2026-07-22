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
  analyzeAccountability,
  buildChain
} from "../src/index.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const policy = readPolicyFile(path.join(root, "config", "policy.yaml"));

let passed = 0;
function assert(cond, msg) {
  if (!cond) throw new Error(msg);
  passed += 1;
  console.log(`  ok  ${msg}`);
}

function tmpLog() {
  return path.join(os.tmpdir(), `glass-audit-${Date.now()}-${Math.random().toString(16).slice(2)}.jsonl`);
}

function loadExample(name) {
  return toEvaluationIntent(
    createPrismSignal(
      JSON.parse(fs.readFileSync(path.join(root, "examples", name), "utf8"))
    )
  );
}

console.log("Glass MVP tests (air-gapped)\n");

{
  console.log("prism");
  const signal = createPrismSignal({
    agent: "agent.x",
    intent_summary: "Do a thing",
    actor_type: "machine",
    action: "summarize_report",
    risk: "low"
  });
  assert(signal.prism_version === "prism_v0.1", "prism version set");
  assert(!!signal.prism_id, "prism_id present");
  assert(signal.glass.actor_type === "machine", "actor_type on glass extension");
  const intent = toEvaluationIntent(signal);
  assert(intent.actor === "agent.x", "evaluation intent actor");
}

{
  console.log("evaluate allow");
  const log = tmpLog();
  const d = evaluateIntent(loadExample("intent-safe.json"), policy, { auditPath: log });
  assert(d.decision === "ALLOW", "safe intent ALLOW");
  assert(d.authorization_status === "AUTHORIZED", "auto authorized");
  assert(d.parties.declarer.type === "machine", "machine declarer");
  assert(d.parties.evaluator.type === "machine", "machine evaluator");
  assert(d.standard === "TL-PX", "TL-PX standard stamp");
  const exec = recordExecution(
    { receipt_id: d.receipt_id, executor_id: "runtime.worker", status: "EXECUTED", result_summary: "summary ok" },
    { auditPath: log }
  );
  assert(exec.status === "EXECUTED", "safe path executed");
  fs.unlinkSync(log);
}

{
  console.log("evaluate escalate + human approve");
  const log = tmpLog();
  const d = evaluateIntent(loadExample("intent-pii-email.json"), policy, { auditPath: log });
  assert(d.decision === "REQUIRE_APPROVAL", "PII email requires approval");
  assert(d.policy_id === "rule_pii_email", "matched pii rule");

  let threw = false;
  try {
    recordExecution(
      { receipt_id: d.receipt_id, executor_id: "runtime", status: "EXECUTED" },
      { auditPath: log }
    );
  } catch {
    threw = true;
  }
  assert(threw, "cannot execute without authorization");

  const op = resolveEscalation(
    d,
    { operator_id: "human.ops.alex", outcome: "APPROVE", note: "Customer requested" },
    { auditPath: log }
  );
  assert(op.authorization_status === "AUTHORIZED", "human authorized");
  assert(op.parties.authorizer.type === "human", "authorizer is human");

  const exec = recordExecution(
    { receipt_id: d.receipt_id, executor_id: "runtime.mailer", status: "EXECUTED" },
    { auditPath: log }
  );
  assert(exec.parties.authorizer.id === "human.ops.alex", "chain keeps human authorizer");
  assert(exec.authorization_source === "human_approve", "auth source human_approve");

  const chain = buildChain(readAudit(log), d.receipt_id);
  assert(chain.length === 3, "chain has decision+operator+execution");

  const report = analyzeAccountability(chain, {
    what_went_wrong: "Wrong attachment sent to customer",
    observed_action: "send_email",
    severity: "high"
  });
  assert(report.parties_involved.human.includes("human.ops.alex"), "human on accountability report");
  assert(report.parties_involved.machine.length >= 1, "machine parties present");
  assert(report.findings.some((f) => f.code === "ACCOUNTABILITY_SURFACE_HUMAN"), "human accountability surface");
  assert(report.findings.some((f) => f.code === "ACCOUNTABILITY_SURFACE_DECLARER"), "declarer accountability surface");
  fs.unlinkSync(log);
}

{
  console.log("human reject");
  const log = tmpLog();
  const d = evaluateIntent(loadExample("intent-funds.json"), policy, { auditPath: log });
  resolveEscalation(
    d,
    { operator_id: "human.finance.sam", outcome: "REJECT", note: "Wrong vendor" },
    { auditPath: log }
  );
  const blocked = recordExecution(
    { receipt_id: d.receipt_id, executor_id: "runtime.payments", status: "BLOCKED" },
    { auditPath: log }
  );
  assert(blocked.status === "BLOCKED", "blocked execution recorded");
  fs.unlinkSync(log);
}

{
  console.log("mismatch finding");
  const log = tmpLog();
  const d = evaluateIntent(loadExample("intent-safe.json"), policy, { auditPath: log });
  recordExecution(
    { receipt_id: d.receipt_id, executor_id: "runtime", status: "EXECUTED" },
    { auditPath: log }
  );
  const chain = buildChain(readAudit(log), d.receipt_id);
  const report = analyzeAccountability(chain, {
    what_went_wrong: "Agent mutated action after gate",
    observed_action: "delete_records",
    severity: "critical"
  });
  assert(
    report.findings.some((f) => f.code === "INTENT_EXECUTION_MISMATCH"),
    "detects declared vs observed mismatch"
  );
  fs.unlinkSync(log);
}

{
  console.log("audit required");
  let threw = false;
  try {
    evaluateIntent(loadExample("intent-safe.json"), policy);
  } catch (e) {
    threw = e.message.includes("auditPath required");
  }
  assert(threw, "evaluate without auditPath throws");
}

console.log(`\nAll tests passed (${passed} assertions).`);
