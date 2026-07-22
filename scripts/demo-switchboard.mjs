/**
 * Switchboard demo: whitelist, credibility, approval routes.
 */
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  createPrismSignal,
  toEvaluationIntent,
  evaluateIntent,
  readPolicyFile,
  loadSwitchboard
} from "../src/index.mjs";
import fs from "node:fs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const policy = readPolicyFile(path.join(root, "config", "policy.yaml"));
const sb = loadSwitchboard(path.join(root, "config", "switchboard.json"));

function run(label, file) {
  const raw = JSON.parse(fs.readFileSync(path.join(root, "examples", file), "utf8"));
  const d = evaluateIntent(toEvaluationIntent(createPrismSignal(raw)), policy, {
    switchboard: sb
  });
  console.log(`\n▸ ${label}`);
  console.log(
    JSON.stringify(
      {
        actor: d.original_intent.actor,
        decision: d.decision,
        reason: d.reason,
        policy_id: d.policy_id,
        authorization_status: d.authorization_status,
        credibility: d.switchboard?.credibility,
        band: d.switchboard?.credibility_band,
        whitelisted: d.switchboard?.whitelisted,
        flags: d.switchboard?.flags,
        approval_route: d.approval_route
      },
      null,
      2
    )
  );
}

console.log("════════════════════════════════════════");
console.log(" Switchboard demo (identity + credibility)");
console.log("════════════════════════════════════════");

run("High trust summarizer (0.91)", "intent-safe.json");
run("Medium mailer + PII (0.62) → route to ops", "intent-pii-email.json");
run("Low credibility finance (0.35) → finance route", "intent-funds.json");
run("Unknown agent → DENY", "intent-unknown-agent.json");
run("Not whitelisted → DENY", "intent-not-whitelisted.json");

console.log("\nDone. Config: config/switchboard.json");
