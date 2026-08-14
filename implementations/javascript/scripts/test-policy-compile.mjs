/**
 * Phase 1 negative suite: strict policy compile and fail-closed load.
 * Sits beside the frozen TL-PX 0.1 conformance fixtures; does not rewrite them.
 */
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { fileURLToPath } from "node:url";
import {
  parsePolicyText,
  compilePolicy,
  compileExpression,
  evaluateCondition,
  evaluateRules,
  evaluateIntent,
  readPolicyFile
} from "../src/index.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

let n = 0;
function assert(c, m) {
  if (!c) throw new Error(m);
  n++;
  console.log(`  ok  ${m}`);
}

function throws(fn, snippet, msg) {
  let err;
  try {
    fn();
  } catch (e) {
    err = e;
  }
  assert(err instanceof Error, `${msg}: threw`);
  if (snippet) {
    assert(String(err.message).includes(snippet), `${msg}: message contains ${JSON.stringify(snippet)}`);
  }
}

function validRule(over = {}) {
  return {
    id: "rule_ok",
    description: "ok",
    if: 'risk == "high"',
    require: "human_approval",
    ...over
  };
}

function validPack(rules) {
  return { policy_pack_id: "test", rules };
}

console.log("Policy compile (Phase 1)\n");

{
  console.log("valid pack still evaluates");
  const policy = readPolicyFile(path.join(root, "config", "policy.yaml"));
  assert(policy.policy_pack_id === "default", "default pack loads");
  assert(compilePolicy(policy) === policy, "compile is idempotent");
  const safe = evaluateRules(
    { action: "summarize_report", risk: "low", data_classes: [], low_credibility: false },
    policy
  );
  assert(safe.decision === "ALLOW" && safe.policy_id === null, "valid pack no-match ALLOW");
  const pii = evaluateRules(
    { action: "send_email", risk: "medium", data_classes: ["PII"], low_credibility: false },
    policy
  );
  assert(pii.decision === "REQUIRE_APPROVAL" && pii.policy_id === "rule_pii_email", "valid pack still matches");
  assert(evaluateCondition('risk == "high"', { risk: "high" }) === true, "compiled eq true");
  assert(evaluateCondition('risk == "high"', { risk: "low" }) === false, "compiled eq false");
}

{
  console.log("known fail-open: risk ==");
  throws(() => compileExpression("risk =="), "expected value", "risk == rejected at compile");
  throws(
    () => compilePolicy(validPack([validRule({ if: "risk ==" })])),
    "expected value",
    "pack with risk == rejected"
  );
  throws(
    () => evaluateCondition("risk ==", { risk: "low" }),
    "expected value",
    "evaluateCondition throws instead of non-match"
  );
  throws(
    () => evaluateRules({ risk: "low" }, validPack([validRule({ if: "risk ==" })])),
    "expected value",
    "evaluateRules does not fall through to ALLOW"
  );
}

{
  console.log("malformed expressions");
  throws(() => compileExpression('risk == "low" || this.constructor'), "illegal character", "|| injection");
  throws(() => compileExpression("__proto__ == \"x\""), "unknown field", "__proto__ field");
  throws(() => compileExpression("constructor == \"x\""), "unknown field", "constructor field");
  throws(() => compileExpression('foobar == "x"'), "unknown field", "unknown expression field");
  throws(() => compileExpression("risk > 1"), "illegal character", "unsupported operator");
  throws(() => compileExpression('(risk == "high"'), "expected )", "unmatched open paren");
  throws(() => compileExpression('risk == "high'), "unterminated string", "unterminated string");
  throws(() => compileExpression(""), "missing condition", "empty condition");
  throws(() => compileExpression('"PII" in risk'), "array field", "in on non-array field");
  throws(() => compileExpression("risk"), "expected == or !=", "bare identifier");
}

