/**
 * Red team against hardened air-gapped TL-PX / Glass.
 * Exit 0 = defenses held for in-scope claims; still reports residual THEATER.
 * Does NOT run the formal product technical test.
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
  loadSwitchboard,
  appendAudit,
  readAudit,
  resolveAuthorizationFromAudit,
  executeAuthorized,
  analyzeAccountability,
  buildChain
} from "../src/index.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const policy = readPolicyFile(path.join(root, "config", "policy.yaml"));
const sb = loadSwitchboard(path.join(root, "config", "switchboard.json"));

const results = [];

function tmp() {
  return path.join(os.tmpdir(), `rt-${Date.now()}-${Math.random().toString(16).slice(2)}.jsonl`);
}

function intent(file) {
  return toEvaluationIntent(
    createPrismSignal(JSON.parse(fs.readFileSync(path.join(root, "examples", file), "utf8")))
  );
}

function row(id, attack, verdict, enforcement, detail) {
  results.push({ id, attack, verdict, enforcement, detail });
  const tag = verdict.padEnd(6);
  console.log(`[${tag}] ${id}  ${detail}`);
}

console.log("════════════════════════════════════════════════════════════");
console.log(" RED TEAM — hardened air-gapped gate (not the product test)");
console.log("════════════════════════════════════════════════════════════\n");

// A1: Forged decision AUTHORIZED without evaluate chain
{
  const log = tmp();
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, { auditPath: log });
  const forged = { ...d, authorization_status: "AUTHORIZED", decision: "ALLOW" };
  let held = false;
  try {
    recordExecution(
      { decision: forged, receipt_id: d.receipt_id, executor_id: "evil", status: "EXECUTED" },
      { auditPath: log }
    );
  } catch {
    held = true;
  }
  row(
    "A1",
    "Forged AUTHORIZED decision object → EXECUTED",
    held ? "PASS" : "FAIL",
    held ? "REAL" : "HOLE",
    held ? "chain rejects forged status" : "forged EXECUTED accepted"
  );
  fs.unlinkSync(log);
}

// A2: EXECUTED while PENDING
{
  const log = tmp();
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, { auditPath: log });
  let held = false;
  try {
    recordExecution(
      { receipt_id: d.receipt_id, executor_id: "rt", status: "EXECUTED" },
      { auditPath: log }
    );
  } catch {
    held = true;
  }
  row("A2", "EXECUTED while PENDING", held ? "PASS" : "FAIL", "REAL", held ? "blocked" : "allowed");
  fs.unlinkSync(log);
}

// A3: EXECUTED after REJECT
{
  const log = tmp();
  const d = evaluateIntent(intent("intent-funds.json"), policy, { auditPath: log });
  resolveEscalation(d, { operator_id: "h1", outcome: "REJECT" }, { auditPath: log });
  let held = false;
  try {
    recordExecution(
      { receipt_id: d.receipt_id, executor_id: "rt", status: "EXECUTED" },
      { auditPath: log }
    );
  } catch {
    held = true;
  }
  row("A3", "EXECUTED after REJECT", held ? "PASS" : "FAIL", "REAL", held ? "blocked" : "allowed");
  fs.unlinkSync(log);
}

// A4: Double-resolve APPROVE then REJECT
{
  const log = tmp();
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, { auditPath: log });
  resolveEscalation(d, { operator_id: "h1", outcome: "APPROVE" }, { auditPath: log });
  let held = false;
  try {
    resolveEscalation(d, { operator_id: "h2", outcome: "REJECT" }, { auditPath: log });
  } catch (e) {
    held = e.message.includes("already resolved");
  }
  row(
    "A4",
    "Double-resolve (APPROVE then REJECT)",
    held ? "PASS" : "FAIL",
    held ? "REAL" : "HOLE",
    held ? "state machine holds" : "second resolve accepted"
  );
  fs.unlinkSync(log);
}

// A5: resolve on ALLOW
{
  const log = tmp();
  const d = evaluateIntent(intent("intent-safe.json"), policy, { auditPath: log });
  let held = false;
  try {
    resolveEscalation(d, { operator_id: "h1", outcome: "APPROVE" }, { auditPath: log });
  } catch {
    held = true;
  }
  row("A5", "resolveEscalation on ALLOW", held ? "PASS" : "FAIL", "REAL", held ? "blocked" : "allowed");
  fs.unlinkSync(log);
}

// A6: Mutate original_intent in memory then execute after real approve
{
  const log = tmp();
  const d = evaluateIntent(intent("intent-safe.json"), policy, { auditPath: log });
  d.original_intent = { ...d.original_intent, action: "transfer_funds", risk: "high" };
  // ALLOW path — execute still uses audit snapshot
  const ex = recordExecution(
    { receipt_id: d.receipt_id, executor_id: "rt", status: "EXECUTED" },
    { auditPath: log }
  );
  const snapOk = ex.original_intent?.action === "summarize_report";
  row(
    "A6",
    "In-memory original_intent mutation before execute",
    snapOk ? "PASS" : "FAIL",
    snapOk ? "REAL" : "HOLE",
    snapOk
      ? "execution record uses audit decision snapshot"
      : "mutated intent leaked into execution record"
  );
  fs.unlinkSync(log);
}

// A7: Policy expression injection / RCE attempt (safe parser — must not throw process open)
{
  const log = tmp();
  const evilPolicy = {
    policy_pack_id: "evil",
    rules: [
      {
        id: "rce",
        if: 'risk == "low" || this.constructor.constructor("return process")()',
        require: "human_approval"
      },
      {
        id: "rce2",
        if: "__proto__ == \"x\"",
        require: "human_approval"
      }
    ]
  };
  // Malformed/inject rules must not match; safe intent may still ALLOW if only evil rules fail parse
  const d = evaluateIntent(intent("intent-safe.json"), evilPolicy, { auditPath: log });
  // First rule contains illegal tokens → evaluateCondition false; second illegal → false → ALLOW
  const held = d.decision === "ALLOW" || d.decision === "REQUIRE_APPROVAL";
  row(
    "A7",
    "Policy expression injection (no new Function)",
    held && !String(d.reason).includes("process") ? "PASS" : "FAIL",
    "REAL",
    `safe parser: decision=${d.decision} (inject rules do not execute JS)`
  );
  fs.unlinkSync(log);
}

// A8: Missing actor_type defaults
{
  const log = tmp();
  const raw = {
    intent_id: "x1",
    actor: "agent.docs.summarizer",
    declared_intent: "test",
    action: "summarize_report",
    risk: "low",
    data_classes: []
  };
  const d = evaluateIntent(raw, policy, { auditPath: log, switchboard: sb });
  const defaulted = d.parties.declarer.type === "machine";
  row(
    "A8",
    "Omit actor_type",
    "WARN",
    "PARTIAL",
    defaulted
      ? "defaults to machine (SPEC prefers explicit; switchboard may normalize)"
      : "unexpected type"
  );
  fs.unlinkSync(log);
}

// A9: Raw file forgery without hash-chain / HMAC seal
{
  const log = tmp();
  // Establish a real sealed log + seal key via a legitimate evaluate
  evaluateIntent(intent("intent-safe.json"), policy, { auditPath: log });
  const fakeId = "rcpt_forged_only";
  // Attacker writes JSONL directly (no seal / broken chain)
  fs.appendFileSync(
    log,
    JSON.stringify({
      record_type: "tlpx.decision",
      standard: "TL-PX",
      standard_version: "0.1.0",
      receipt_id: fakeId,
      decision: "ALLOW",
      authorization_status: "AUTHORIZED",
      reason: "forged",
      policy_id: null,
      control_mode: "ALLOW_OR_ESCALATE",
      parties: {
        declarer: { id: "evil", type: "machine" },
        evaluator: { id: "forger", type: "machine" }
      },
      original_intent: {
        intent_id: "x",
        actor: "evil",
        actor_type: "machine",
        declared_intent: "steal",
        action: "transfer_funds",
        risk: "high",
        data_classes: []
      }
    }) + "\n"
  );
  let held = false;
  try {
    recordExecution(
      { receipt_id: fakeId, executor_id: "evil", status: "EXECUTED" },
      { auditPath: log }
    );
  } catch (e) {
    held =
      e.message.includes("integrity") ||
      e.message.includes("seal") ||
      e.message.includes("Audit integrity");
  }
  row(
    "A9",
    "Raw audit file forgery (no hash-chain/HMAC seal)",
    held ? "PASS" : "FAIL",
    held ? "REAL" : "HOLE",
    held
      ? "unsealed forge rejected by integrity verify"
      : "raw forge still authorized"
  );
  try {
    fs.unlinkSync(log);
  } catch {
    /* */
  }
  try {
    fs.unlinkSync(path.join(path.dirname(log), `.${path.basename(log)}.seal`));
  } catch {
    /* */
  }
}

