/**
 * No-BS theory test — adversarial probes against TL-PX claims.
 * Exit 0 only if library-level enforcement holds for what it claims to enforce.
 * Prints THEATER for claims that are documentation/cooperation only.
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
  appendAudit,
  readAudit,
  buildChain,
  analyzeAccountability,
  validateDecisionRecord,
  validateExecutionRecord
} from "../src/index.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const policy = readPolicyFile(path.join(root, "config", "policy.yaml"));

const results = [];

function record(id, claim, status, detail) {
  results.push({ id, claim, status, detail });
  const tag =
    status === "ENFORCED" ? "ENFORCED" : status === "THEATER" ? "THEATER " : status === "BROKEN" ? "BROKEN  " : status;
  console.log(`[${tag}] ${id}: ${detail}`);
}

function tmp() {
  return path.join(os.tmpdir(), `nobs-${Date.now()}-${Math.random().toString(16).slice(2)}.jsonl`);
}

function intentFrom(file) {
  return toEvaluationIntent(
    createPrismSignal(JSON.parse(fs.readFileSync(path.join(root, "examples", file), "utf8")))
  );
}

console.log("=== NO-BS THEORY TEST ===\n");

// --- 1. Happy path: theory works when you cooperate ---
{
  const d = evaluateIntent(intentFrom("intent-safe.json"), policy);
  const ex = recordExecution({ decision: d, executor_id: "rt", status: "EXECUTED" });
  record(
    "T1",
    "cooperative path works",
    d.decision === "ALLOW" && ex.status === "EXECUTED" ? "ENFORCED" : "BROKEN",
    `ALLOW→EXECUTED decision=${d.decision} exec=${ex.status}`
  );
}

// --- 2. Cannot EXECUTED while PENDING (library API) ---
{
  const d = evaluateIntent(intentFrom("intent-pii-email.json"), policy);
  let threw = false;
  try {
    recordExecution({ decision: d, executor_id: "rt", status: "EXECUTED" });
  } catch {
    threw = true;
  }
  record(
    "T2",
    "no EXECUTED without AUTHORIZED (API)",
    threw ? "ENFORCED" : "BROKEN",
    threw ? "recordExecution throws on PENDING" : "ALLOWED unauthorized EXECUTED — theory dead"
  );
}

// --- 3. Cannot EXECUTED after REJECT ---
{
  const d = evaluateIntent(intentFrom("intent-funds.json"), policy);
  const op = resolveEscalation(d, { operator_id: "h1", outcome: "REJECT" });
  let threw = false;
  try {
    recordExecution({ decision: d, operator_action: op, executor_id: "rt", status: "EXECUTED" });
  } catch {
    threw = true;
  }
  record(
    "T3",
    "no EXECUTED after REJECT",
    threw && op.authorization_status === "DENIED" ? "ENFORCED" : "BROKEN",
    threw ? "REJECT blocks EXECUTED" : "EXECUTED after REJECT"
  );
}

// --- 4. Approve non-escalated ALLOW should fail ---
{
  const d = evaluateIntent(intentFrom("intent-safe.json"), policy);
  let threw = false;
  try {
    resolveEscalation(d, { operator_id: "h1", outcome: "APPROVE" });
  } catch {
    threw = true;
  }
  record(
    "T4",
    "cannot approve non-escalated ALLOW",
    threw ? "ENFORCED" : "BROKEN",
    threw ? "resolveEscalation rejects ALLOW" : "approved ALLOW — nonsense auth path"
  );
}

// --- 5. Double-approve: does library prevent? ---
{
  const d = evaluateIntent(intentFrom("intent-pii-email.json"), policy);
  const a1 = resolveEscalation(d, { operator_id: "h1", outcome: "APPROVE" });
  let threw = false;
  try {
    // Second approve on same decision object — library has no audit-aware double-approve guard
    resolveEscalation(d, { operator_id: "h2", outcome: "APPROVE" });
  } catch {
    threw = true;
  }
  // Spec SHOULD single outcome; impl currently allows multiple operator actions if caller doesn't check audit
  record(
    "T5",
    "single operator outcome per receipt",
    threw ? "ENFORCED" : "THEATER",
    threw
      ? "double-approve blocked"
      : "double-approve allowed at API — need audit-aware guard for production (SPEC SHOULD, not MUST enforced)"
  );
  void a1;
}

// --- 6. Determinism of decision (not ids) ---
{
  const intent = intentFrom("intent-funds.json");
  const a = evaluateIntent(intent, policy);
  const b = evaluateIntent(intent, policy);
  const same =
    a.decision === b.decision && a.policy_id === b.policy_id && a.reason === b.reason;
  const idsDiffer = a.receipt_id !== b.receipt_id;
  record(
    "T6",
    "deterministic policy outcome",
    same ? "ENFORCED" : "BROKEN",
    same
      ? `decision/policy/reason stable; receipt_ids differ=${idsDiffer} (expected)`
      : "non-deterministic decision"
  );
}

// --- 7. Manual policy re-derive vs engine ---
{
  // intent-pii: action send_email + PII → first matching rule_pii_email
  const d = evaluateIntent(intentFrom("intent-pii-email.json"), policy);
  const manual = "rule_pii_email";
  record(
    "T7",
    "engine matches manual rule walk",
    d.policy_id === manual && d.decision === "REQUIRE_APPROVAL" ? "ENFORCED" : "BROKEN",
    `engine policy_id=${d.policy_id} expected=${manual}`
  );
}

// --- 8. Intent mutation after decision: receipt still has original snapshot ---
{
  const intent = intentFrom("intent-safe.json");
  const d = evaluateIntent(intent, policy);
  intent.action = "transfer_funds";
  intent.risk = "high";
  // Decision snapshot should still be original
  const snapOk = d.original_intent.action === "summarize_report";
  // Re-evaluate mutated intent should escalate
  const d2 = evaluateIntent(intent, policy);
  record(
    "T8",
    "decision freezes intent snapshot",
    snapOk && d2.decision === "REQUIRE_APPROVAL" ? "ENFORCED" : "BROKEN",
    snapOk
      ? `snapshot frozen; re-eval of mutated intent → ${d2.decision}`
      : "snapshot not frozen"
  );
}

// --- 9. Forged audit line: append fake EXECUTED without going through API ---
{
  const log = tmp();
  const d = evaluateIntent(intentFrom("intent-pii-email.json"), policy, { auditPath: log });
  // Attacker writes directly to audit
  appendAudit(log, {
    record_type: "tlpx.execution",
    standard: "TL-PX",
    standard_version: "0.1.0",
    receipt_id: d.receipt_id,
    linked_receipt_id: d.receipt_id,
    executed_at: new Date().toISOString(),
    status: "EXECUTED",
    executor: { id: "evil", type: "machine" },
    authorization_status: "AUTHORIZED" // lie
  });
  const chain = buildChain(readAudit(log), d.receipt_id);
  const forged = chain.find((r) => r.status === "EXECUTED");
  const validation = forged ? validateExecutionRecord(forged) : { ok: false };
  // Theory: real systems must protect audit integrity. Library happily appends.
  record(
    "T9",
    "audit integrity / anti-forgery",
    "THEATER",
    `raw append of fake EXECUTED accepted; validator.ok=${validation.ok} (no signature, no authn on append). Coop-only integrity.`
  );
  fs.unlinkSync(log);
}

// --- 10. Side effect without calling glass at all ---
{
  // Simulate agent that never calls the gate
  const sideEffect = { did: "transfer_funds", amount: 1_000_000 };
  record(
    "T10",
    "gate cannot stop non-integrated agents",
    "THEATER",
    `sideEffect=${JSON.stringify(sideEffect)} with zero Glass calls — SPEC admits this; enforcement is integration property`
  );
}

// --- 11. Malicious policy expression ---
{
  let threw = false;
  let weird = null;
  try {
    const evilPolicy = {
      policy_pack_id: "evil",
      rules: [{ id: "x", if: 'risk == "low"', require: "human_approval" }]
    };
    // safe intent has risk low — should REQUIRE_APPROVAL under evil policy
    weird = evaluateIntent(intentFrom("intent-safe.json"), evilPolicy);
  } catch (e) {
    threw = true;
  }
  record(
    "T11",
    "policy is operator-controlled (can be harsh)",
    !threw && weird?.decision === "REQUIRE_APPROVAL" ? "ENFORCED" : "BROKEN",
    `evil policy escalates safe intent → ${weird?.decision} (by design: policy authority is outside agent)`
  );
}

// --- 12. new Function RCE surface in policy ---
{
  let rce = false;
  try {
    const evilPolicy = {
      policy_pack_id: "rce",
      rules: [
        {
          id: "rce",
          // Attempt to break out — with(intent) scope; classic APEX risk
          if: 'risk == "low" || this.constructor.constructor("return process")().exit',
          require: "human_approval"
        }
      ]
    };
    evaluateIntent(intentFrom("intent-safe.json"), evilPolicy);
  } catch {
    rce = true; // threw — maybe blocked
  }
  // Even if it doesn't exit, the existence of new Function is a known risk
  record(
    "T12",
    "policy expression sandbox",
    "THEATER",
    "evaluateCondition uses new Function + with(intent) — untrusted policy authors = code exec risk. Fine for local operator-owned policy; NOT multi-tenant safe."
  );
  void rce;
}

// --- 13. Human + machine both on incident chain ---
{
  const log = tmp();
  const d = evaluateIntent(intentFrom("intent-pii-email.json"), policy, { auditPath: log });
  const op = resolveEscalation(
    d,
    { operator_id: "human.ops.alex", outcome: "APPROVE" },
    { auditPath: log }
  );
  recordExecution(
    { decision: d, operator_action: op, executor_id: "runtime.mailer", status: "EXECUTED" },
    { auditPath: log }
  );
  const report = analyzeAccountability(buildChain(readAudit(log), d.receipt_id), {
    what_went_wrong: "bad attachment",
    severity: "high"
  });
  const both =
    report.parties_involved.human.length > 0 && report.parties_involved.machine.length > 0;
  record(
    "T13",
    "dual accountability evidence",
    both ? "ENFORCED" : "BROKEN",
    both
      ? `human=${report.parties_involved.human.join(",")} machine=${report.parties_involved.machine.join(",")}`
      : "missing party class"
  );
  fs.unlinkSync(log);
}

// --- 14. BLOCKED is allowed without AUTHORIZED ---
{
  const d = evaluateIntent(intentFrom("intent-funds.json"), policy);
  const op = resolveEscalation(d, { operator_id: "h1", outcome: "REJECT" });
  let ok = false;
  try {
    const ex = recordExecution({
      decision: d,
      operator_action: op,
      executor_id: "rt",
      status: "BLOCKED"
    });
    ok = ex.status === "BLOCKED";
  } catch {
    ok = false;
  }
  record(
    "T14",
    "BLOCKED after REJECT (honest non-execution)",
    ok ? "ENFORCED" : "BROKEN",
    ok ? "can record BLOCKED when DENIED" : "could not record BLOCKED"
  );
}

// --- 15. Validator catches EXECUTED + non-AUTHORIZED records ---
{
  const bad = {
    record_type: "tlpx.execution",
    standard: "TL-PX",
    standard_version: "0.1.0",
    receipt_id: "x",
    executed_at: new Date().toISOString(),
    status: "EXECUTED",
    executor: { id: "e", type: "machine" },
    authorization_status: "PENDING_HUMAN_APPROVAL"
  };
  const v = validateExecutionRecord(bad);
  record(
    "T15",
    "validator rejects illegal execution shape",
    !v.ok ? "ENFORCED" : "BROKEN",
    !v.ok ? v.errors.join("; ") : "validator accepted illegal EXECUTED"
  );
}

// --- Summary ---
console.log("\n=== SCOREBOARD ===\n");
const enforced = results.filter((r) => r.status === "ENFORCED");
const theater = results.filter((r) => r.status === "THEATER");
const broken = results.filter((r) => r.status === "BROKEN");

for (const r of results) {
  console.log(`${r.status.padEnd(8)} ${r.id}  ${r.claim}`);
}

console.log(`\nENFORCED: ${enforced.length}`);
console.log(`THEATER:  ${theater.length}  (real limits — not fake passes)`);
console.log(`BROKEN:   ${broken.length}`);

console.log(`
=== VERDICT ===

Library/API theory (when you call the APIs):
  - Intent snapshot + deterministic decision: ${enforced.some((r) => r.id === "T6") ? "HOLDS" : "FAIL"}
  - No EXECUTED without AUTHORIZED via recordExecution: ${enforced.some((r) => r.id === "T2") ? "HOLDS" : "FAIL"}
  - Dual party accountability report: ${enforced.some((r) => r.id === "T13") ? "HOLDS" : "FAIL"}

What is NOT enforced (SPEC already mostly admits this):
  - Non-integrated agents (T10)
  - Audit forgery / no crypto (T9)
  - Policy sandbox / multi-tenant policy (T12)
  - Double-approve at API without audit guard (T5)

Production tomorrow?
  As an open MINIMUM STANDARD + reference for cooperative runtimes: YES, theory is real.
  As a security boundary against hostile agents or hostile audit writers: NO — not yet.
`);

// Fail only if something claimed ENFORCED is broken
if (broken.length > 0) {
  console.error("NO-BS: BROKEN claims present — fail");
  process.exit(1);
}
console.log("NO-BS: no broken library claims; theater items documented honestly.");
process.exit(0);
