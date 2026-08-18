/** TL-PX 0.2 slice 2.4 policy/ordering contract tests. */
import {
  POLICY_PRECEDENCE,
  REQUIREMENT_MATURITY,
  validatedPolicyBundleHash,
  resolvePolicyDecision,
  selectActivePolicyBundle,
  validatePolicyBundleManifest,
  verifyCompleteAuthoritySequence
} from "../src/policy-v02.mjs";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const H = `sha256:${"a".repeat(64)}`;
const T0 = "2026-08-17T12:00:00.000Z";
const T1 = "2026-08-17T13:00:00.000Z";
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const golden = JSON.parse(
  fs.readFileSync(path.join(repo, "tests/fixtures/tlpx-0.2/policy/manifest-golden.json"), "utf8")
);
let n = 0;

function assert(condition, message) {
  if (!condition) throw new Error(message);
  n += 1;
  console.log(`  ok  ${message}`);
}

function throwsCode(fn, code, message) {
  let error;
  try {
    fn();
  } catch (caught) {
    error = caught;
  }
  assert(error instanceof Error, `${message}: threw`);
  assert(error.code === code, `${message}: ${code}`);
}

function manifest(overrides = {}) {
  return {
    manifest_type: "tlpx.policy_bundle",
    standard: "TL-PX",
    standard_version: "0.2.0",
    policy_bundle_id: "tenant-policy",
    policy_bundle_version: "1.0.0",
    issuer: { id: "security.platform", type: "human" },
    content_type: "application/vnd.tlpx.rust-exact-match+json;version=2",
    content_hash: H,
    activated_at: T0,
    retired_at: null,
    environment: "production",
    tenant: "tenant_abc",
    precedence: [...POLICY_PRECEDENCE],
    default_decision: "DENY",
    ...overrides
  };
}

console.log("TL-PX 0.2 slice 2.4 policy profile\n");

{
  console.log("provenance manifest");
  const valid = manifest();
  assert(validatePolicyBundleManifest(valid).ok, "valid manifest accepted");
  assert(validatedPolicyBundleHash(valid).startsWith("sha256:"), "manifest hash is canonical sha256 form");
  assert(
    !validatePolicyBundleManifest({ ...valid, issuer: undefined }).ok,
    "missing issuer rejected"
  );
  assert(
    !validatePolicyBundleManifest({ ...valid, policy_bundle_version: "01.0.0" }).ok,
    "non-semver version rejected"
  );
  assert(
    !validatePolicyBundleManifest({ ...valid, activated_at: "tomorrow" }).ok,
    "untrusted activation time rejected"
  );
  assert(
    !validatePolicyBundleManifest({ ...valid, retired_at: T0 }).ok,
    "non-positive activation interval rejected"
  );
  assert(
    !validatePolicyBundleManifest({ ...valid, precedence: [...POLICY_PRECEDENCE].reverse() }).ok,
    "reordered precedence rejected"
  );
  assert(
    !validatePolicyBundleManifest({
      ...valid,
      supersedes: {
        policy_bundle_id: valid.policy_bundle_id,
        policy_bundle_version: valid.policy_bundle_version,
        policy_bundle_hash: validatedPolicyBundleHash(valid)
      }
    }).ok,
    "self-supersession rejected"
  );
  assert(validatePolicyBundleManifest(golden.manifest).ok, "golden manifest accepted");
  assert(
    validatedPolicyBundleHash(golden.manifest) === golden.policy_bundle_hash,
    "golden manifest hash matches"
  );
}

{
  console.log("active bundle selection");
  const v1 = manifest();
  const selected = selectActivePolicyBundle([v1], {
    environment: "production",
    tenant: "tenant_abc",
    at: T1
  });
  assert(selected.manifest === v1, "single exact-scope active bundle selected");
  assert(selected.policy_bundle_hash === validatedPolicyBundleHash(v1), "selection returns manifest hash");
  throwsCode(
    () => selectActivePolicyBundle([v1], { environment: "staging", tenant: "tenant_abc", at: T1 }),
    "POLICY_UNAVAILABLE",
    "scope mismatch fails closed"
  );
  const unrelated = manifest({ policy_bundle_id: "other-policy", policy_bundle_version: "9.0.0" });
  throwsCode(
    () => selectActivePolicyBundle([v1, unrelated], { environment: "production", tenant: "tenant_abc", at: T1 }),
    "POLICY_PRECEDENCE_AMBIGUOUS",
    "version number does not decide overlap"
  );
  const v2 = manifest({
    policy_bundle_version: "2.0.0",
    activated_at: "2026-08-17T12:30:00.000Z",
    supersedes: {
      policy_bundle_id: "tenant-policy",
      policy_bundle_version: "1.0.0",
      policy_bundle_hash: validatedPolicyBundleHash(v1)
    }
  });
  assert(
    selectActivePolicyBundle([v1, v2], { environment: "production", tenant: "tenant_abc", at: T1 }).manifest === v2,
    "explicit successor uniquely wins"
  );
  const cycleA = manifest({
    policy_bundle_version: "3.0.0",
    supersedes: {
      policy_bundle_id: "tenant-policy",
      policy_bundle_version: "4.0.0",
      policy_bundle_hash: H
    }
  });
  const cycleB = manifest({
    policy_bundle_version: "4.0.0",
    supersedes: {
      policy_bundle_id: "tenant-policy",
      policy_bundle_version: "3.0.0",
      policy_bundle_hash: H
    }
  });
  throwsCode(
    () => selectActivePolicyBundle([cycleA, cycleB], { environment: "production", tenant: "tenant_abc", at: T1 }),
    "POLICY_PROVENANCE_INVALID",
    "supersession cycle rejected"
  );
  const wrongHash = manifest({
    policy_bundle_version: "2.1.0",
    activated_at: "2026-08-17T12:30:00.000Z",
    supersedes: {
      policy_bundle_id: "tenant-policy",
      policy_bundle_version: "1.0.0",
      policy_bundle_hash: H
    }
  });
  throwsCode(
    () => selectActivePolicyBundle([v1, wrongHash], { environment: "production", tenant: "tenant_abc", at: T1 }),
    "POLICY_PROVENANCE_INVALID",
    "predecessor hash mismatch rejected"
  );
  const crossScope = manifest({
    policy_bundle_version: "2.2.0",
    activated_at: "2026-08-17T12:30:00.000Z",
    tenant: "tenant_other",
    supersedes: {
      policy_bundle_id: "tenant-policy",
      policy_bundle_version: "1.0.0",
      policy_bundle_hash: validatedPolicyBundleHash(v1)
    }
  });
  throwsCode(
    () => selectActivePolicyBundle([v1, crossScope], { environment: "production", tenant: "tenant_abc", at: T1 }),
    "POLICY_PROVENANCE_INVALID",
    "cross-scope supersession rejected"
  );
  const timeReversed = manifest({
    policy_bundle_version: "2.3.0",
    activated_at: "2026-08-17T11:00:00.000Z",
    supersedes: {
      policy_bundle_id: "tenant-policy",
      policy_bundle_version: "1.0.0",
      policy_bundle_hash: validatedPolicyBundleHash(v1)
    }
  });
  throwsCode(
    () => selectActivePolicyBundle([v1, timeReversed], { environment: "production", tenant: "tenant_abc", at: T1 }),
    "POLICY_PROVENANCE_INVALID",
    "time-reversed supersession rejected"
  );
}