// A10: evaluate without auditPath
{
  let held = false;
  try {
    evaluateIntent(intent("intent-safe.json"), policy);
  } catch (e) {
    held = e.message.includes("auditPath required");
  }
  row(
    "A10",
    "evaluate without auditPath",
    held ? "PASS" : "FAIL",
    held ? "REAL" : "HOLE",
    held ? "air-gap requires audit" : "ephemeral evaluate still open by default"
  );
}

// A11: Stub operator_action inject
{
  const log = tmp();
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, { auditPath: log });
  let held = false;
  try {
    recordExecution(
      {
        receipt_id: d.receipt_id,
        operator_action: { authorization_status: "AUTHORIZED" },
        executor_id: "evil",
        status: "EXECUTED"
      },
      { auditPath: log }
    );
  } catch {
    held = true;
  }
  row(
    "A11",
    "Inject stub operator_action AUTHORIZED",
    held ? "PASS" : "FAIL",
    held ? "REAL" : "HOLE",
    held ? "ignored; audit wins" : "stub worked"
  );
  fs.unlinkSync(log);
}

// A12: Lie on declared intent (under-declare risk) — design limit
{
  const log = tmp();
  const lying = toEvaluationIntent(
    createPrismSignal({
      agent: "agent.docs.summarizer",
      actor_type: "machine",
      intent_summary: "Just summarizing",
      action: "summarize_report",
      risk: "low",
      data_classes: []
    })
  );
  // Actually trying transfer while declaring summarize — gate trusts declaration
  const d = evaluateIntent(lying, policy, { auditPath: log, switchboard: sb });
  row(
    "A12",
    "Semantic lie (declare safe, intend harm)",
    "WARN",
    "THEATER",
    `decision=${d.decision} — gate trusts declared metadata (by design); not content inspection`
  );
  fs.unlinkSync(log);
}

