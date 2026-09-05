#!/usr/bin/env node

import fs from "node:fs";
import { TextDecoder } from "node:util";
import { canonicalize } from "../src/jcs.mjs";
import { validateV02 } from "../src/validate-v02.mjs";

const input = process.argv[2];
const expectedBinaryHash = process.argv[3];
if (!input || !expectedBinaryHash || process.argv.length !== 4) {
  console.error(
    "usage: node scripts/validate-pep-evidence.mjs EVIDENCE_JSONL EXPECTED_BINARY_SHA256"
  );
  process.exit(2);
}
if (!/^sha256:[0-9a-f]{64}$/.test(expectedBinaryHash)) {
  console.error("expected binary SHA-256 must use canonical sha256: lowercase-hex form");
  process.exit(2);
}

const kindByType = new Map([
  ["tlpx.decision", "decision"],
  ["tlpx.authorization", "authorization"],
  ["tlpx.authorization_claim", "authorization-claim"],
  ["tlpx.execution", "execution"]
]);
const expectedRows = [
  ["tlpx.decision", "DENY"],
  ["tlpx.decision", "ALLOW"],
  ["tlpx.authorization", "AUTHORIZED_UNCLAIMED"],
  ["tlpx.authorization_claim", "CLAIMED"],
  ["tlpx.execution", "COMPLETED"]
];
const envelopeFields = [
  "record_hash",
  "previous_chain_hash",
  "chain_hash",
  "seal_algorithm",
  "seal_key_id",
  "seal"
];

let failures = 0;
const fail = (message) => {
  console.log(`  FAIL  ${message}`);
  failures += 1;
};

console.log("Restricted PEP evidence crosscheck\n");

let text;
try {
  // Preserve a BOM so the JSON/JCS checks reject those extra input bytes.
  text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(fs.readFileSync(input));
} catch {
  fail("evidence is unavailable or is not valid UTF-8");
}

let lines = [];
if (text !== undefined) {
  if (!text.endsWith("\n")) fail("evidence must end with one LF");
  if (text.includes("\r")) fail("evidence must use LF line endings");
  lines = text.endsWith("\n") ? text.slice(0, -1).split("\n") : text.split("\n");
  if (lines.some((line) => line.length === 0)) fail("blank evidence rows are forbidden");
  if (lines.length !== expectedRows.length) {
    fail(`expected exactly ${expectedRows.length} rows, got ${lines.length}`);
  }
}

const records = [];
for (const [index, line] of lines.entries()) {
  let record;
  try {
    record = JSON.parse(line);
  } catch {
    fail(`row ${index + 1}: invalid JSON`);
    continue;
  }
  const expected = expectedRows[index];
  if (!expected || record.record_type !== expected[0]) {
    fail(`row ${index + 1}: unexpected ${record.record_type ?? "record type"}`);
    continue;
  }
  const kind = kindByType.get(record.record_type);
  const validation = validateV02(kind, record);
  let canonical = false;
  try {
    canonical = canonicalize(record) === line;
  } catch {
    canonical = false;
  }
  const envelopeLeak = envelopeFields.some((field) => field in record);
  if (!validation.ok || !canonical || envelopeLeak) {
    fail(record.record_type);
    for (const error of validation.errors) console.log(`        ${error}`);
    if (!canonical) console.log("        record is not canonical JCS");
    if (envelopeLeak) console.log("        storage-envelope metadata leaked into record");
    continue;
  }
  const outcome = record.decision ?? record.state;
  if (outcome !== expected[1]) {
    fail(`row ${index + 1}: expected ${expected[1]}, got ${outcome}`);
    continue;
  }
  records.push(record);
  console.log(`  PASS  ${record.record_type} ${outcome}`);
}

