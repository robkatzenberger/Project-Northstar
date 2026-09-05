#!/usr/bin/env node

import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { canonicalize } from "../src/jcs.mjs";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const validator = path.join(scriptDirectory, "validate-pep-evidence.mjs");
const fixture = path.resolve(
  scriptDirectory,
  "../../../tests/reports/slice-3.9-administrator-gate-2026-09-04-064717.evidence.jsonl"
);
const testedBinaryHash =
  "sha256:7c483a6eb95e4defd1ad4b2999cd23b520c90324450bbc7222565145768f5470";
const wrongBinaryHash = `sha256:${"0".repeat(64)}`;
const original = fs
  .readFileSync(fixture, "utf8")
  .trimEnd()
  .split("\n")
  .map((line) => JSON.parse(line));
const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "northstar-pep-evidence-test-"));

function run(input, binaryHash = testedBinaryHash) {
  return spawnSync(process.execPath, [validator, input, binaryHash], {
    cwd: path.dirname(validator),
    encoding: "utf8"
  });
}

function writeRows(name, rows) {
  const output = path.join(temporary, name);
  fs.writeFileSync(output, `${rows.map((record) => canonicalize(record)).join("\n")}\n`);
  return output;
}

function expectFailure(label, result) {
  assert.notEqual(
    result.status,
    0,
    `${label} unexpectedly passed\n${result.stdout}${result.stderr}`
  );
}

try {
  const positive = run(fixture);
  assert.equal(positive.status, 0, `${positive.stdout}${positive.stderr}`);

  expectFailure("extra ALLOW row", run(writeRows("extra-allow.jsonl", [
    original[0],
    original[1],
    original[1],
    ...original.slice(2)
  ])));
  expectFailure("extra authorization row", run(writeRows("extra-authorization.jsonl", [
    ...original.slice(0, 3),
    original[2],
    ...original.slice(3)
  ])));
  expectFailure("reordered rows", run(writeRows("reordered.jsonl", [
    original[1],
    original[0],
    ...original.slice(2)
  ])));
  expectFailure("wrong expected binary", run(fixture, wrongBinaryHash));

  const wrongReceipt = structuredClone(original);
  wrongReceipt[4].receipt_id = "rcpt_00000000000000000000000000000000";
  expectFailure("receipt mismatch", run(writeRows("wrong-receipt.jsonl", wrongReceipt)));

  const wrongPolicy = structuredClone(original);
  wrongPolicy[4].policy_bundle_hash = wrongBinaryHash;
  expectFailure("policy mismatch", run(writeRows("wrong-policy.jsonl", wrongPolicy)));

  const wrongBinding = structuredClone(original);
  wrongBinding[3].action_binding_hash = wrongBinaryHash;
  expectFailure("action binding mismatch", run(writeRows("wrong-binding.jsonl", wrongBinding)));

  const wrongExecutionAuthorization = structuredClone(original);
  wrongExecutionAuthorization[4].authorized_action_hash = wrongBinaryHash;
  expectFailure(
    "execution authorization mismatch",
    run(writeRows("wrong-execution-authorization.jsonl", wrongExecutionAuthorization))
  );

  const wrongPolicyIdentity = structuredClone(original);
  wrongPolicyIdentity[4].policy_bundle_id = "different-policy";
  expectFailure(
    "policy identity mismatch",
    run(writeRows("wrong-policy-identity.jsonl", wrongPolicyIdentity))
  );

  const wrongEvaluator = structuredClone(original);
  wrongEvaluator[1].parties.evaluator.id = "different.authority";
  expectFailure("evaluator mismatch", run(writeRows("wrong-evaluator.jsonl", wrongEvaluator)));

  const wrongLease = structuredClone(original);
  wrongLease[3].lease_expires_at = new Date(
    Date.parse(wrongLease[3].claimed_at) + 3_000
  ).toISOString();
  expectFailure("lease mismatch", run(writeRows("wrong-lease.jsonl", wrongLease)));

  const blankRow = path.join(temporary, "blank-row.jsonl");
  fs.writeFileSync(blankRow, `${original.map((record) => canonicalize(record)).join("\n")}\n\n`);
  expectFailure("blank row", run(blankRow));

  const missingFinalLf = path.join(temporary, "missing-final-lf.jsonl");
  fs.writeFileSync(missingFinalLf, original.map((record) => canonicalize(record)).join("\n"));
  expectFailure("missing final LF", run(missingFinalLf));

  const invalidUtf8 = path.join(temporary, "invalid-utf8.jsonl");
  fs.writeFileSync(invalidUtf8, Buffer.from([0xff, 0xfe, 0xfd]));
  expectFailure("invalid UTF-8", run(invalidUtf8));

  const leadingBom = path.join(temporary, "leading-bom.jsonl");
  fs.writeFileSync(leadingBom, Buffer.concat([
    Buffer.from([0xef, 0xbb, 0xbf]),
    fs.readFileSync(fixture)
  ]));
  expectFailure("UTF-8 BOM before canonical evidence", run(leadingBom));

  console.log("PEP evidence validator: 1 positive and 15 negative cases passed");
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
