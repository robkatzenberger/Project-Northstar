/**
 * TL-PX 0.2 slice 2.4 policy provenance, precedence, and ordering oracle.
 * This module does not turn the JavaScript 0.1 gate into a 0.2 authority.
 */
import { hashValue } from "./hash.mjs";
import { validateSchemaFile } from "./schema.mjs";

export const POLICY_PRECEDENCE = Object.freeze([
  "EMERGENCY_DENY",
  "TENANT_ENVIRONMENT_RESTRICTION",
  "SWITCHBOARD_SCOPE",
  "BASE_POLICY",
  "ACTION_POLICY",
  "HUMAN_APPROVAL_CONDITION"
]);

export const REQUIREMENT_MATURITY = Object.freeze([
  "IMPLEMENTED",
  "PLANNED",
  "EXTENSION_EXPERIMENTAL"
]);

const STAGE_OUTCOMES = Object.freeze({
  EMERGENCY_DENY: new Set(["PASS", "DENY"]),
  TENANT_ENVIRONMENT_RESTRICTION: new Set(["PASS", "REQUIRE_APPROVAL", "DENY"]),
  SWITCHBOARD_SCOPE: new Set(["PASS", "DENY"]),
  BASE_POLICY: new Set(["ALLOW", "REQUIRE_APPROVAL", "DENY"]),
  ACTION_POLICY: new Set(["PASS", "ALLOW", "REQUIRE_APPROVAL", "DENY"]),
  HUMAN_APPROVAL_CONDITION: new Set(["PASS", "REQUIRE_APPROVAL", "DENY"])
});

class PolicyProfileError extends Error {
  constructor(code, message) {
    super(`${code}: ${message}`);
    this.name = "PolicyProfileError";
    this.code = code;
  }
}

function parseInstant(value, field, errors) {
  const ms = Date.parse(value);
  if (!Number.isFinite(ms) || new Date(ms).toISOString() !== value) {
    errors.push(`${field} must be canonical UTC ISO-8601 with milliseconds`);
    return null;
  }
  return ms;
}

function bundleKey(bundle) {
  return `${bundle.policy_bundle_id}@${bundle.policy_bundle_version}`;
}

export function validatePolicyBundleManifest(value) {
  const result = validateSchemaFile("policy-bundle.schema.json", value);
  if (!result.ok) return result;

  const activated = parseInstant(value.activated_at, "activated_at", result.errors);
  const retired = value.retired_at === null
    ? null
    : parseInstant(value.retired_at, "retired_at", result.errors);
  if (activated !== null && retired !== null && retired <= activated) {
    result.errors.push("retired_at must be later than activated_at");
  }
  if (
    value.precedence.length !== POLICY_PRECEDENCE.length ||
    value.precedence.some((stage, index) => stage !== POLICY_PRECEDENCE[index])
  ) {
    result.errors.push(`precedence must equal ${POLICY_PRECEDENCE.join(" -> ")}`);
  }
  if (
    value.supersedes &&
    value.supersedes.policy_bundle_id === value.policy_bundle_id &&
    value.supersedes.policy_bundle_version === value.policy_bundle_version
  ) {
    result.errors.push("a policy bundle cannot supersede itself");
  }
  result.ok = result.errors.length === 0;
  return result;
}

export function validatedPolicyBundleHash(manifest) {
  const validation = validatePolicyBundleManifest(manifest);
  if (!validation.ok) {
    throw new PolicyProfileError("POLICY_PROVENANCE_INVALID", validation.errors.join("; "));
  }
  return hashValue("policy-bundle", manifest);
}

function supersessionAncestors(candidate, byKey) {
  const seen = new Set();
  const edges = [];
  let cursor = candidate;
  while (cursor.supersedes) {
    const key = `${cursor.supersedes.policy_bundle_id}@${cursor.supersedes.policy_bundle_version}`;
    if (seen.has(key)) {
      throw new PolicyProfileError("POLICY_PROVENANCE_INVALID", "supersession cycle");
    }
    seen.add(key);
    const prior = byKey.get(key);
    if (!prior) {
      throw new PolicyProfileError(
        "POLICY_PROVENANCE_INVALID",
        `superseded manifest ${key} is unavailable`
      );
    }
    edges.push([cursor, prior]);
    cursor = prior;
  }
  for (const [successor, prior] of edges) {
    if (successor.environment !== prior.environment || successor.tenant !== prior.tenant) {
      throw new PolicyProfileError(
        "POLICY_PROVENANCE_INVALID",
        "supersession must preserve exact tenant/environment scope"
      );
    }
    if (Date.parse(successor.activated_at) <= Date.parse(prior.activated_at)) {
      throw new PolicyProfileError(
        "POLICY_PROVENANCE_INVALID",
        "a successor must activate after its predecessor"
      );
    }
    if (successor.supersedes.policy_bundle_hash !== validatedPolicyBundleHash(prior)) {
      throw new PolicyProfileError(
        "POLICY_PROVENANCE_INVALID",
        "supersession predecessor hash mismatch"
      );
    }
  }
  return seen;
}