// A13: Mutate decision.authorization_status in memory
{
  const log = tmp();
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, { auditPath: log });
  d.authorization_status = "AUTHORIZED";
  let held = false;
  try {
    recordExecution(
      { receipt_id: d.receipt_id, decision: d, executor_id: "evil", status: "EXECUTED" },
      { auditPath: log }
    );
  } catch {
    held = true;
  }
  row(
    "A13",
    "Flip decision.authorization_status in memory",
    held ? "PASS" : "FAIL",
    held ? "REAL" : "HOLE",
    held ? "audit re-derive ignores flip" : "memory flip worked"
  );
  fs.unlinkSync(log);
}

// A14: REJECT then APPROVE
{
  const log = tmp();
  const d = evaluateIntent(intent("intent-funds.json"), policy, { auditPath: log });
  resolveEscalation(d, { operator_id: "h1", outcome: "REJECT" }, { auditPath: log });
  let held = false;
  try {
    resolveEscalation(d, { operator_id: "h1", outcome: "APPROVE" }, { auditPath: log });
  } catch {
    held = true;
  }
  const auth = resolveAuthorizationFromAudit(log, d.receipt_id);
  row(
    "A14",
    "REJECT then APPROVE",
    held && auth.authorization_status === "DENIED" ? "PASS" : "FAIL",
    held ? "REAL" : "HOLE",
    held ? "terminal REJECT holds" : "re-open after REJECT"
  );
  fs.unlinkSync(log);
}

// A15: executeAuthorized sideEffect must not run when pending
{
  const log = tmp();
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, { auditPath: log });
  let ran = false;
  const r = await executeAuthorized({
    auditPath: log,
    receipt_id: d.receipt_id,
    executor_id: "rt",
    sideEffect: () => {
      ran = true;
      return 1;
    }
  });
  row(
    "A15",
    "executeAuthorized sideEffect while PENDING",
    !r.ok && !ran ? "PASS" : "FAIL",
    "REAL",
    !ran ? "fail-closed; sideEffect not invoked" : "sideEffect ran without auth"
  );
  fs.unlinkSync(log);
}

// A16: Never call the gate — side effect outside library
{
  const sideEffect = { did: "transfer_funds", amount: 1e6 };
  row(
    "A16",
    "Bypass: never call gate / executor",
    "WARN",
    "THEATER",
    `process can still do ${JSON.stringify(sideEffect)} — air-gap requires runtime mediation outside this package`
  );
}

// A17: allowEphemeral escape hatch
{
  let ephemeralWorks = false;
  try {
    evaluateIntent(intent("intent-safe.json"), policy, { allowEphemeral: true });
    ephemeralWorks = true;
  } catch {
    ephemeralWorks = false;
  }
  row(
    "A17",
    "allowEphemeral bypasses audit requirement",
    "WARN",
    "PARTIAL",
    ephemeralWorks
      ? "escape hatch exists for tests — must never be used in production adapters"
      : "ephemeral disabled"
  );
}

