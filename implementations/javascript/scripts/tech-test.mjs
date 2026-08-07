/**
 * Formal air-gapped technical test #1
 * Log: <monorepo>/var/tech-test-audit.jsonl
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  createPrismSignal,
  toEvaluationIntent,
  evaluateIntent,
  resolveEscalation,
  executeAuthorized,
  resolveAuthorizationFromAudit,
  buildChain,
  readAudit,
  readPolicyFile,
  loadSwitchboard
} from "../src/index.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const monorepoRoot = path.resolve(root, "../..");
const auditPath = path.join(monorepoRoot, "var", "tech-test-audit.jsonl");
const policy = readPolicyFile(path.join(root, "config", "policy.yaml"));
const sb = loadSwitchboard(path.join(root, "config", "switchboard.json"));

const results = [];

function banner(t) {
  console.log(`\n${"═".repeat(64)}\n ${t}\n${"═".repeat(64)}`);
}

function check(id, cond, detail) {
  results.push({ id, pass: !!cond, detail });
  console.log(`  ${cond ? "✓ PASS" : "✗ FAIL"}  ${id}: ${detail}`);
  return !!cond;
}

function intent(fileOrObj) {
  if (typeof fileOrObj === "string") {
    return toEvaluationIntent(
      createPrismSignal(
        JSON.parse(fs.readFileSync(path.join(root, "examples", fileOrObj), "utf8"))
      )
    );
  }
  return toEvaluationIntent(createPrismSignal(fileOrObj));
}

// Clean slate
fs.mkdirSync(path.dirname(auditPath), { recursive: true });
fs.writeFileSync(auditPath, "", { mode: 0o600 });

banner("TECHNICAL TEST #1 — Air-gapped Trust Layer");
console.log(`Audit log: ${auditPath}`);
console.log(`Started:   ${new Date().toISOString()}`);
console.log(`Switchboard: ${sb.switchboard_id}`);

// ─── T1 Safe allow + execute ───────────────────────────────────────────
banner("T1  Safe path: ALLOW → executeAuthorized");
{
  const d = evaluateIntent(intent("intent-safe.json"), policy, {
    auditPath,
    switchboard: sb
  });
  check("T1.1", d.decision === "ALLOW", `decision=${d.decision}`);
  check("T1.2", d.authorization_status === "AUTHORIZED", `auth=${d.authorization_status}`);
  check("T1.3", d.switchboard?.whitelisted === true, "principal whitelisted");

  let ran = false;
  const r = await executeAuthorized({
    auditPath,
    receipt_id: d.receipt_id,
    executor_id: "tech-test.runtime",
    sideEffect: () => {
      ran = true;
      return "summary-complete";
    }
  });
  check("T1.4", r.ok === true && ran === true, `sideEffect ran, ok=${r.ok}`);
  check("T1.5", r.execution?.status === "EXECUTED", `status=${r.execution?.status}`);
  console.log(`  receipt: ${d.receipt_id}`);
}

// ─── T2 Switchboard DENY (unknown) — automatic block ───────────────────
banner("T2  Switchboard first line: unknown principal → DENY → no side effect");
{
  const d = evaluateIntent(intent("intent-unknown-agent.json"), policy, {
    auditPath,
    switchboard: sb
  });
  check("T2.1", d.decision === "DENY", `decision=${d.decision}`);
  check(
    "T2.2",
    d.policy_id === "switchboard.unknown_deny",
    `policy_id=${d.policy_id} (switchboard, not policy rule)`
  );
  check("T2.3", d.authorization_status === "DENIED", `auth=${d.authorization_status}`);

  let ran = false;
  const r = await executeAuthorized({
    auditPath,
    receipt_id: d.receipt_id,
    executor_id: "tech-test.runtime",
    sideEffect: () => {
      ran = true;
      return "SHOULD-NOT-RUN";
    }
  });
  check("T2.4", r.ok === false && ran === false, `sideEffect blocked (ran=${ran})`);
  check("T2.5", r.execution?.status === "BLOCKED", `status=${r.execution?.status}`);
  console.log(`  receipt: ${d.receipt_id}`);
}

// ─── T3 Not whitelisted DENY ───────────────────────────────────────────
banner("T3  Switchboard: not whitelisted → DENY");
{
  const d = evaluateIntent(intent("intent-not-whitelisted.json"), policy, {
    auditPath,
    switchboard: sb
  });
  check("T3.1", d.decision === "DENY", `decision=${d.decision}`);
  check(
    "T3.2",
    d.policy_id === "switchboard.not_whitelisted",
    `policy_id=${d.policy_id}`
  );
  let ran = false;
  await executeAuthorized({
    auditPath,
    receipt_id: d.receipt_id,
    executor_id: "tech-test.runtime",
    sideEffect: () => {
      ran = true;
    }
  });
  check("T3.3", ran === false, "sideEffect did not run");
  console.log(`  receipt: ${d.receipt_id}`);
}

// ─── T4 Escalate → APPROVE → execute ───────────────────────────────────
banner("T4  Gate: REQUIRE_APPROVAL → human APPROVE → execute");
{
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, {
    auditPath,
    switchboard: sb
  });
  check("T4.1", d.decision === "REQUIRE_APPROVAL", `decision=${d.decision}`);
  check(
    "T4.2",
    d.authorization_status === "PENDING_HUMAN_APPROVAL",
    `auth=${d.authorization_status}`
  );
  check(
    "T4.3",
    Array.isArray(d.approval_route) && d.approval_route.length > 0,
    `approval_route=${JSON.stringify(d.approval_route)}`
  );

  let ranPending = false;
  const blocked = await executeAuthorized({
    auditPath,
    receipt_id: d.receipt_id,
    executor_id: "tech-test.runtime",
    sideEffect: () => {
      ranPending = true;
    }
  });
  check("T4.4", blocked.ok === false && !ranPending, "cannot execute while PENDING");

  const op = resolveEscalation(
    d,
    {
      operator_id: d.approval_route[0] || "human.ops.alex",
      outcome: "APPROVE",
      note: "Technical test approval"
    },
    { auditPath }
  );
  check("T4.5", op.outcome === "APPROVE", "operator APPROVE recorded");

  const auth = resolveAuthorizationFromAudit(auditPath, d.receipt_id);
  check("T4.6", auth.authorization_status === "AUTHORIZED", `chain auth=${auth.authorization_status}`);
  check("T4.7", auth.source === "human_approve", `source=${auth.source}`);

  let ran = false;
  const r = await executeAuthorized({
    auditPath,
    receipt_id: d.receipt_id,
    executor_id: "tech-test.mailer",
    sideEffect: () => {
      ran = true;
      return "email-sent-sim";
    }
  });
  check("T4.8", r.ok && ran, "sideEffect ran after human approve");
  check("T4.9", r.execution?.status === "EXECUTED", `status=${r.execution?.status}`);
  console.log(`  receipt: ${d.receipt_id}`);
}

// ─── T5 REJECT is terminal block ───────────────────────────────────────
banner("T5  Gate: REQUIRE_APPROVAL → human REJECT → blocked");
{
  const d = evaluateIntent(intent("intent-funds.json"), policy, {
    auditPath,
    switchboard: sb
  });
  check("T5.1", d.decision === "REQUIRE_APPROVAL", `decision=${d.decision} (low cred / funds)`);

  resolveEscalation(
    d,
    {
      operator_id: d.approval_route[0] || "human.finance.sam",
      outcome: "REJECT",
      note: "Technical test rejection"
    },
    { auditPath }
  );

  let ran = false;
  const r = await executeAuthorized({
    auditPath,
    receipt_id: d.receipt_id,
    executor_id: "tech-test.payments",
    sideEffect: () => {
      ran = true;
    }
  });
  check("T5.2", r.ok === false && !ran, "sideEffect blocked after REJECT");
  check("T5.3", r.execution?.status === "BLOCKED", `status=${r.execution?.status}`);

  const auth = resolveAuthorizationFromAudit(auditPath, d.receipt_id);
  check("T5.4", auth.authorization_status === "DENIED", `auth=${auth.authorization_status}`);
  console.log(`  receipt: ${d.receipt_id}`);
}

// ─── T6 Audit integrity snapshot ───────────────────────────────────────
banner("T6  Audit log present and chainable");
{
  const lines = fs.readFileSync(auditPath, "utf8").trim().split("\n").filter(Boolean);
  check("T6.1", lines.length >= 10, `audit lines=${lines.length}`);
  const records = readAudit(auditPath);
  const decisions = records.filter((r) => r.record_type === "tlpx.decision");
  check("T6.2", decisions.length >= 5, `decision records=${decisions.length}`);

  // Spot-check full chain: escalated PII path (REQUIRE_APPROVAL + operator + execute)
  // Note: not-whitelisted fixture also uses send_email — filter by decision type.
  const pii = decisions.find(
    (r) =>
      r.original_intent?.action === "send_email" &&
      r.decision === "REQUIRE_APPROVAL"
  );
  if (pii) {
    const chain = buildChain(records, pii.receipt_id);
    const types = chain.map((r) => r.record_type);
    check(
      "T6.3",
      types.includes("tlpx.decision") &&
        types.includes("tlpx.operator_action") &&
        types.includes("tlpx.execution"),
      `pii escalate chain types=${types.join(" → ")}`
    );
  } else {
    check("T6.3", false, "pii REQUIRE_APPROVAL decision not found");
  }
}

// ─── Summary ───────────────────────────────────────────────────────────
banner("RESULTS");
const passed = results.filter((r) => r.pass).length;
const failed = results.filter((r) => !r.pass).length;

for (const r of results) {
  console.log(`${r.pass ? "PASS" : "FAIL"}  ${r.id}  ${r.detail}`);
}

console.log(`\n${"─".repeat(64)}`);
console.log(`TOTAL: ${passed} passed, ${failed} failed out of ${results.length}`);
console.log(`Audit: ${auditPath}`);
console.log(`Bytes: ${fs.statSync(auditPath).size}`);
console.log(`Ended: ${new Date().toISOString()}`);

if (failed === 0) {
  console.log(`
╔══════════════════════════════════════════════════════════════╗
║  TECHNICAL TEST #1: PASS                                     ║
║  Air-gapped gate + Switchboard-first DENY + fail-closed exec ║
╚══════════════════════════════════════════════════════════════╝
`);
  process.exit(0);
}

console.log(`
*** TECHNICAL TEST #1: FAIL — review FAIL lines above ***
`);
process.exit(1);
