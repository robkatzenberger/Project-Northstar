/**
 * Air-gap hardening tests: state machine, chain auth, fail-closed executor, anti-forgery.
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
  appendAudit,
  verifyAudit,
  resolveAuthorizationFromAudit,
  executeAuthorized
} from "../src/index.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const policy = readPolicyFile(path.join(root, "config", "policy.yaml"));

let n = 0;
function assert(cond, msg) {
  if (!cond) throw new Error(`FAIL: ${msg}`);
  n += 1;
  console.log(`  ok  ${msg}`);
}

function tmp() {
  return path.join(os.tmpdir(), `airgap-${Date.now()}-${Math.random().toString(16).slice(2)}.jsonl`);
}

function intent(file) {
  return toEvaluationIntent(
    createPrismSignal(JSON.parse(fs.readFileSync(path.join(root, "examples", file), "utf8")))
  );
}

console.log("Air-gap hardening tests\n");

// H1: forged in-memory AUTHORIZED cannot execute
{
  console.log("H1 forged decision object");
  const log = tmp();
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, { auditPath: log });
  const forged = {
    ...d,
    authorization_status: "AUTHORIZED",
    decision: "ALLOW"
  };
  let threw = false;
  try {
    recordExecution(
      { decision: forged, receipt_id: d.receipt_id, executor_id: "evil", status: "EXECUTED" },
      { auditPath: log }
    );
  } catch (e) {
    threw = e.message.includes("PENDING") || e.message.includes("cannot EXECUTED");
  }
  assert(threw, "forged AUTHORIZED on pending receipt rejected");
  fs.unlinkSync(log);
}

// H2: stub operator_action ignored — audit wins
{
  console.log("H2 stub operator ignored");
  const log = tmp();
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, { auditPath: log });
  let threw = false;
  try {
    recordExecution(
      {
        receipt_id: d.receipt_id,
        decision: d,
        operator_action: { authorization_status: "AUTHORIZED" },
        executor_id: "evil",
        status: "EXECUTED"
      },
      { auditPath: log }
    );
  } catch {
    threw = true;
  }
  assert(threw, "stub operator_action cannot authorize");
  fs.unlinkSync(log);
}

// H3: double-resolve blocked
{
  console.log("H3 single operator outcome");
  const log = tmp();
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, { auditPath: log });
  resolveEscalation(d, { operator_id: "h1", outcome: "APPROVE" }, { auditPath: log });
  let threw = false;
  try {
    resolveEscalation(d, { operator_id: "h2", outcome: "REJECT" }, { auditPath: log });
  } catch (e) {
    threw = e.message.includes("already resolved");
  }
  assert(threw, "double-resolve throws");
  fs.unlinkSync(log);
}

// H4: REJECT is terminal — cannot later APPROVE
{
  console.log("H4 reject terminal");
  const log = tmp();
  const d = evaluateIntent(intent("intent-funds.json"), policy, { auditPath: log });
  resolveEscalation(d, { operator_id: "h1", outcome: "REJECT" }, { auditPath: log });
  let threw = false;
  try {
    resolveEscalation(d, { operator_id: "h1", outcome: "APPROVE" }, { auditPath: log });
  } catch {
    threw = true;
  }
  assert(threw, "cannot approve after reject");
  const auth = resolveAuthorizationFromAudit(log, d.receipt_id);
  assert(auth.authorization_status === "DENIED", "still DENIED");
  assert(auth.terminal === true, "terminal");
  fs.unlinkSync(log);
}

// H5: sealed append of fake EXECUTED does not make auth AUTHORIZED (still pending)
{
  console.log("H5 auth from chain not forged execution");
  const log = tmp();
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, { auditPath: log });
  appendAudit(log, {
    record_type: "tlpx.execution",
    receipt_id: d.receipt_id,
    status: "EXECUTED",
    authorization_status: "AUTHORIZED",
    executor: { id: "forger", type: "machine" }
  });
  const auth = resolveAuthorizationFromAudit(log, d.receipt_id);
  assert(auth.authorization_status === "PENDING_HUMAN_APPROVAL", "forged execution does not authorize");
  const v = verifyAudit(log);
  assert(v.ok === true, "sealed chain still verifies");
  fs.unlinkSync(log);
}

// H5b: raw unsealed forge breaks integrity (A9)
{
  console.log("H5b raw forge breaks verify");
  const log = tmp();
  evaluateIntent(intent("intent-safe.json"), policy, { auditPath: log });
  fs.appendFileSync(
    log,
    JSON.stringify({
      record_type: "tlpx.decision",
      receipt_id: "rcpt_raw_forge",
      decision: "ALLOW",
      authorization_status: "AUTHORIZED"
    }) + "\n"
  );
  const v = verifyAudit(log);
  assert(v.ok === false, "verify fails on unsealed line");
  let threw = false;
  try {
    readAudit(log);
  } catch {
    threw = true;
  }
  assert(threw, "readAudit fail-closed on broken chain");
  fs.unlinkSync(log);
}

// H6: fail-closed executor
{
  console.log("H6 executeAuthorized fail-closed");
  const log = tmp();
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, { auditPath: log });
  let ran = false;
  const blocked = await executeAuthorized({
    auditPath: log,
    receipt_id: d.receipt_id,
    executor_id: "rt",
    sideEffect: () => {
      ran = true;
      return "should-not-run";
    }
  });
  assert(blocked.ok === false, "executor ok=false when pending");
  assert(ran === false, "sideEffect did not run");
  assert(blocked.execution?.status === "BLOCKED", "BLOCKED recorded");

  resolveEscalation(d, { operator_id: "h1", outcome: "APPROVE" }, { auditPath: log });
  const ok = await executeAuthorized({
    auditPath: log,
    receipt_id: d.receipt_id,
    executor_id: "rt",
    sideEffect: () => {
      ran = true;
      return "wire-transfer-sim";
    }
  });
  assert(ok.ok === true, "executor ok after approve");
  assert(ran === true, "sideEffect ran after authorize");
  assert(ok.execution.status === "EXECUTED", "EXECUTED recorded");
  fs.unlinkSync(log);
}

// H7: DENY path fail-closed
{
  console.log("H7 deny fail-closed");
  const log = tmp();
  // ephemeral-style deny via policy+switchboard would need sb; use funds reject
  const d = evaluateIntent(intent("intent-funds.json"), policy, { auditPath: log });
  resolveEscalation(d, { operator_id: "h1", outcome: "REJECT" }, { auditPath: log });
  let ran = false;
  const r = await executeAuthorized({
    auditPath: log,
    receipt_id: d.receipt_id,
    executor_id: "rt",
    sideEffect: () => {
      ran = true;
    }
  });
  assert(r.ok === false && !ran, "rejected never runs sideEffect");
  fs.unlinkSync(log);
}

// H8: receipt ids unique under burst
{
  console.log("H8 receipt uniqueness");
  const log = tmp();
  const ids = new Set();
  for (let i = 0; i < 20; i++) {
    const d = evaluateIntent(intent("intent-safe.json"), policy, { auditPath: log });
    ids.add(d.receipt_id);
  }
  assert(ids.size === 20, "20 unique receipt ids");
  fs.unlinkSync(log);
}

// H9: ALLOW path executor
{
  console.log("H9 allow executor");
  const log = tmp();
  const d = evaluateIntent(intent("intent-safe.json"), policy, { auditPath: log });
  const r = await executeAuthorized({
    auditPath: log,
    receipt_id: d.receipt_id,
    executor_id: "rt",
    sideEffect: () => "summarized"
  });
  assert(r.ok && r.result === "summarized", "allow path executes");
  fs.unlinkSync(log);
}

console.log(`\nAir-gap tests passed (${n} assertions).`);