{
  console.log("pack structure");
  throws(() => compilePolicy({ policy_pack_id: "x", rules: [] }), "empty policy pack", "empty pack");
  throws(() => compilePolicy({ rules: [validRule()] , extra: true }), "unknown pack field", "unknown pack field");
  throws(() => compilePolicy({ policy_pack_id: "x" }), "rules must be an array", "missing rules array");
  throws(
    () => compilePolicy(validPack([validRule({ extra: "nope" })])),
    "unknown field",
    "unknown rule field"
  );
  throws(
    () => compilePolicy(validPack([validRule({ if: undefined, id: "missing_if" })])),
    "missing condition",
    "missing if"
  );
  throws(
    () => compilePolicy(validPack([{ id: "no_effect", if: 'risk == "high"' }])),
    "has no effect",
    "missing effect"
  );
  throws(
    () =>
      compilePolicy(
        validPack([
          validRule({ id: "dup" }),
          validRule({ id: "dup", if: 'action == "deploy"' })
        ])
      ),
    "duplicate rule id",
    "duplicate ids"
  );
  throws(
    () => compilePolicy(validPack([validRule({ require: "rubber_stamp" })])),
    "invalid require",
    "invalid require enum"
  );
  throws(
    () => compilePolicy(validPack([validRule({ require: undefined, deny: false, id: "bad_deny" })])),
    "invalid deny",
    "deny false is not an effect"
  );
  throws(
    () => compilePolicy(validPack([{ id: 1, if: 'risk == "high"', require: "human_approval" }])),
    "missing id",
    "numeric id"
  );
  throws(
    () =>
      compilePolicy(
        validPack([
          validRule({ id: "good" }),
          validRule({ id: "bad", if: "risk ==" })
        ])
      ),
    "expected value",
    "one bad rule rejects the pack"
  );
}

{
  console.log("YAML parse");
  throws(() => parsePolicyText("not yaml"), "unsupported structure", "garbage text");
  throws(() => parsePolicyText("- id: orphan\n  if: risk == \"high\"\n"), "rule before rules", "missing rules key");
  throws(
    () => parsePolicyText("rules:\n  - id: x\n    if: risk == \"high\"\n    mystery: 1\n    require: human_approval\n"),
    "unknown field",
    "unknown YAML field"
  );
  throws(
    () => parsePolicyText("rules:\n  - id: x\n    if: risk == \"high\"\n    require: human_approval\n  surprise\n"),
    "unsupported structure",
    "unmatched YAML line"
  );
  const parsed = parsePolicyText(`
rules:
  - id: rule_high
    description: high risk
    if: risk == "high"
    require: human_approval
`);
  const compiled = compilePolicy(parsed);
  assert(compiled.rules[0].id === "rule_high", "YAML compile");
}

{
  console.log("deny:true remains escalate");
  const out = evaluateRules(
    { risk: "high" },
    validPack([{ id: "legacy_deny", if: 'risk == "high"', deny: true }])
  );
  assert(out.decision === "REQUIRE_APPROVAL" && out.policy_id === "legacy_deny", "deny:true escalates");
}

{
  console.log("evaluateIntent issues no authorization");
  const log = path.join(os.tmpdir(), `pc-${Date.now()}-${Math.random().toString(16).slice(2)}.jsonl`);
  throws(
    () =>
      evaluateIntent(
        {
          intent_id: "itest",
          actor: "agent.docs.summarizer",
          actor_type: "machine",
          declared_intent: "safe",
          action: "summarize_report",
          risk: "low"
        },
        validPack([validRule({ if: "risk ==" })]),
        { auditPath: log }
      ),
    "expected value",
    "evaluateIntent rejects malformed pack"
  );
  assert(!fs.existsSync(log), "no decision record written");
}

{
  console.log("load/startup validation");
  const tmpYaml = path.join(os.tmpdir(), `pc-${Date.now()}.yaml`);
  fs.writeFileSync(
    tmpYaml,
    `rules:\n  - id: broken\n    if: risk ==\n    require: human_approval\n`
  );
  throws(() => readPolicyFile(tmpYaml), "expected value", "readPolicyFile fails closed");
  fs.unlinkSync(tmpYaml);
  const good = readPolicyFile(path.join(root, "config", "policy.yaml"));
  assert(Array.isArray(good.rules) && good.rules.length > 0, "good pack still loads");
}

console.log(`\nPolicy compile tests passed (${n}).`);