{
  console.log("deterministic precedence");
  assert(resolvePolicyDecision({ BASE_POLICY: "ALLOW" }).decision === "ALLOW", "base ALLOW");
  assert(
    resolvePolicyDecision({ BASE_POLICY: "ALLOW", SWITCHBOARD_SCOPE: "DENY" }).decision === "DENY",
    "Switchboard DENY cannot be weakened"
  );
  assert(
    resolvePolicyDecision({ BASE_POLICY: "ALLOW", EMERGENCY_DENY: "DENY" }).decision === "DENY",
    "emergency DENY cannot be weakened"
  );
  assert(
    resolvePolicyDecision({
      TENANT_ENVIRONMENT_RESTRICTION: "REQUIRE_APPROVAL",
      BASE_POLICY: "ALLOW",
      ACTION_POLICY: "ALLOW"
    }).decision === "REQUIRE_APPROVAL",
    "later ALLOW cannot lower earlier approval requirement"
  );
  assert(
    resolvePolicyDecision({ BASE_POLICY: "ALLOW", HUMAN_APPROVAL_CONDITION: "REQUIRE_APPROVAL" }).decision === "REQUIRE_APPROVAL",
    "human approval condition tightens ALLOW"
  );
  throwsCode(
    () => resolvePolicyDecision({ ACTION_POLICY: "ALLOW" }),
    "POLICY_PRECEDENCE_AMBIGUOUS",
    "missing explicit base outcome fails closed"
  );
  throwsCode(
    () => resolvePolicyDecision({ BASE_POLICY: "ALLOW", SWITCHBOARD_SCOPE: "ALLOW" }),
    "POLICY_PRECEDENCE_AMBIGUOUS",
    "invalid stage outcome fails closed"
  );
  throwsCode(
    () => resolvePolicyDecision({ BASE_POLICY: "ALLOW", MADE_UP_STAGE: "DENY" }),
    "POLICY_PRECEDENCE_AMBIGUOUS",
    "unknown stage fails closed"
  );
}

{
  console.log("trusted authority ordering");
  assert(
    verifyCompleteAuthoritySequence([
      { sequence: 41, occurred_at: T1 },
      { sequence: 42, occurred_at: T0 },
      { sequence: 43, occurred_at: T1 }
    ], 41),
    "sequence, not wall clock, establishes order"
  );
  throwsCode(
    () => verifyCompleteAuthoritySequence([{ sequence: 41 }, { sequence: 42 }]),
    "TRUSTED_SEQUENCE_INVALID",
    "undeclared missing stream prefix rejected"
  );
  throwsCode(
    () => verifyCompleteAuthoritySequence([{ sequence: 1 }, { sequence: 1 }]),
    "TRUSTED_SEQUENCE_INVALID",
    "duplicate sequence rejected"
  );
  throwsCode(
    () => verifyCompleteAuthoritySequence([{ sequence: 2 }, { sequence: 1 }], 2),
    "TRUSTED_SEQUENCE_INVALID",
    "backward sequence rejected"
  );
  throwsCode(
    () => verifyCompleteAuthoritySequence([{ sequence: 7 }, { sequence: 9 }], 7),
    "TRUSTED_SEQUENCE_INVALID",
    "gap in complete stream rejected"
  );
}

{
  console.log("maturity vocabulary");
  assert(REQUIREMENT_MATURITY.includes("IMPLEMENTED"), "implemented label exists");
  assert(REQUIREMENT_MATURITY.includes("PLANNED"), "planned label exists");
  assert(REQUIREMENT_MATURITY.includes("EXTENSION_EXPERIMENTAL"), "extension label exists");
}

console.log(`\nSlice 2.4 policy tests passed (${n}).`);
