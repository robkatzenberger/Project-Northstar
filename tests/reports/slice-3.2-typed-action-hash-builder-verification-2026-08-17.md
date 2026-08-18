# Slice 3.2 typed action and hash boundary — builder verification

**Local date:** 2026-08-17 (America/Chicago)  
**Base commit:** `31831a2c5e12735f908e49c544368988fcbef283`  
**Dependencies:** local slice 2.4 commit `a87f822`; local slice 3.1 commit `1addb5c`  
**Artifact reviewed:** uncommitted slice 3.2 working-tree delta  
**Disposition:** BOUNDED BUILDER-VERIFIED WORKING-TREE CANDIDATE — not committed, full-Section-3 verified, independently reviewed, or accepted

## Scope

This increment closes the typed cross-language evidence boundary for the three distinct action objects already present in the Rust authority. It adds:

- shared schema-valid Submitted Intent, Authorized Action, Executed Action, and Action Binding fixture cases;
- exact canonical strings, canonical UTF-8 bytes, raw SHA-256 digests, and domain-separated `sha256:` hashes for each case;
- strict JavaScript helpers that validate the matching 0.2 schema before canonicalization or hashing;
- exact Action Binding projection and comparison helpers using the `executed-action` domain;
- Rust tests that reconstruct typed values and match the same fixture bytes and hashes;
- explicit coverage for required nullable payload/artifact digests versus the optional absent/present retry link;
- exhaustive mutation coverage for every binding component, including adapter id and version separately;
- authority-only mutation coverage proving those fields change the full Authorized Action hash without changing the Action Binding; and
- Authorized Action schema enforcement that `effective_risk` cannot be lower than `derived_risk`, matching the existing Rust type invariant.

The production Rust action types, distinct domain hashes, and binding projection already existed in the accepted local-authority line. This slice binds them to shared schema-level evidence and closes a JavaScript/schema parity gap; it does not add a second authority implementation.

## Acceptance cases exercised

The shared fixture contains two typed cases:

- required `payload_hash` and `artifact_hash` explicitly `null`, with `retry_of_receipt_id` absent; and
- non-null payload/artifact digests, a present retry link, Unicode strings, nested objects and arrays, and safe integers.

The dedicated JavaScript suite contains 145 assertions covering fixture shape, schema validation, canonical strings, UTF-8 bytes, raw digests, distinct domain hashes, invalid type mixing, forbidden fields, missing required nullable fields, the complete derived/effective risk ordering matrix, floating-point rejection, detached binding projections, ten binding mutations, nine authority-only mutations, and eighteen Submitted Intent mutations.

The Rust slice test contains three focused tests covering typed reconstruction of both cases, required-null versus optional-absent serialization, exact Action Binding keys, and exclusion of authority-only fields. The existing Rust JCS/hash suite adds adapter-version mutation to the binding negative cases.

## Verification environment

- Node.js `v25.5.0`
- npm `11.8.0`
- rustc `1.97.1 (8bab26f4f 2026-07-14)`
- cargo `1.97.1 (c980f4866 2026-06-30)`

## Executed bounded checks

All listed commands ran from the local working tree and exited successfully.

| Check | Result |
| --- | --- |
| `npm run test:actions:0.2` | PASS — 145 assertions |
| `npm run conformance:0.2` | PASS — 62 passed / 0 failed, including the five slice 3.2 checks |
| `npm run test:jcs` | PASS — 493 assertions |
| Fixture regeneration plus SHA-256 comparison | PASS — identical before/after checksum `1a078c7e60bbd3cbb2e6aa19fb1b65bda34a4288dd193f85f30f4766ee9c75ba` |
| JavaScript syntax checks for the new/changed modules | PASS |
| `cargo fmt --all -- --check` | PASS |
| `cargo test --test action_types --test jcs_golden --offline` | PASS — 3 slice tests plus 9 JCS/type/hash tests |
| `cargo clippy --test action_types --test jcs_golden --offline -- -D warnings` | PASS |
| JSON parse check | PASS — 27 JSON files parsed |
| Local Markdown-link check | PASS — 65 Markdown files checked |
| `git diff --check` | PASS |

## Explicit limits and non-claims

- This is an uncommitted working-tree candidate. Under the maturity rules, slice 3.2 remains `PLANNED`.
- The owner intentionally deferred the full JavaScript, Rust, technical-test, red-team, and exact-commit Section 3 matrix until all of Section 3 is complete. This report records bounded 3.2 checks only.
- No independent reviewer has evaluated this delta. Builder verification is not acceptance.
- The JavaScript helpers are a schema/hash oracle. They do not issue, claim, or execute a 0.2 authorization and do not replace Rust as the planned authority.
- Schema validation and cross-language hash agreement do not authenticate requester, operator, executor, policy publisher, adapter, or transport identity.
- Matching an Action Binding does not by itself enforce capability or resource-scope constraints; those remain separate claim/PEP checks.
- This increment emits no execution receipt, cancellation evidence, reconciliation evidence, or revocation evidence.
- It performs no protected side effect and establishes no forced mediation.
- The still-provisional execution-side schema contract is not closed by these action-object fixtures.
- Named commit `c9bdd0f` still requires its own independent exact-commit review.

## Next gate

Commit this bounded 3.2 delta only when the owner asks. Then continue with slice 3.3: authenticated local requester, operator, executor, and cancellation boundaries. Once Section 3 is complete, run the full matrix against named 2.4/3.x commits and obtain an independent contract/security/conformance review before changing maturity or acceptance claims.