if (records.length === expectedRows.length) {
  const [deny, allow, authorization, claim, execution] = records;
  const evaluatedAt = Date.parse(allow.evaluated_at);
  const issuedAt = Date.parse(authorization.issued_at);
  const claimExpiresAt = Date.parse(authorization.claim_expires_at);
  const claimedAt = Date.parse(claim.claimed_at);
  const leaseExpiresAt = Date.parse(claim.lease_expires_at);
  const startedAt = Date.parse(execution.started_at);
  const endedAt = Date.parse(execution.ended_at);
  const timestamps = [
    evaluatedAt,
    issuedAt,
    claimExpiresAt,
    claimedAt,
    leaseExpiresAt,
    startedAt,
    endedAt
  ];
  const checks = [
    [deny.request_id === "denied-1", "DENY request id"],
    [deny.reason_code === "SWITCHBOARD_ACTION_DENIED", "DENY reason"],
    [deny.reason === deny.reason_code, "DENY reason detail"],
    [deny.authorization_state === "DENIED", "DENY authorization state"],
    [
      deny.policy_bundle_id === "tlpx.switchboard" &&
        deny.policy_bundle_version === "0.2.0" &&
        deny.policy_id === null,
      "DENY Switchboard provenance"
    ],
    [allow.request_id === "allowed-1", "ALLOW request id"],
    [allow.reason_code === "POLICY_ALLOW", "ALLOW reason"],
    [allow.reason === allow.reason_code, "ALLOW reason detail"],
    [allow.authorization_state === "AUTHORIZED_UNCLAIMED", "ALLOW authorization state"],
    [
      allow.policy_bundle_id === "restricted-marker-pep" &&
        allow.policy_bundle_version === "1.0.0" &&
        allow.policy_id === "allow-restricted-marker",
      "ALLOW policy provenance"
    ],
    [
      deny.control_mode === "ALLOW_ESCALATE_OR_DENY" &&
        allow.control_mode === "ALLOW_ESCALATE_OR_DENY",
      "decision control mode"
    ],
    [
      deny.parties.evaluator.id === "restricted.pep.authority" &&
        allow.parties.evaluator.id === deny.parties.evaluator.id &&
        deny.parties.router.id === "restricted.pep.switchboard" &&
        allow.parties.router.id === deny.parties.router.id &&
        deny.parties.requester.id === "restricted.agent" &&
        allow.parties.requester.id === deny.parties.requester.id,
      "decision party identities"
    ],
    [
      deny.sequence === 1 &&
        allow.sequence === 2 &&
        claim.sequence === 3 &&
        execution.sequence === 4,
      "authority sequence"
    ],
    [deny.receipt_id !== allow.receipt_id, "distinct DENY receipt"],
    [deny.intent_hash !== allow.intent_hash, "distinct DENY intent"],
    [
      allow.receipt_id === authorization.receipt_id &&
        authorization.receipt_id === claim.receipt_id &&
        claim.receipt_id === execution.receipt_id,
      "receipt linkage"
    ],
    [
      authorization.authorization_id === claim.authorization_id &&
        claim.authorization_id === execution.authorization_id,
      "authorization linkage"
    ],
    [claim.claim_id === execution.claim_id, "claim linkage"],
    [
      allow.intent_hash === authorization.intent_hash &&
        authorization.intent_hash === execution.intent_hash,
      "intent-hash linkage"
    ],
    [allow.policy_bundle_hash === execution.policy_bundle_hash, "policy-bundle linkage"],
    [
      authorization.authorized_action_hash === claim.authorized_action_hash &&
        claim.authorized_action_hash === execution.authorized_action_hash,
      "authorized-action-hash linkage"
    ],
    [
      authorization.action_binding_hash === claim.action_binding_hash &&
        claim.action_binding_hash === claim.executed_action_hash &&
        claim.executed_action_hash === execution.executed_action_hash,
      "action-binding linkage"
    ],
    [
      authorization.adapter.id === claim.adapter.id &&
        claim.adapter.id === execution.adapter.id &&
        authorization.adapter.version === claim.adapter.version &&
        claim.adapter.version === execution.adapter.version,
      "adapter linkage"
    ],
    [
      authorization.adapter.id === "adapter.restricted-marker" &&
        authorization.adapter.version === "1.0.0" &&
        execution.adapter_principal === "restricted.pep.adapter",
      "trusted adapter identity"
    ],
    [
      allow.parties.requester.id === "restricted.agent" &&
        authorization.requesting_principal === "restricted.agent" &&
        authorization.executing_principal === "restricted.agent" &&
        claim.executing_principal === "restricted.agent" &&
        execution.requesting_principal === "restricted.agent" &&
        execution.executing_principal === "restricted.agent",
      "principal linkage"
    ],
    [authorization.action === "shell.exec", "authorized action"],
    [
      authorization.target === "/usr/bin/touch" && execution.target === authorization.target,
      "protected target linkage"
    ],
    [
      authorization.environment === "restricted_pep" &&
        authorization.tenant === "local_acceptance" &&
        authorization.execution_lease_seconds === 2,
      "authorization execution scope"
    ],
    [
      execution.policy_bundle_id === allow.policy_bundle_id &&
        execution.policy_bundle_version === allow.policy_bundle_version,
      "policy identity linkage"
    ],
    [
      execution.result_summary ===
        "protected command exited with code 0; output withheld (stdout 0 bytes, stderr 0 bytes)" &&
        execution.result_hash === null &&
        execution.external_evidence_reference === null &&
        execution.cancellation_outcome === null,
      "bounded completion evidence"
    ],
    [execution.adapter_binary_hash === expectedBinaryHash, "tested-binary linkage"],
    [timestamps.every(Number.isFinite), "valid timestamps"],
    [
      evaluatedAt === issuedAt &&
        issuedAt <= claimedAt &&
        claimedAt < claimExpiresAt &&
        claimExpiresAt - issuedAt === 5_000 &&
        claimedAt <= startedAt &&
        startedAt <= endedAt &&
        startedAt < leaseExpiresAt &&
        leaseExpiresAt - claimedAt === 2_000,
      "timestamp ordering"
    ]
  ];
  for (const [holds, label] of checks) {
    if (!holds) fail(`${label} mismatch`);
  }
}

console.log(`\nRestricted PEP evidence: ${lines.length} rows, ${failures} failures`);
if (failures) process.exit(1);
