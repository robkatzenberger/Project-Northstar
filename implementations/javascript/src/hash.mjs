/**
 * TL-PX 0.2 domain-separated SHA-256 over Northstar JCS.
 * Exact string form: sha256: + 64 lowercase hex.
 */

import { createHash } from "node:crypto";
import { canonicalize, canonicalizeJsonText, utf8Bytes } from "./jcs.mjs";

export const HASH_PATTERN = /^sha256:[0-9a-f]{64}$/;

/** Domain prefixes include a trailing NUL. */
export const HASH_DOMAINS = Object.freeze({
  intent: "northstar:intent:v1\0",
  "authorized-action": "northstar:authorized-action:v1\0",
  "executed-action": "northstar:executed-action:v1\0",
  "approval-context": "northstar:approval-context:v1\0",
  "policy-bundle": "northstar:policy-bundle:v1\0"
});

export function assertHashString(value) {
  if (typeof value !== "string" || !HASH_PATTERN.test(value)) {
    throw new Error("hash: expected sha256: plus 64 lowercase hex characters");
  }
  return value;
}

export function resolveDomain(domain) {
  if (typeof domain !== "string" || domain.length === 0) {
    throw new Error("hash: domain required");
  }
  return Object.prototype.hasOwnProperty.call(HASH_DOMAINS, domain)
    ? HASH_DOMAINS[domain]
    : domain;
}

export function digestBytes(domain, canonical) {
  const prefix = resolveDomain(domain);
  return createHash("sha256")
    .update(prefix, "utf8")
    .update(utf8Bytes(canonical))
    .digest();
}

export function digestHex(domain, canonical) {
  return digestBytes(domain, canonical).toString("hex");
}

export function hashString(domain, canonical) {
  return `sha256:${digestHex(domain, canonical)}`;
}

export function hashValue(domain, value) {
  return hashString(domain, canonicalize(value));
}

export function hashJsonText(domain, text) {
  return hashString(domain, canonicalizeJsonText(text));
}

export function intentHash(value) {
  return hashValue("intent", value);
}

export function authorizedActionHash(value) {
  return hashValue("authorized-action", value);
}

export function executedActionHash(value) {
  return hashValue("executed-action", value);
}

export function approvalContextHash(value) {
  return hashValue("approval-context", value);
}

export function policyBundleHash(value) {
  return hashValue("policy-bundle", value);
}
