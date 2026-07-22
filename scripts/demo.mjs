/**
 * End-to-end demo of Glass MVP dual accountability.
 * Writes a fresh audit log under var/demo-audit.jsonl
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  createPrismSignal,
  toEvaluationIntent,
  evaluateIntent,
  resolveEscalation,
  recordExecution,
  readPolicyFile,
  reportFromAuditFile
} from "../src/index.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const policy = readPolicyFile(path.join(root, "config", "policy.yaml"));
const logPath = path.join(root, "var", "demo-audit.jsonl");

fs.mkdirSync(path.dirname(logPath), { recursive: true });
if (fs.existsSync(logPath)) fs.unlinkSync(logPath);

function banner(title) {
  console.log(`\n${"═".repeat(64)}\n ${title}\n${"═".repeat(64)}`);
}

function show(label, obj) {
  console.log(`\n▸ ${label}`);
  console.log(JSON.stringify(obj, null, 2));
}

// Scenario A: safe auto-allow
banner("Scenario A — Safe machine action (auto-allow)");
const safeSignal = createPrismSignal(
  JSON.parse(fs.readFileSync(path.join(root, "examples/intent-safe.json"), "utf8"))
);
show("Prism signal", safeSignal);
const safeDecision = evaluateIntent(toEvaluationIntent(safeSignal), policy, { auditPath: logPath });
show("Glass decision", {
  receipt_id: safeDecision.receipt_id,
  decision: safeDecision.decision,
  authorization_status: safeDecision.authorization_status,
  parties: safeDecision.parties
});
const safeExec = recordExecution(
  {
    receipt_id: safeDecision.receipt_id,
    executor_id: "runtime.docs",
    status: "EXECUTED",
    result_summary: "Report summarized"
  },
  { auditPath: logPath }
);
show("Execution", { status: safeExec.status, executor: safeExec.executor });

// Scenario B: escalate → human approve → later incident
banner("Scenario B — PII email (human-in-the-loop) + post-incident accountability");
const emailSignal = createPrismSignal(
  JSON.parse(fs.readFileSync(path.join(root, "examples/intent-pii-email.json"), "utf8"))
);
show("Prism signal", emailSignal);
const emailDecision = evaluateIntent(toEvaluationIntent(emailSignal), policy, { auditPath: logPath });
show("Glass decision", {
  receipt_id: emailDecision.receipt_id,
  decision: emailDecision.decision,
  reason: emailDecision.reason,
  policy_id: emailDecision.policy_id,
  authorization_status: emailDecision.authorization_status
});

const approval = resolveEscalation(
  emailDecision,
  {
    operator_id: "human.ops.alex",
    outcome: "APPROVE",
    note: "Looks like a normal support reply"
  },
  { auditPath: logPath }
);
show("Human operator action", {
  operator: approval.operator,
  outcome: approval.outcome,
  authorization_status: approval.authorization_status
});

recordExecution(
  {
    receipt_id: emailDecision.receipt_id,
    executor_id: "runtime.mailer",
    status: "EXECUTED",
    result_summary: "Email sent"
  },
  { auditPath: logPath }
);

const { report } = reportFromAuditFile(
  logPath,
  emailDecision.receipt_id,
  {
    what_went_wrong: "Attachment included full SSN file not disclosed in intent summary",
    observed_action: "send_email",
    severity: "critical"
  },
  { persist: true }
);

show("Accountability report (human + machine surfaces)", {
  receipt_id: report.receipt_id,
  parties_involved: report.parties_involved,
  findings: report.findings.map((f) => ({
    code: f.code,
    party_type: f.party_type,
    party_role: f.party_role,
    party_id: f.party_id,
    severity: f.severity,
    summary: f.summary
  }))
});

// Scenario C: human rejects fund transfer
banner("Scenario C — High-risk funds transfer rejected by human");
const fundsSignal = createPrismSignal(
  JSON.parse(fs.readFileSync(path.join(root, "examples/intent-funds.json"), "utf8"))
);
const fundsDecision = evaluateIntent(toEvaluationIntent(fundsSignal), policy, { auditPath: logPath });
const rejection = resolveEscalation(
  fundsDecision,
  { operator_id: "human.finance.sam", outcome: "REJECT", note: "Vendor not on allowlist" },
  { auditPath: logPath }
);
const blocked = recordExecution(
  {
    receipt_id: fundsDecision.receipt_id,
    executor_id: "runtime.payments",
    status: "BLOCKED",
    result_summary: "Transfer not sent"
  },
  { auditPath: logPath }
);
show("Rejected path", {
  decision: fundsDecision.decision,
  operator: rejection.outcome,
  execution: blocked.status
});

banner("Demo complete");
console.log(`Audit log: ${logPath}`);
console.log(`Lines: ${fs.readFileSync(logPath, "utf8").trim().split("\n").length}`);
console.log("\nTry:");
console.log(`  node bin/glass.mjs chain ${emailDecision.receipt_id} --log ${logPath}`);
