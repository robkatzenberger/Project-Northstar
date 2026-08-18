/**
 * TL-PX 0.2 schema-bound action canonicalization and hashing oracle.
 * This is contract tooling, not a decision engine or PEP.
 */
import { canonicalize } from "./jcs.mjs";
import {
  intentHash,
  authorizedActionHash,
  executedActionHash
} from "./hash.mjs";
import { validateV02 } from "./validate-v02.mjs";

export const ACTION_BINDING_FIELDS = Object.freeze([
  "executing_principal",
  "action",
  "target",
  "arguments",
  "environment",
  "tenant",
  "payload_hash",
  "artifact_hash",
  "adapter"
]);

function assertValid(kind, value) {
  const result = validateV02(kind, value);
  if (!result.ok) {
    const error = new Error(`${kind}: ${result.errors.join("; ")}`);
    error.code = "ACTION_SCHEMA_INVALID";
    throw error;
  }
  return value;
}

function bindingValue(value) {
  return Object.fromEntries(
    ACTION_BINDING_FIELDS.map((field) => [field, structuredClone(value[field])])
  );
}

export function canonicalSubmittedIntentV02(value) {
  return canonicalize(assertValid("submitted-intent", value));
}

export function canonicalAuthorizedActionV02(value) {
  return canonicalize(assertValid("authorized-action", value));
}

export function canonicalExecutedActionV02(value) {
  return canonicalize(assertValid("executed-action", value));
}

export function submittedIntentHashV02(value) {
  assertValid("submitted-intent", value);
  return intentHash(value);
}

export function authorizedActionHashV02(value) {
  assertValid("authorized-action", value);
  return authorizedActionHash(value);
}

export function executedActionHashV02(value) {
  assertValid("executed-action", value);
  return executedActionHash(value);
}

export function authorizedActionBindingV02(value) {
  assertValid("authorized-action", value);
  return bindingValue(value);
}

export function executedActionBindingV02(value) {
  assertValid("executed-action", value);
  return bindingValue(value);
}

export function authorizedActionBindingHashV02(value) {
  return executedActionHash(authorizedActionBindingV02(value));
}

export function executedActionBindingHashV02(value) {
  return executedActionHash(executedActionBindingV02(value));
}

export function actionBindingsMatchV02(authorized, executed) {
  return (
    authorizedActionBindingHashV02(authorized) ===
    executedActionBindingHashV02(executed)
  );
}
