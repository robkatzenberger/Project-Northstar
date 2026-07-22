import fs from "node:fs";
import path from "node:path";
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

console.log("Switchboard tests\n");

{
  console.log("high trust allow");
  const d = evaluateIntent(intent("intent-safe.json"), policy, { switchboard: sb });
  assert(d.decision === "ALLOW", "docs summarizer ALLOW");
  assert(d.switchboard.credibility === 0.91, "credibility 0.91");
  assert(d.switchboard.credibility_band === "high", "high band");
  assert(d.switchboard.whitelisted === true, "whitelisted");
  assert(d.parties.router?.id === "local-switchboard-v0", "router on parties");
}

{
  console.log("medium cred pii route");
  const d = evaluateIntent(intent("intent-pii-email.json"), policy, { switchboard: sb });
  assert(d.decision === "REQUIRE_APPROVAL", "pii still escalates");
  assert(d.switchboard.credibility === 0.62, "mailer cred");
  assert(d.approval_route[0] === "human.ops.alex", "route starts with ops.alex");
  assert(d.approval_route.includes("human.support.lead"), "support lead on route");
}

{
  console.log("low credibility finance");
  const d = evaluateIntent(intent("intent-funds.json"), policy, { switchboard: sb });
  assert(d.decision === "REQUIRE_APPROVAL", "finance escalates");
  assert(d.switchboard.credibility_band === "low", "low band");
  assert(d.switchboard.flags.includes("LOW_CREDIBILITY"), "LOW_CREDIBILITY flag");
  assert(d.approval_route[0] === "human.finance.sam", "finance route");
  assert(d.policy_id === "rule_low_credibility", `policy_id low cred got ${d.policy_id}`);
}

{
  console.log("unknown deny");
  const d = evaluateIntent(intent("intent-unknown-agent.json"), policy, { switchboard: sb });
  assert(d.decision === "DENY", "unknown DENY");
  assert(d.authorization_status === "DENIED", "DENIED status");
  assert(d.policy_id === "switchboard.unknown_deny", "unknown_deny id");
  let threw = false;
  try {
    recordExecution({ decision: d, executor_id: "rt", status: "EXECUTED" });
  } catch {
    threw = true;
  }
  assert(threw, "cannot execute after DENY");
}

{
  console.log("not whitelisted");
  const d = evaluateIntent(intent("intent-not-whitelisted.json"), policy, { switchboard: sb });
  assert(d.decision === "DENY", "not whitelisted DENY");
  assert(d.policy_id === "switchboard.not_whitelisted", "not_whitelisted id");
}

{
  console.log("action not permitted");
  const d = evaluateIntent(
    intent({
      agent: "agent.docs.summarizer",
      actor_type: "machine",
      intent_summary: "Summarizer tries to move money",
      action: "transfer_funds",
      target: "external_account",
      risk: "high",
      data_classes: ["FINANCIAL"]
    }),
    policy,
    { switchboard: sb }
  );
  assert(d.decision === "DENY", "action denied by switchboard");
  assert(d.policy_id === "switchboard.action_denied", "action_denied");
}

{
  console.log("no switchboard fallback");
  const d = evaluateIntent(intent("intent-unknown-agent.json"), policy);
  assert(d.decision === "ALLOW", "without switchboard unknown can ALLOW");
  assert(d.switchboard === null, "no switchboard block");
}

console.log(`\nSwitchboard tests passed (${n} assertions).`);