export function selectActivePolicyBundle(manifests, context) {
  if (!Array.isArray(manifests) || manifests.length === 0) {
    throw new PolicyProfileError("POLICY_UNAVAILABLE", "no policy bundles configured");
  }
  if (!context || typeof context !== "object") {
    throw new PolicyProfileError("POLICY_PROVENANCE_INVALID", "selection context required");
  }
  const at = Date.parse(context.at);
  if (!Number.isFinite(at) || new Date(at).toISOString() !== context.at) {
    throw new PolicyProfileError("POLICY_PROVENANCE_INVALID", "trusted selection time is invalid");
  }

  const byKey = new Map();
  for (const manifest of manifests) {
    const validation = validatePolicyBundleManifest(manifest);
    if (!validation.ok) {
      throw new PolicyProfileError("POLICY_PROVENANCE_INVALID", validation.errors.join("; "));
    }
    const key = bundleKey(manifest);
    if (byKey.has(key)) {
      throw new PolicyProfileError("POLICY_PROVENANCE_INVALID", `duplicate policy identity ${key}`);
    }
    byKey.set(key, manifest);
  }

  // Validate every configured supersession chain, including inactive bundles.
  for (const manifest of manifests) supersessionAncestors(manifest, byKey);

  const active = manifests.filter((manifest) => {
    if (manifest.environment !== context.environment || manifest.tenant !== context.tenant) {
      return false;
    }
    const activated = Date.parse(manifest.activated_at);
    const retired = manifest.retired_at === null ? null : Date.parse(manifest.retired_at);
    return activated <= at && (retired === null || at < retired);
  });
  if (active.length === 0) {
    throw new PolicyProfileError("POLICY_UNAVAILABLE", "no active policy for exact tenant/environment");
  }
  if (active.length === 1) {
    return { manifest: active[0], policy_bundle_hash: validatedPolicyBundleHash(active[0]) };
  }

  const activeKeys = new Set(active.map(bundleKey));
  const winners = active.filter((candidate) => {
    const ancestors = supersessionAncestors(candidate, byKey);
    return [...activeKeys].every((key) => key === bundleKey(candidate) || ancestors.has(key));
  });
  if (winners.length !== 1) {
    throw new PolicyProfileError(
      "POLICY_PRECEDENCE_AMBIGUOUS",
      "overlapping active bundles require one explicit supersession winner"
    );
  }
  return { manifest: winners[0], policy_bundle_hash: validatedPolicyBundleHash(winners[0]) };
}

export function resolvePolicyDecision(stageResults) {
  if (!stageResults || typeof stageResults !== "object" || Array.isArray(stageResults)) {
    throw new PolicyProfileError("POLICY_PRECEDENCE_AMBIGUOUS", "stage results object required");
  }
  for (const stage of Object.keys(stageResults)) {
    if (!POLICY_PRECEDENCE.includes(stage)) {
      throw new PolicyProfileError(
        "POLICY_PRECEDENCE_AMBIGUOUS",
        `unknown precedence stage ${stage}`
      );
    }
  }
  let result = "ALLOW";
  const applied = [];
  for (const stage of POLICY_PRECEDENCE) {
    const outcome = stageResults[stage] ?? (stage === "BASE_POLICY" ? undefined : "PASS");
    if (!STAGE_OUTCOMES[stage].has(outcome)) {
      throw new PolicyProfileError(
        "POLICY_PRECEDENCE_AMBIGUOUS",
        `${stage} has invalid or missing outcome ${String(outcome)}`
      );
    }
    if (outcome === "PASS") continue;
    applied.push({ stage, outcome });
    if (outcome === "DENY") return { decision: "DENY", applied };
    if (outcome === "REQUIRE_APPROVAL") result = "REQUIRE_APPROVAL";
    // ALLOW never lowers a prior REQUIRE_APPROVAL.
  }
  return { decision: result, applied };
}

export function verifyCompleteAuthoritySequence(events, expectedStart = 1) {
  if (!Array.isArray(events)) {
    throw new PolicyProfileError("TRUSTED_SEQUENCE_INVALID", "events must be an array");
  }
  if (!Number.isSafeInteger(expectedStart) || expectedStart < 1) {
    throw new PolicyProfileError("TRUSTED_SEQUENCE_INVALID", "expected start must be positive");
  }
  if (events.length === 0) return true;
  let expected = expectedStart;
  for (const [index, event] of events.entries()) {
    if (!event || !Number.isSafeInteger(event.sequence) || event.sequence < 1) {
      throw new PolicyProfileError("TRUSTED_SEQUENCE_INVALID", `event ${index} has invalid sequence`);
    }
    if (event.sequence !== expected) {
      throw new PolicyProfileError(
        "TRUSTED_SEQUENCE_INVALID",
        `expected sequence ${expected}, got ${event.sequence}`
      );
    }
    expected += 1;
  }
  return true;
}
