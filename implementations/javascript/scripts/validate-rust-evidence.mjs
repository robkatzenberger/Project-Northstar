#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { canonicalize } from "../src/jcs.mjs";
import { validateV02 } from "../src/validate-v02.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const rustDir = path.resolve(here, "../../rust");
const run = spawnSync(
  "cargo",
  [
    "run",
    "--quiet",
    "--offline",
    "--example",
    "local_authority",
    "--",
    ":memory:",
    "rust-schema-crosscheck",
    "--evidence-jsonl"
  ],
  { cwd: rustDir, encoding: "utf8" }
);

if (run.error) throw run.error;
if (run.status !== 0) {
  process.stderr.write(run.stderr);
  process.exit(run.status ?? 1);
}

const kindByType = new Map([
  ["tlpx.decision", "decision"],
  ["tlpx.evaluation_error", "evaluation-error"],
  ["tlpx.operator_action", "operator-action"],
  ["tlpx.authorization", "authorization"],
  ["tlpx.authorization_claim", "authorization-claim"]
]);
const lines = run.stdout.trim().split("\n").filter(Boolean);
const seen = new Set();
let approvals = 0;
let failures = 0;

console.log("Rust TL-PX 0.2 runtime-evidence schema crosscheck\n");
for (const [index, line] of lines.entries()) {
  const record = JSON.parse(line);
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
  seen.add(record.record_type);
  if (record.record_type === "tlpx.operator_action" && record.outcome === "APPROVE") {
    approvals += 1;
  }
  console.log(`  PASS  ${record.record_type}`);
}

for (const expected of kindByType.keys()) {
  if (!seen.has(expected)) {
    console.log(`  FAIL  missing ${expected}`);
    failures += 1;
  }
}
if (approvals !== 1) {
  console.log(`  FAIL  expected one schema-valid APPROVE record, got ${approvals}`);
  failures += 1;
}
if (lines.length !== 9) {
  console.log(`  FAIL  expected 9 bounded records, got ${lines.length}`);
  failures += 1;
}

console.log(`\nRuntime evidence: ${lines.length - failures} checked, ${failures} failed`);
if (failures) process.exit(1);
