/**
 * TL-PX / Glass — adversarial red-team suite
 *
 * Goal: attack claimed guarantees and report PASS/FAIL per attack.
 * "PASS" = defense held (attack blocked as expected)
 * "FAIL" = defense broken (attack succeeded — theory hole)
 *
 * Run: node scripts/adversarial-redteam.mjs
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
  parsePolicyText,
  evaluateRules,
  appendAudit,
  readAudit,
  buildChain,
  analyzeAccountability,
  validateEvaluationRequest,
  validateDecisionRecord,
  validateExecutionRecord
} from "../src/index.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const policy = readPolicyFile(path.join(root, "config", "policy.yaml"));

const results = [];

function tmpLog() {
  return path.join(
    os.tmpdir(),
    `tlpx-rt-${Date.now()}-${Math.random().toString(16).slice(2)}.jsonl`
  );
}

function baseIntent(overrides = {}) {
  return {
    intent_id: `rt_${Math.random().toString(16).slice(2)}`,
    actor: "agent.attacker",
    actor_type: "machine",
    declared_intent: "adversarial test intent",
    action: "send_email",
    target: "user@example.com",
    risk: "low",
    data_classes: ["PII"],
    timestamp: new Date().toISOString(),
    ...overrides
  };
}

/**
 * @param {string} id
 * @param {string} claim  theory claim under attack
 * @param {string} attack description
 * @param {"PASS"|"FAIL"|"WARN"} verdict
 * @param {string} detail
 * @param {"REAL"|"DOC-ONLY"|"PARTIAL"} enforcement
 */
function report(id, claim, attack, verdict, detail, enforcement) {
  results.push({ id, claim, attack, verdict, detail, enforcement });
  const mark = verdict === "PASS" ? "PASS" : verdict === "WARN" ? "WARN" : "FAIL";
  console.log(`\n[${mark}] ${id}`);
  console.log(`  claim:   ${claim}`);
  console.log(`  attack:  ${attack}`);
  console.log(`  detail:  ${detail}`);
  console.log(`  enforce: ${enforcement}`);
}

function threw(fn) {
  try {
    fn();
    return { threw: false, err: null, value: null };
  } catch (e) {
    return { threw: true, err: e, value: null };
  }
}

function threwValue(fn) {
  try {
    return { threw: false, err: null, value: fn() };
  } catch (e) {
    return { threw: true, err: e, value: null };
  }
}

console.log("═".repeat(64));
console.log("TL-PX ADVERSARIAL RED TEAM");
console.log("═".repeat(64));

