#!/usr/bin/env node

import fs from "node:fs";
import { canonicalize } from "../src/jcs.mjs";
import { validateV02 } from "../src/validate-v02.mjs";

const input = process.argv[2];
if (!input || process.argv.length !== 3) {
  console.error("usage: node scripts/validate-pep-evidence.mjs EVIDENCE_JSONL");
  process.exit(2);
}

const kindByType = new Map([
  ["tlpx.decision", "decision"],
  ["tlpx.evaluation_error", "evaluation-error"],
  ["tlpx.operator_action", "operator-action"],
  ["tlpx.authorization", "authorization"],
  ["tlpx.authorization_claim", "authorization-claim"],
  ["tlpx.execution", "execution"]
]);
const lines = fs.readFileSync(input, "utf8").trim().split("\n").filter(Boolean);
let failures = 0;
let denyDecisions = 0;
let completedExecutions = 0;
let claims = 0;
const records = [];

console.log("Restricted PEP evidence crosscheck\n");
for (const [index, line] of lines.entries()) {
  let record;
  try {
    record = JSON.parse(line);
  } catch {
    console.log(`  FAIL  row ${index + 1}: invalid JSON`);
    failures += 1;
    continue;
  }
  const kind = kindByType.get(record.record_type);
  if (!kind) {
    console.log(`  FAIL  row ${index + 1}: unexpected ${record.record_type}`);
    failures += 1;
    continue;
  }
  const validation = validateV02(kind, record);
  const canonical = canonicalize(record) === line;
  const envelopeLeak = [
    "record_hash",
    "previous_chain_hash",
    "chain_hash",
    "seal_algorithm",
    "seal_key_id",
    "seal"
  ].some((field) => field in record);
  if (!validation.ok || !canonical || envelopeLeak) {
    console.log(`  FAIL  ${record.record_type}`);
    for (const error of validation.errors) console.log(`        ${error}`);
    if (!canonical) console.log("        record is not canonical JCS");
    if (envelopeLeak) console.log("        storage-envelope metadata leaked into record");
    failures += 1;
    continue;
  }
  if (record.record_type === "tlpx.decision" && record.decision === "DENY") denyDecisions += 1;
  if (record.record_type === "tlpx.authorization_claim") claims += 1;
  if (record.record_type === "tlpx.execution" && record.state === "COMPLETED") completedExecutions += 1;
  records.push(record);
  console.log(`  PASS  ${record.record_type}`);
}

const authorization = records.find((record) => record.record_type === "tlpx.authorization");
const claim = records.find((record) => record.record_type === "tlpx.authorization_claim");
const execution = records.find((record) => record.record_type === "tlpx.execution");
const allowDecision = records.find(
  (record) => record.record_type === "tlpx.decision" && record.decision === "ALLOW"
);
const identityAndBindingHold = authorization && claim && execution && allowDecision &&
  authorization.requesting_principal === "restricted.agent" &&
  authorization.executing_principal === "restricted.agent" &&
  claim.executing_principal === "restricted.agent" &&
  execution.requesting_principal === "restricted.agent" &&
  execution.executing_principal === "restricted.agent" &&
  allowDecision.parties.requester.id === "restricted.agent" &&
  authorization.action === "shell.exec" &&
  authorization.adapter.id === "adapter.restricted-marker" &&
  claim.adapter.id === "adapter.restricted-marker" &&
  execution.adapter.id === "adapter.restricted-marker" &&
  execution.adapter_principal === "restricted.pep.adapter" &&
  authorization.authorization_id === claim.authorization_id &&
  claim.authorization_id === execution.authorization_id &&
  claim.claim_id === execution.claim_id &&
  authorization.intent_hash === allowDecision.intent_hash &&
  authorization.intent_hash === execution.intent_hash &&
  authorization.authorized_action_hash === claim.authorized_action_hash &&
  claim.authorized_action_hash === execution.authorized_action_hash &&
  claim.executed_action_hash === execution.executed_action_hash;
if (!identityAndBindingHold) {
  console.log("  FAIL  authenticated principal, adapter, identifier, or hash linkage mismatch");
  failures += 1;
}

for (const [label, count] of [
  ["DENY decision", denyDecisions],
  ["authorization claim", claims],
  ["COMPLETED execution", completedExecutions]
]) {
  if (count !== 1) {
    console.log(`  FAIL  expected exactly one ${label}, got ${count}`);
    failures += 1;
  }
}

console.log(`\nRestricted PEP evidence: ${lines.length} rows, ${failures} failures`);
if (failures) process.exit(1);