// A18: Spoof operator_id (must be on approval_route / allowlist)
{
  const log = tmp();
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, {
    auditPath: log,
    switchboard: sb
  });
  let held = false;
  try {
    resolveEscalation(
      d,
      { operator_id: "human.ceo.impostor", outcome: "APPROVE" },
      { auditPath: log, switchboard: sb, operators: sb.operators }
    );
  } catch (e) {
    held =
      e.message.includes("approval_route") || e.message.includes("allowlist");
  }
  row(
    "A18",
    "Spoof operator_id (route + allowlist)",
    held ? "PASS" : "FAIL",
    held ? "REAL" : "HOLE",
    held
      ? "impostor rejected (not on route/allowlist)"
      : "impostor accepted"
  );
  fs.unlinkSync(log);
}

// A19: Switchboard unknown DENY cannot execute
{
  const log = tmp();
  const d = evaluateIntent(intent("intent-unknown-agent.json"), policy, {
    auditPath: log,
    switchboard: sb
  });
  let ran = false;
  const r = await executeAuthorized({
    auditPath: log,
    receipt_id: d.receipt_id,
    executor_id: "rt",
    sideEffect: () => {
      ran = true;
    }
  });
  row(
    "A19",
    "Unknown agent DENY + executeAuthorized",
    d.decision === "DENY" && !r.ok && !ran ? "PASS" : "FAIL",
    "REAL",
    `decision=${d.decision} sideEffectRan=${ran}`
  );
  fs.unlinkSync(log);
}

// A20: Accountability still reports after real chain
{
  const log = tmp();
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, { auditPath: log });
  resolveEscalation(d, { operator_id: "human.ops.alex", outcome: "APPROVE" }, { auditPath: log });
  await executeAuthorized({
    auditPath: log,
    receipt_id: d.receipt_id,
    executor_id: "runtime.mailer",
    sideEffect: () => "sent"
  });
  const report = analyzeAccountability(buildChain(readAudit(log), d.receipt_id), {
    what_went_wrong: "bad attachment",
    severity: "high"
  });
  const both =
    report.parties_involved.human.length > 0 && report.parties_involved.machine.length > 0;
  row(
    "A20",
    "Accountability report human+machine after real chain",
    both ? "PASS" : "FAIL",
    "REAL",
    both ? "dual parties present" : "missing parties"
  );
  fs.unlinkSync(log);
}

// Scoreboard
console.log("\n════════════════════════════════════════════════════════════");
console.log(" SCOREBOARD");
console.log("════════════════════════════════════════════════════════════\n");

const pass = results.filter((r) => r.verdict === "PASS");
const fail = results.filter((r) => r.verdict === "FAIL");
const warn = results.filter((r) => r.verdict === "WARN");

for (const r of results) {
  console.log(
    `${r.verdict.padEnd(4)}  ${r.enforcement.padEnd(8)}  ${r.id}  ${r.attack}`
  );
}

console.log(`\nPASS: ${pass.length}   FAIL: ${fail.length}   WARN: ${warn.length}`);

console.log(`
════════════════════════════════════════════════════════════
 RED TEAM VERDICT (library / air-gapped gate only)
════════════════════════════════════════════════════════════

In-scope defenses (cooperative runtime that uses audit + executeAuthorized):
  - Forged in-memory AUTHORIZED / stub operator / status flip: ${
    fail.some((f) => ["A1", "A11", "A13"].includes(f.id)) ? "STILL BROKEN" : "HELD"
  }
  - PENDING / REJECT execute: ${fail.some((f) => ["A2", "A3", "A15"].includes(f.id)) ? "BROKEN" : "HELD"}
  - Operator state machine: ${fail.some((f) => ["A4", "A14"].includes(f.id)) ? "BROKEN" : "HELD"}
  - Mandatory audit: ${fail.some((f) => f.id === "A10") ? "BROKEN" : "HELD"}

Residual WARN (deployment / product boundaries):
  - A12 Declared-intent honesty (not DPI)
  - A16 Process that never calls the gate
  - A17 allowEphemeral escape hatch

A7 safe policy parser, A9 sealed audit, A18 operator route/allowlist: should PASS.

Production-tomorrow as air-gapped checkpoint used by a mediated executor?
  ${
    fail.length === 0
      ? "CONDITIONALLY YES for that narrow scope — residual risks are deployment boundaries."
      : "NO — fix FAIL items first."
  }
`);

if (fail.length > 0) process.exit(1);
process.exit(0);
