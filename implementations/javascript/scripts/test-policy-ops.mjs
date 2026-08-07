/**
 * A7 safe policy parser + A18 operator allowlist tests
 */
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { fileURLToPath } from "node:url";
import {
  evaluateCondition,
  evaluateRules,
  evaluateIntent,
  resolveEscalation,
  createPrismSignal,
  toEvaluationIntent,
  readPolicyFile,
  loadSwitchboard
} from "../src/index.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const policy = readPolicyFile(path.join(root, "config", "policy.yaml"));
const sb = loadSwitchboard(path.join(root, "config", "switchboard.json"));

let n = 0;
function assert(c, m) {
  if (!c) throw new Error(m);
  n++;
  console.log(`  ok  ${m}`);
}
function tmp() {
  return path.join(os.tmpdir(), `po-${Date.now()}-${Math.random().toString(16).slice(2)}.jsonl`);
}

console.log("Policy parser + operators\n");

{
  console.log("safe expressions");
  assert(evaluateCondition('risk == "high"', { risk: "high" }) === true, "eq true");
  assert(evaluateCondition('risk == "high"', { risk: "low" }) === false, "eq false");
  assert(evaluateCondition('"PII" in data_classes', { data_classes: ["PII"] }) === true, "in true");
  assert(
    evaluateCondition('action == "send_email" and "PII" in data_classes', {
      action: "send_email",
      data_classes: ["PII"]
    }) === true,
    "and combo"
  );
  assert(evaluateCondition('low_credibility == true', { low_credibility: true }) === true, "bool true");
  assert(evaluateCondition('risk == "low" || this.constructor', { risk: "low" }) === false, "inject rejected");
  assert(evaluateCondition("__proto__ == \"x\"", {}) === false, "proto inject rejected");
}

{
  console.log("policy fixtures");
  const safe = evaluateRules(
    { action: "summarize_report", risk: "low", data_classes: [], low_credibility: false },
    policy
  );
  assert(safe.decision === "ALLOW", "safe allow");
  const pii = evaluateRules(
    { action: "send_email", risk: "medium", data_classes: ["PII"], low_credibility: false },
    policy
  );
  assert(pii.decision === "REQUIRE_APPROVAL" && pii.policy_id === "rule_pii_email", "pii rule");
}

{
  console.log("operator route");
  const log = tmp();
  const intent = toEvaluationIntent(
    createPrismSignal(JSON.parse(fs.readFileSync(path.join(root, "examples/intent-pii-email.json"), "utf8")))
  );
  const d = evaluateIntent(intent, policy, { auditPath: log, switchboard: sb });
  let threw = false;
  try {
    resolveEscalation(d, { operator_id: "human.ceo.impostor", outcome: "APPROVE" }, {
      auditPath: log,
      switchboard: sb
    });
  } catch {
    threw = true;
  }
  assert(threw, "impostor blocked");
  const op = resolveEscalation(d, { operator_id: "human.ops.alex", outcome: "APPROVE" }, {
    auditPath: log,
    switchboard: sb
  });
  assert(op.outcome === "APPROVE", "route operator allowed");
  fs.unlinkSync(log);
}

console.log(`\nPolicy/ops tests passed (${n}).`);
