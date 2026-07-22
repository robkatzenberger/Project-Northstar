import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { fileURLToPath } from "node:url";
import {
  createPrismSignal,
  toEvaluationIntent,
  evaluateIntent,
  recordExecution,
  readPolicyFile,
  loadSwitchboard
} from "../src/index.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const policy = readPolicyFile(path.join(root, "config", "policy.yaml"));
const sb = loadSwitchboard(path.join(root, "config", "switchboard.json"));

let n = 0;
function assert(cond, msg) {
  if (!cond) throw new Error(msg);
  n += 1;
  console.log(`  ok  ${msg}`);
}

function tmp() {
  return path.join(os.tmpdir(), `sb-${Date.now()}-${Math.random().toString(16).slice(2)}.jsonl`);
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

function ev(fileOrObj, log) {
  return evaluateIntent(intent(fileOrObj), policy, { switchboard: sb, auditPath: log });
}

console.log("Switchboard tests\n");

{
  console.log("high trust allow");
  const log = tmp();
  const d = ev("intent-safe.json", log);
  assert(d.decision === "ALLOW", "docs summarizer ALLOW");
  assert(d.switchboard.credibility === 0.91, "credibility 0.91");
  assert(d.switchboard.credibility_band === "high", "high band");
  assert(d.switchboard.whitelisted === true, "whitelisted");
  assert(d.parties.router?.id === "local-switchboard-v0", "router on parties");
  fs.unlinkSync(log);
}

{
  console.log("medium cred pii route");
  const log = tmp();
  const d = ev("intent-pii-email.json", log);
  assert(d.decision === "REQUIRE_APPROVAL", "pii still escalates");
  assert(d.switchboard.credibility === 0.62, "mailer cred");
  assert(d.approval_route[0] === "human.ops.alex", "route starts with ops.alex");
  assert(d.approval_route.includes("human.support.lead"), "support lead on route");
  fs.unlinkSync(log);
}

{
  console.log("low credibility finance");
  const log = tmp();
  const d = ev("intent-funds.json", log);
  assert(d.decision === "REQUIRE_APPROVAL", "finance escalates");
  assert(d.switchboard.credibility_band === "low", "low band");
  assert(d.switchboard.flags.includes("LOW_CREDIBILITY"), "LOW_CREDIBILITY flag");
  assert(d.approval_route[0] === "human.finance.sam", "finance route");
  assert(d.policy_id === "rule_low_credibility", `policy_id low cred got ${d.policy_id}`);
  fs.unlinkSync(log);
}

{
  console.log("unknown deny");
  const log = tmp();
  const d = ev("intent-unknown-agent.json", log);
  assert(d.decision === "DENY", "unknown DENY");
  assert(d.authorization_status === "DENIED", "DENIED status");
  assert(d.policy_id === "switchboard.unknown_deny", "unknown_deny id");
  let threw = false;
  try {
    recordExecution(
      { receipt_id: d.receipt_id, executor_id: "rt", status: "EXECUTED" },
      { auditPath: log }
    );
  } catch {
    threw = true;
  }
  assert(threw, "cannot execute after DENY");
  fs.unlinkSync(log);
}

{
  console.log("not whitelisted");
  const log = tmp();
  const d = ev("intent-not-whitelisted.json", log);
  assert(d.decision === "DENY", "not whitelisted DENY");
  assert(d.policy_id === "switchboard.not_whitelisted", "not_whitelisted id");
  fs.unlinkSync(log);
}

{
  console.log("action not permitted");
  const log = tmp();
  const d = ev(
    {
      agent: "agent.docs.summarizer",
      actor_type: "machine",
      intent_summary: "Summarizer tries to move money",
      action: "transfer_funds",
      target: "external_account",
      risk: "high",
      data_classes: ["FINANCIAL"]
    },
    log
  );
  assert(d.decision === "DENY", "action denied by switchboard");
  assert(d.policy_id === "switchboard.action_denied", "action_denied");
  fs.unlinkSync(log);
}

{
  console.log("no switchboard fallback");
  const log = tmp();
  const d = evaluateIntent(intent("intent-unknown-agent.json"), policy, { auditPath: log });
  assert(d.decision === "ALLOW", "without switchboard unknown can ALLOW");
  assert(d.switchboard === null, "no switchboard block");
  fs.unlinkSync(log);
}

console.log(`\nSwitchboard tests passed (${n} assertions).`);