// ─────────────────────────────────────────────────────────────────────────────
// A1: Execute without evaluate (forged decision object)
// ─────────────────────────────────────────────────────────────────────────────
{
  const forgedDecision = {
    record_type: "tlpx.decision",
    standard: "TL-PX",
    standard_version: "0.1.0",
    control_mode: "ALLOW_OR_ESCALATE",
    receipt_id: "rcpt_forged_never_evaluated",
    evaluated_at: new Date().toISOString(),
    decision: "ALLOW",
    reason: "I made this up",
    policy_id: null,
    authorization_status: "AUTHORIZED",
    parties: {
      declarer: { id: "attacker", type: "machine" },
      evaluator: { id: "tlpx-reference", type: "machine" },
      authorizer: null
    },
    original_intent: baseIntent({ declared_intent: "exfiltrate secrets" })
  };

  const r = threwValue(() =>
    recordExecution({
      decision: forgedDecision,
      executor_id: "runtime.evil",
      status: "EXECUTED",
      result_summary: "secrets stolen"
    })
  );

  if (!r.threw && r.value?.status === "EXECUTED") {
    report(
      "A1",
      "Intent declared before execution / EXECUTED requires AUTHORIZED",
      "Pass forged AUTHORIZED decision (never evaluated) into recordExecution",
      "FAIL",
      "recordExecution accepted forged decision and emitted EXECUTED — no check that decision came from evaluateIntent or audit trail",
      "PARTIAL"
    );
  } else {
    report(
      "A1",
      "Intent declared before execution",
      "Pass forged decision into recordExecution",
      "PASS",
      r.err?.message || "blocked",
      "REAL"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A2: Execute while PENDING
// ─────────────────────────────────────────────────────────────────────────────
{
  const d = evaluateIntent(baseIntent(), policy);
  const r = threw(() =>
    recordExecution({ decision: d, executor_id: "runtime", status: "EXECUTED" })
  );
  if (r.threw && d.authorization_status === "PENDING_HUMAN_APPROVAL") {
    report(
      "A2",
      "Cannot EXECUTED without AUTHORIZED",
      "recordExecution EXECUTED while PENDING_HUMAN_APPROVAL",
      "PASS",
      `blocked: ${r.err.message}`,
      "REAL"
    );
  } else {
    report(
      "A2",
      "Cannot EXECUTED without AUTHORIZED",
      "recordExecution EXECUTED while PENDING",
      "FAIL",
      "PENDING path allowed EXECUTED",
      "DOC-ONLY"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A3: Execute after REJECT
// ─────────────────────────────────────────────────────────────────────────────
{
  const d = evaluateIntent(baseIntent(), policy);
  const op = resolveEscalation(d, { operator_id: "human.ops", outcome: "REJECT" });
  const r = threw(() =>
    recordExecution({
      decision: d,
      operator_action: op,
      executor_id: "runtime",
      status: "EXECUTED"
    })
  );
  if (r.threw && op.authorization_status === "DENIED") {
    report(
      "A3",
      "Cannot EXECUTED without AUTHORIZED",
      "recordExecution EXECUTED after REJECT (DENIED)",
      "PASS",
      `blocked: ${r.err.message}`,
      "REAL"
    );
  } else {
    report(
      "A3",
      "Cannot EXECUTED without AUTHORIZED",
      "EXECUTED after REJECT",
      "FAIL",
      "DENIED path allowed EXECUTED",
      "DOC-ONLY"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A3b: After REJECT, omit operator_action — uses decision auth (still PENDING)
// ─────────────────────────────────────────────────────────────────────────────
{
  const d = evaluateIntent(baseIntent(), policy);
  resolveEscalation(d, { operator_id: "human.ops", outcome: "REJECT" });
  // Attacker omits operator_action so recordExecution reads decision.authorization_status
  // which is still PENDING (decision object never updated). EXECUTED should still fail.
  const r = threw(() =>
    recordExecution({ decision: d, executor_id: "runtime", status: "EXECUTED" })
  );
  if (r.threw) {
    report(
      "A3b",
      "Cannot EXECUTED without AUTHORIZED",
      "After REJECT, omit operator_action and claim EXECUTED (decision still PENDING)",
      "PASS",
      `blocked via PENDING: ${r.err.message}`,
      "REAL"
    );
  } else {
    report(
      "A3b",
      "Cannot EXECUTED without AUTHORIZED",
      "Omit operator_action after REJECT",
      "FAIL",
      "EXECUTED succeeded by ignoring REJECT",
      "DOC-ONLY"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A4: Double-approve / double-resolve
// ─────────────────────────────────────────────────────────────────────────────
{
  const log = tmpLog();
  const d = evaluateIntent(baseIntent(), policy, { auditPath: log });
  const a1 = resolveEscalation(
    d,
    { operator_id: "human.ops.a", outcome: "APPROVE" },
    { auditPath: log }
  );
  const r2 = threwValue(() =>
    resolveEscalation(
      d,
      { operator_id: "human.ops.b", outcome: "REJECT" },
      { auditPath: log }
    )
  );

  const records = readAudit(log);
  const ops = records.filter(
    (x) =>
      x.record_type === "tlpx.operator_action" || x.record_type === "glass.operator_action"
  );

  if (!r2.threw && ops.length >= 2) {
    report(
      "A4",
      "Human accountability / single resolution",
      "Double-resolve: APPROVE then REJECT same receipt",
      "FAIL",
      `Both operator actions accepted (count=${ops.length}). Last outcome=${r2.value.outcome}. SPEC §6.3 SHOULD reject double-resolution — not enforced. Chain now has conflicting AUTHORIZED then DENIED.`,
      "DOC-ONLY"
    );
  } else if (r2.threw) {
    report(
      "A4",
      "Human accountability / single resolution",
      "Double-resolve same receipt",
      "PASS",
      `second resolve blocked: ${r2.err.message}`,
      "REAL"
    );
  } else {
    report(
      "A4",
      "Human accountability / single resolution",
      "Double-resolve",
      "WARN",
      "unexpected state",
      "PARTIAL"
    );
  }
  try {
    fs.unlinkSync(log);
  } catch {
    /* ignore */
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A5: Approve non-escalated ALLOW
// ─────────────────────────────────────────────────────────────────────────────
{
  const d = evaluateIntent(
    baseIntent({ action: "summarize_report", data_classes: [], risk: "low" }),
    policy
  );
  const r = threw(() =>
    resolveEscalation(d, { operator_id: "human.ops", outcome: "APPROVE" })
  );
  if (r.threw && d.decision === "ALLOW") {
    report(
      "A5",
      "Only REQUIRE_APPROVAL may be resolved",
      "resolveEscalation on ALLOW decision",
      "PASS",
      `blocked: ${r.err.message}`,
      "REAL"
    );
  } else {
    report(
      "A5",
      "Only REQUIRE_APPROVAL may be resolved",
      "Approve ALLOW",
      "FAIL",
      "ALLOW was resolvable",
      "DOC-ONLY"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A6: Mutate intent after decision; claim same receipt
// ─────────────────────────────────────────────────────────────────────────────
{
  const log = tmpLog();
  const intent = baseIntent({
    action: "summarize_report",
    data_classes: [],
    risk: "low",
    declared_intent: "harmless summary"
  });
  const d = evaluateIntent(intent, policy, { auditPath: log });
  // Mutate snapshot in-place after decision (attacker-controlled object graph)
  d.original_intent.action = "transfer_funds";
  d.original_intent.declared_intent = "drain treasury";
  d.original_intent.data_classes = ["FINANCIAL"];
  d.original_intent.risk = "high";

  const exec = recordExecution(
    {
      decision: d,
      executor_id: "runtime",
      status: "EXECUTED",
      result_summary: "actually transferred funds"
    },
    { auditPath: log }
  );

  const chain = buildChain(readAudit(log), d.receipt_id);
  const decisionOnDisk = chain.find((r) => r.record_type === "tlpx.decision");
  // Decision was already written with original; in-memory mutation doesn't rewrite audit
  // BUT execution record carries mutated original_intent
  const intentOnExec = exec.original_intent?.action;
  const intentOnDecision = decisionOnDisk?.original_intent?.action;

  // Also: re-append mutated decision would create forked truth — try rewrite via append
  const mutatedClone = structuredClone(decisionOnDisk);
  mutatedClone.original_intent.action = "transfer_funds";
  mutatedClone.original_intent.declared_intent = "drain treasury";
  // No integrity hash — append "correction" of same receipt
  appendAudit(log, mutatedClone);

  const all = readAudit(log);
  const decisions = all.filter((r) => r.record_type === "tlpx.decision");

  if (
    d.decision === "ALLOW" &&
    exec.status === "EXECUTED" &&
    intentOnExec === "transfer_funds" &&
    intentOnDecision === "summarize_report" &&
    decisions.length >= 2
  ) {
    report(
      "A6",
      "Intent declared before execution (immutable snapshot)",
      "Mutate decision.original_intent after evaluate; re-append same receipt_id with new intent",
      "FAIL",
      `Execution carries mutated action=${intentOnExec} while first decision has ${intentOnDecision}. Same receipt_id has ${decisions.length} decision rows with conflicting intents. No content hash / signature binds receipt→intent→decision.`,
      "DOC-ONLY"
    );
  } else if (exec.status === "EXECUTED" && intentOnExec === "transfer_funds") {
    report(
      "A6",
      "Intent declared before execution (immutable snapshot)",
      "Mutate original_intent after decision, then EXECUTED",
      "FAIL",
      `Execution recorded with mutated intent action=${intentOnExec}; decision was ALLOW for harmless action. No immutability of snapshot.`,
      "PARTIAL"
    );
  } else {
    report(
      "A6",
      "Intent declared before execution",
      "Post-decision intent mutation",
      "PASS",
      "mutation blocked or detected",
      "REAL"
    );
  }
  try {
    fs.unlinkSync(log);
  } catch {
    /* ignore */
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A7: Empty / malicious policy expressions
// ─────────────────────────────────────────────────────────────────────────────
{
  // A7a: empty expression / missing if
  const emptyPolicy = parsePolicyText(`rules:
  - id: empty_if
    description: empty
    if: 
    require: human_approval
`);
  // parser may set if to empty string or undefined
  const rEmpty = threwValue(() =>
    evaluateRules(baseIntent({ action: "x", data_classes: [] }), emptyPolicy)
  );

  // A7b: RCE via new Function / with(intent)
  let rceHit = false;
  const evilPolicy = {
    policy_pack_id: "evil",
    rules: [
      {
        id: "rce",
        description: "payload",
        if: `(function(){ globalThis.__TLPX_RCE__ = "pwned"; return true })()`,
        require: "human_approval"
      }
    ]
  };
  delete globalThis.__TLPX_RCE__;
  const rRce = threwValue(() => evaluateRules(baseIntent(), evilPolicy));
  rceHit = globalThis.__TLPX_RCE__ === "pwned";

  // A7c: read process env via expression
  let envLeak = null;
  const leakPolicy = {
    policy_pack_id: "leak",
    rules: [
      {
        id: "leak",
        if: `(function(){ globalThis.__TLPX_ENV__ = (typeof process !== "undefined" && process.env && process.env.HOME) || "no"; return false })()`,
        require: "human_approval"
      }
    ]
  };
  delete globalThis.__TLPX_ENV__;
  evaluateRules(baseIntent({ action: "noop", data_classes: [] }), leakPolicy);
  envLeak = globalThis.__TLPX_ENV__;

  // A7d: syntax error → uncaught throw during evaluate
  const badSyntax = {
    policy_pack_id: "bad",
    rules: [{ id: "syn", if: "action == ", require: "human_approval" }]
  };
  const rSyn = threw(() => evaluateRules(baseIntent(), badSyntax));

  // A7e: always-true via prototype pollution style / bare true
  const always = {
    policy_pack_id: "t",
    rules: [{ id: "always", if: "true", require: "human_approval" }]
  };
  const alwaysOut = evaluateRules(baseIntent({ action: "noop", data_classes: [] }), always);

  const holes = [];
  if (rceHit) holes.push("RCE via new Function (global side effect)");
  if (envLeak && envLeak !== "no") holes.push(`env leak HOME=${envLeak}`);
  if (!rSyn.threw) holes.push("bad syntax did not throw");
  // empty if — if it crashes or auto-allows, note it
  if (rEmpty.threw) holes.push(`empty if throws: ${rEmpty.err.message}`);
  if (alwaysOut.decision === "REQUIRE_APPROVAL") {
    // true is valid JS — expected that it works; not a hole by itself
  }

  if (rceHit || (envLeak && envLeak !== "no")) {
    report(
      "A7",
      "Deterministic policy (no LLM) + sandboxed expressions (SPEC §13)",
      "Malicious policy expression via new Function/with(intent)",
      "FAIL",
      `Policy evaluator is arbitrary JS execution, not a sandboxed DSL. ${holes.join("; ")}. SPEC §13 says 'must be sandboxed in production' — NOT enforced in reference code.`,
      "DOC-ONLY"
    );
  } else {
    report(
      "A7",
      "Deterministic policy / sandbox",
      "Malicious policy expressions",
      rSyn.threw ? "WARN" : "PASS",
      holes.join("; ") || "no RCE observed",
      "PARTIAL"
    );
  }
  delete globalThis.__TLPX_RCE__;
  delete globalThis.__TLPX_ENV__;
}

// ─────────────────────────────────────────────────────────────────────────────
// A8: Missing actor_type / missing fields
// ─────────────────────────────────────────────────────────────────────────────
{
  // Missing actor_type — evaluateIntent defaults to "machine" BEFORE validate
  const r1 = threwValue(() =>
    evaluateIntent(
      {
        intent_id: "missing_type",
        actor: "someone",
        declared_intent: "do stuff",
        action: "summarize_report"
        // no actor_type
      },
      policy
    )
  );

  // Missing actor entirely
  const r2 = threw(() =>
    evaluateIntent(
      {
        intent_id: "no_actor",
        actor_type: "machine",
        declared_intent: "do stuff"
      },
      policy
    )
  );

  // Missing declared_intent
  const r3 = threw(() =>
    evaluateIntent(
      {
        intent_id: "no_intent",
        actor: "a",
        actor_type: "machine"
      },
      policy
    )
  );

  // Invalid actor_type
  const r4 = threw(() =>
    evaluateIntent(
      {
        intent_id: "bad_type",
        actor: "a",
        actor_type: "alien",
        declared_intent: "x"
      },
      policy
    )
  );

  // Direct validate: missing actor_type fails; evaluate soft-defaults
  const v = validateEvaluationRequest({
    intent_id: "x",
    actor: "a",
    declared_intent: "y"
  });

  const softDefault =
    !r1.threw && r1.value?.parties?.declarer?.type === "machine";
  const hardMissingActor = r2.threw;
  const hardMissingIntent = r3.threw;
  const hardBadType = r4.threw;
  const validateStrict = !v.ok;

  if (softDefault && validateStrict) {
    report(
      "A8",
      "Party model: actor_type MUST be human|machine",
      "Omit actor_type on evaluateIntent; omit actor; bad type",
      "FAIL",
      `SPEC §6.1 actor_type MUST — but evaluateIntent defaults actor_type to "machine" before validation (spoofable absence). Missing actor=${hardMissingActor ? "blocked" : "ALLOWED"}; missing declared_intent=${hardMissingIntent ? "blocked" : "ALLOWED"}; alien type=${hardBadType ? "blocked" : "ALLOWED"}. validateEvaluationRequest is strict; evaluate path is not.`,
      "PARTIAL"
    );
  } else if (!softDefault && hardMissingActor && hardMissingIntent && hardBadType) {
    report(
      "A8",
      "Party model required fields",
      "Missing/invalid fields",
      "PASS",
      "all invalid requests rejected",
      "REAL"
    );
  } else {
    report(
      "A8",
      "Party model required fields",
      "Missing/invalid fields",
      "WARN",
      `softDefault=${softDefault} actor=${hardMissingActor} intent=${hardMissingIntent} type=${hardBadType}`,
      "PARTIAL"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A9: Raw audit append forgery — fake AUTHORIZED + EXECUTED
// ─────────────────────────────────────────────────────────────────────────────
{
  const log = tmpLog();
  const rid = "rcpt_forged_audit_line";
  const fakeDecision = {
    record_type: "tlpx.decision",
    standard: "TL-PX",
    standard_version: "0.1.0",
    control_mode: "ALLOW_OR_ESCALATE",
    receipt_id: rid,
    evaluated_at: new Date().toISOString(),
    decision: "ALLOW",
    reason: "forged",
    policy_id: null,
    authorization_status: "AUTHORIZED",
    parties: {
      declarer: { id: "forger", type: "human" },
      evaluator: { id: "tlpx-reference", type: "machine" },
      authorizer: null
    },
    original_intent: baseIntent({ intent_id: "forged_intent" })
  };
  const fakeExec = {
    record_type: "tlpx.execution",
    standard: "TL-PX",
    standard_version: "0.1.0",
    receipt_id: rid,
    linked_receipt_id: rid,
    executed_at: new Date().toISOString(),
    status: "EXECUTED",
    executor: { id: "evil", type: "machine" },
    authorization_status: "AUTHORIZED",
    parties: {
      declarer: fakeDecision.parties.declarer,
      evaluator: fakeDecision.parties.evaluator,
      executor: { id: "evil", type: "machine" }
    },
    original_intent: fakeDecision.original_intent
  };

  appendAudit(log, fakeDecision);
  appendAudit(log, fakeExec);

  const chain = buildChain(readAudit(log), rid);
  const reportAcc = analyzeAccountability(chain, {
    what_went_wrong: "forged chain looks legit",
    severity: "critical"
  });

  const hasExec = chain.some((r) => r.status === "EXECUTED");
  const unauthorizedFinding = reportAcc.findings?.some(
    (f) => f.code === "UNAUTHORIZED_EXECUTION"
  );
  const noDecision = reportAcc.findings?.some((f) => f.code === "NO_DECISION");

  // Validators would accept these forged records
  const vd = validateDecisionRecord(fakeDecision);
  const ve = validateExecutionRecord(fakeExec);

  if (hasExec && !unauthorizedFinding && !noDecision && vd.ok && ve.ok) {
    report(
      "A9",
      "Append-only audit + execution accountability",
      "appendAudit forged AUTHORIZED decision + EXECUTED without evaluate/resolve",
      "FAIL",
      `Audit accepts arbitrary JSON lines. Chain rebuilds forged EXECUTED as legitimate. Accountability report finds NO unauthorized execution. validateDecisionRecord/ExecutionRecord both ok=${vd.ok}/${ve.ok}. No signatures, HMACs, or write-path authN.`,
      "DOC-ONLY"
    );
  } else {
    report(
      "A9",
      "Audit integrity",
      "Raw append forgery",
      "PASS",
      "forgery detected or rejected",
      "REAL"
    );
  }
  try {
    fs.unlinkSync(log);
  } catch {
    /* ignore */
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A10: Same intent, different policy → different results (determinism scope)
// ─────────────────────────────────────────────────────────────────────────────
{
  const intent = baseIntent({
    action: "transfer_funds",
    data_classes: ["FINANCIAL"],
    risk: "high"
  });
  const p1 = policy;
  const p2 = { policy_pack_id: "permissive", rules: [] };
  const d1 = evaluateIntent(intent, p1);
  const d2 = evaluateIntent(intent, p2);

  // Same intent+policy twice must match
  const d1b = evaluateIntent(intent, p1);
  const deterministic =
    d1.decision === d1b.decision && d1.policy_id === d1b.policy_id;
  const policySwapChanges = d1.decision !== d2.decision;

  // Decision does NOT record policy snapshot hash — only policy_pack_id string
  const hasPolicyHash =
    d1.policy_hash || d1.policy_snapshot || d1.rules_digest || null;

  if (deterministic && policySwapChanges && !hasPolicyHash) {
    report(
      "A10",
      "Deterministic policy: same request + same Policy Snapshot ⇒ same decision",
      "Same intent under strict vs empty policy; check snapshot binding",
      "WARN",
      `Decision is deterministic for fixed policy (ok). Empty policy flips ${d1.decision}→${d2.decision}. But Decision Record only stores policy_pack_id="${d1.policy_pack_id}" with no policy content hash — cannot prove which rules produced the decision later. Replay attacks with swapped packs look identical on the chain.`,
      "PARTIAL"
    );
  } else if (!deterministic) {
    report(
      "A10",
      "Deterministic policy",
      "Same intent+policy twice",
      "FAIL",
      "non-deterministic outcomes",
      "DOC-ONLY"
    );
  } else {
    report(
      "A10",
      "Deterministic policy + snapshot binding",
      "Policy swap / hash",
      "PASS",
      "deterministic and snapshot bound",
      "REAL"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A11: Human declarer vs machine declarer edge cases
// ─────────────────────────────────────────────────────────────────────────────
{
  // Machine claims to be human
  const d1 = evaluateIntent(
    baseIntent({
      actor: "bot.gpt",
      actor_type: "human",
      action: "deploy",
      target: "production",
      declared_intent: "I am totally a human deploying"
    }),
    policy
  );

  // Human claims to be machine
  const d2 = evaluateIntent(
    baseIntent({
      actor: "alice@corp",
      actor_type: "machine",
      action: "deploy",
      target: "production",
      declared_intent: "human pretending to be machine"
    }),
    policy
  );

  // Operator is hardcoded human — try to pass machine-looking id (still type human)
  const op = resolveEscalation(d1, {
    operator_id: "service-account-not-a-human",
    outcome: "APPROVE"
  });

  // Forge operator_action with type machine via raw path — resolveEscalation hardcodes human
  // But recordExecution trusts any operator_action object:
  const fakeMachineOp = {
    authorization_status: "AUTHORIZED",
    parties: {
      authorizer: { id: "machine.auto_approver", type: "machine", outcome: "APPROVE" }
    },
    outcome: "APPROVE",
    operator: { id: "machine.auto_approver", type: "machine" }
  };
  const pending = evaluateIntent(baseIntent(), policy);
  const rFakeOp = threwValue(() =>
    recordExecution({
      decision: pending,
      operator_action: fakeMachineOp,
      executor_id: "runtime",
      status: "EXECUTED"
    })
  );

  if (
    d1.parties.declarer.type === "human" &&
    d1.parties.declarer.id === "bot.gpt" &&
    d2.parties.declarer.type === "machine" &&
    op.operator.type === "human" &&
    op.operator.id === "service-account-not-a-human" &&
    !rFakeOp.threw &&
    rFakeOp.value?.status === "EXECUTED"
  ) {
    report(
      "A11",
      "Human + machine first-class parties (accountability)",
      "Spoof actor_type; spoof operator_id; forge machine auto-approver into recordExecution",
      "FAIL",
      `actor_type is free self-assertion (bot.gpt as human accepted). operator_id unbound to authN (service-account recorded as type=human). Forged operator_action with AUTHORIZED bypasses PENDING without resolveEscalation — EXECUTED=${rFakeOp.value.status}. Party labels are decorative, not authenticated.`,
      "DOC-ONLY"
    );
  } else {
    report(
      "A11",
      "Human + machine parties",
      "Spoof types / forge operator_action",
      "PASS",
      "spoofing blocked",
      "REAL"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A12: Lie on declared fields to evade policy (intent is trusted)
// ─────────────────────────────────────────────────────────────────────────────
{
  // Real action is transfer_funds + PII, but attacker omits sensitive fields
  const lying = evaluateIntent(
    baseIntent({
      action: "summarize_report", // lie
      data_classes: [], // omit PII
      risk: "low", // lie
      declared_intent: "just summarizing"
    }),
    policy
  );
  const honest = evaluateIntent(
    baseIntent({
      action: "transfer_funds",
      data_classes: ["PII", "FINANCIAL"],
      risk: "high",
      declared_intent: "transfer all funds with customer PII"
    }),
    policy
  );

  if (lying.decision === "ALLOW" && honest.decision === "REQUIRE_APPROVAL") {
    report(
      "A12",
      "Intent declared before execution (semantic truth)",
      "Under-declare action/risk/data_classes to get ALLOW",
      "FAIL",
      `Lying intent → ${lying.decision}/${lying.authorization_status}; honest → ${honest.decision}. SPEC §1.2 admits this is out of scope — enforcement of declaration honesty is DOCUMENTATION-ONLY / non-goal. Gate is only as honest as the caller.`,
      "DOC-ONLY"
    );
  } else {
    report(
      "A12",
      "Semantic intent honesty",
      "Under-declare fields",
      "PASS",
      "unexpected: lying did not get ALLOW",
      "PARTIAL"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A13: operator_action injection bypass (forged APPROVE without resolveEscalation)
// ─────────────────────────────────────────────────────────────────────────────
{
  const d = evaluateIntent(baseIntent(), policy);
  const r = threwValue(() =>
    recordExecution({
      decision: d,
      operator_action: {
        // minimal forge — only field recordExecution reads for auth
        authorization_status: "AUTHORIZED"
      },
      executor_id: "runtime",
      status: "EXECUTED"
    })
  );

  if (!r.threw && r.value?.status === "EXECUTED" && d.authorization_status === "PENDING_HUMAN_APPROVAL") {
    report(
      "A13",
      "Cannot EXECUTED without AUTHORIZED (chain integrity)",
      "Inject minimal {authorization_status:AUTHORIZED} as operator_action",
      "FAIL",
      `PENDING decision executed via forged operator_action stub. recordExecution does not verify operator_action.linked_receipt_id, outcome, operator.type, or presence on audit log.`,
      "PARTIAL"
    );
  } else {
    report(
      "A13",
      "Operator action binding",
      "Forged operator_action stub",
      "PASS",
      r.err?.message || "blocked",
      "REAL"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A14: CLI path — double approve via API already covered; approve after REJECT still works
// ─────────────────────────────────────────────────────────────────────────────
{
  const log = tmpLog();
  const d = evaluateIntent(baseIntent(), policy, { auditPath: log });
  resolveEscalation(
    d,
    { operator_id: "human.a", outcome: "REJECT" },
    { auditPath: log }
  );
  // Second resolution APPROVE after REJECT
  const r = threwValue(() =>
    resolveEscalation(
      d,
      { operator_id: "human.b", outcome: "APPROVE" },
      { auditPath: log }
    )
  );
  let execOk = false;
  if (!r.threw) {
    const ex = threwValue(() =>
      recordExecution(
        {
          decision: d,
          operator_action: r.value,
          executor_id: "runtime",
          status: "EXECUTED"
        },
        { auditPath: log }
      )
    );
    execOk = !ex.threw && ex.value?.status === "EXECUTED";
  }

  if (!r.threw && execOk) {
    report(
      "A14",
      "REJECT is terminal / authorization finality",
      "REJECT then later APPROVE then EXECUTED",
      "FAIL",
      "Post-REJECT APPROVE accepted; EXECUTED succeeded. No terminal state machine on receipt. Latest operator_action wins if caller passes it.",
      "DOC-ONLY"
    );
  } else {
    report(
      "A14",
      "Authorization finality",
      "REJECT then APPROVE",
      "PASS",
      "terminal reject held",
      "REAL"
    );
  }
  try {
    fs.unlinkSync(log);
  } catch {
    /* ignore */
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A15: BLOCKED status allowed without auth — OK; FAILED without auth — OK
//      But FAILED with forged success narrative; also EXECUTED with ALLOW after mutating auth
// ─────────────────────────────────────────────────────────────────────────────
{
  const d = evaluateIntent(baseIntent(), policy);
  // Mutate decision.authorization_status in memory without operator
  d.authorization_status = "AUTHORIZED";
  const r = threwValue(() =>
    recordExecution({
      decision: d,
      executor_id: "runtime",
      status: "EXECUTED"
    })
  );

  if (!r.threw && r.value?.status === "EXECUTED" && d.decision === "REQUIRE_APPROVAL") {
    report(
      "A15",
      "Cannot EXECUTED without AUTHORIZED (integrity of status)",
      "Mutate decision.authorization_status PENDING→AUTHORIZED in memory",
      "FAIL",
      `recordExecution only checks authorization_status field, not consistency with decision===REQUIRE_APPROVAL and authorizer presence. In-memory tampering yields EXECUTED.`,
      "PARTIAL"
    );
  } else {
    report(
      "A15",
      "authorization_status integrity",
      "In-memory status flip",
      "PASS",
      r.err?.message || "blocked",
      "REAL"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A16: evaluateIntent without auditPath — no durable declaration
// ─────────────────────────────────────────────────────────────────────────────
{
  const d = evaluateIntent(
    baseIntent({ action: "summarize_report", data_classes: [], risk: "low" }),
    policy
    // no auditPath
  );
  const exec = recordExecution({
    decision: d,
    executor_id: "runtime",
    status: "EXECUTED"
  });
  // Claim: intent declared before execution as durable records
  if (d && exec.status === "EXECUTED") {
    report(
      "A16",
      "Intent declared before execution as durable Decision Records",
      "evaluate + execute with no auditPath — pure in-memory",
      "WARN",
      "API allows full evaluate→EXECUTED with zero durable records. Audit is optional (opts.auditPath). Conforming 'system' can be all-ephemeral; SPEC §8 MUST append-only sequence is deployment/integration choice, not library-enforced.",
      "PARTIAL"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// A17: receipt_id collision / predictability (no cryptographic binding)
// ─────────────────────────────────────────────────────────────────────────────
{
  const intentId = "fixed_intent_id";
  const at = new Date("2026-01-01T00:00:00.000Z");
  // receiptId is deterministic from intent_id + timestamp
  const { receiptId } = await import("../src/ids.mjs");
  const r1 = receiptId(intentId, at);
  const r2 = receiptId(intentId, at);
  if (r1 === r2 && r1.includes("fixed_intent_id")) {
    report(
      "A17",
      "Stable identifiers / chain integrity",
      "Predict receipt_id from intent_id + timestamp",
      "WARN",
      `receipt_id is deterministic and non-secret (${r1}). No hash of intent body or policy. Attacker who can write audit can pre-place records for future receipts.`,
      "PARTIAL"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Summary
// ─────────────────────────────────────────────────────────────────────────────
console.log("\n" + "═".repeat(64));
console.log("RESULTS TABLE");
console.log("═".repeat(64));

const pass = results.filter((r) => r.verdict === "PASS").length;
const fail = results.filter((r) => r.verdict === "FAIL").length;
const warn = results.filter((r) => r.verdict === "WARN").length;

console.log(
  "\n| ID  | Verdict | Enforcement | Attack |"
);
console.log("|-----|---------|-------------|--------|");
for (const r of results) {
  console.log(`| ${r.id.padEnd(3)} | ${r.verdict.padEnd(7)} | ${r.enforcement.padEnd(11)} | ${r.attack.slice(0, 70)} |`);
}

console.log(`\nTotals: ${pass} PASS (defense held), ${fail} FAIL (hole), ${warn} WARN`);

// Theory claim rollup
console.log("\n" + "═".repeat(64));
console.log("THEORY CLAIMS: REAL vs DOCUMENTATION-ONLY");
console.log("═".repeat(64));

const claims = {
  "1. Intent declared before execution": {
    real: "evaluateIntent builds structured request; original_intent snapshotted onto Decision Record when evaluate runs",
    doc: "No durability forced (audit optional); no immutability/hash of snapshot; caller can lie about fields (A12); forged decisions accepted (A1); post-hoc mutation of in-memory decision works (A6/A15)"
  },
  "2. Deterministic policy decisions (no LLM)": {
    real: "evaluateRules is pure first-match YAML rules; no LLM in path; same intent+policy ⇒ same decision/policy_id",
    doc: "Evaluator is new Function() — arbitrary JS, not a constrained DSL; no sandbox (A7); policy snapshot not hashed onto receipt (A10)"
  },
  "3. Cannot EXECUTED without AUTHORIZED": {
    real: "recordExecution throws if status===EXECUTED && authStatus!==AUTHORIZED for the objects you pass it (A2, A3)",
    doc: "authStatus is trusted from caller-supplied decision/operator_action with no verification (A13, A15); forged decisions work (A1); audit forgery works (A9); REJECT not terminal (A14)"
  },
  "4. Human + machine accountability on the chain": {
    real: "Records carry parties.declarer/evaluator/authorizer/executor with type labels; accountability report lists both kinds",
    doc: "Types and ids are self-asserted, not authenticated (A11); double-resolution allowed (A4); forged chains look accountable (A9)"
  },
  "5. Suitable as minimum standard for real apps": {
    real: "Useful contract + evidence shapes + conformance suite for cooperative integrations",
    doc: "SPEC itself admits bypass is a deployment property (§1.2). Library does not enforce gate on side effects — only on whether it will emit an EXECUTED *record*. Production trust requires OS/runtime wrapping, signed audit, authN for operators, sandboxed policy — none shipped."
  }
};

for (const [k, v] of Object.entries(claims)) {
  console.log(`\n${k}`);
  console.log(`  REAL ENFORCEMENT:     ${v.real}`);
  console.log(`  DOCUMENTATION-ONLY:   ${v.doc}`);
}

console.log("\n" + "═".repeat(64));
console.log("BRUTAL SUMMARY");
console.log("═".repeat(64));
console.log(`
Would I trust this in production tomorrow?

NO.

This is a cooperative bookkeeping layer with a few assert()-style guards,
not a control plane that can stop a hostile or merely buggy runtime.

What actually works (narrowly):
- If every side effect goes through recordExecution AND callers only pass
  objects produced by evaluateIntent/resolveEscalation AND nobody writes the
  audit file except the library AND policy YAML is trusted admin input —
  then PENDING/REJECTED paths cannot get an EXECUTED *receipt*.

What does not work:
- The gate does not wrap side effects. Nothing stops performing the action
  without calling the library at all.
- recordExecution is a JSON stamp factory. Feed it forged AUTHORIZED
  objects and it happily stamps EXECUTED (A1, A13, A15).
- appendAudit is world-writable truth with no MAC/signature (A9).
- Policy evaluation is eval-equivalent (new Function + with) (A7).
- Operator resolution has no single-outcome state machine (A4, A14).
- Party types are costume jewelry — self-asserted strings (A8, A11).
- Declared intent honesty is explicitly out of scope and trivially gamed (A12).

The SPEC is more honest than the marketing posture: §1.2 says a malicious
runtime can bypass; §13 says sandbox policy and protect audit. The reference
implementation implements the happy-path contract and the conformance suite
only probes cooperative cases (one negative test: EXECUTED while PENDING).

As a "minimum standard" for *interoperable evidence shapes* — fine, ship it
with giant warnings. As a *trust layer you put in front of money/PII/prod
deploys tomorrow* — it is theater unless the host runtime enforces the gate
outside this library.
`);

// Exit non-zero if any FAIL — red team found holes
process.exit(fail > 0 ? 1 : 0);
