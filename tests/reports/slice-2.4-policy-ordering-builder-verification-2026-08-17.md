# Slice 2.4 policy and ordering — builder verification

**Local date:** 2026-08-17 (America/Chicago)
**Base commit:** `c9bdd0fcd2d4c51fda9f3861724db0fd97524003`
**Artifact reviewed:** uncommitted working-tree delta on the base commit
**Disposition:** BUILDER-VERIFIED WORKING-TREE CANDIDATE — not committed, independently reviewed, or accepted

## Scope

This increment makes the Phase 2 policy-provenance and ordering contract executable without claiming a new authority runtime or protected-execution path. It adds:

- a closed `tlpx.policy_bundle_manifest` schema with exact tenant/environment scope, active interval, issuer, content binding, standard version, and explicit default behavior;
- a fifth domain-separated RFC 8785 hash, `northstar:policy-bundle:v1\0`, implemented by the JavaScript oracle and Rust hash library with one shared golden vector;
- fail-closed active-bundle selection using exact scope, trusted activation time, and explicit hash-bound supersession rather than semantic-version ordering;
- fixed policy-layer precedence with final `DENY` and monotone `REQUIRE_APPROVAL` behavior;
- complete-stream authority-sequence verification that rejects undeclared missing prefixes, duplicate, backward, or gapped sequence numbers and does not use wall-clock timestamps to establish order;
- the exact maturity vocabulary `IMPLEMENTED`, `PLANNED`, and `EXTENSION_EXPERIMENTAL`; and
- 0.2 reason codes and conformance cases for provenance, ambiguity, and trusted-sequence failures.

The normative contract is in `docs/standard/SPEC-v0.2.md`; delivery boundaries and maturity are in `docs/BUILD-SPEC-SHEET.md`.

## Acceptance cases exercised

The dedicated slice test contains 48 assertions covering:

- valid manifests and the cross-language golden hash;
- missing issuer, invalid semantic version, untrusted activation time, non-positive active interval, reordered precedence, and self-supersession;
- exact-scope selection, no matching bundle, overlapping versions without provenance, explicit unique supersession, cycles, missing or hash-mismatched predecessors, cross-scope supersession, and time-reversed supersession;
- base `ALLOW`, final Switchboard/emergency `DENY`, approval-floor preservation, human-approval tightening, missing base outcome, invalid outcome, and unknown stage;
- timestamp-independent authority ordering plus undeclared-prefix, duplicate, backward, and complete-stream-gap rejection; and
- all three exact maturity labels.

The 0.2 contract suite independently exercises nine slice-specific positive and negative checks inside its 57-case total.

## Verification environment

- Node.js `v25.5.0`
- npm `11.8.0`
- rustc `1.97.1 (8bab26f4f 2026-07-14)`
- cargo `1.97.1 (c980f4866 2026-06-30)`

## Executed checks

All commands ran from the local working tree and exited successfully.

| Check | Result |
| --- | --- |
| `npm test` | PASS — 763 assertions across the JS reference, policy compile, slice 2.4, JCS/hash, and 0.2 contract suites |
| `npm run conformance` | PASS — TL-PX 0.1 Minimum Profile, 47 passed / 0 failed |
| `npm run tech-test` | PASS — 29 passed / 0 failed |
| `npm run redteam` | PASS for all in-scope attacks — 16 PASS / 0 FAIL / 4 documented deployment-boundary WARN |
| `npm run test:rust-evidence` | PASS — 4 supported Rust runtime record types validate |
| `cargo fmt --check` | PASS |
| `cargo test --all-targets --offline` | PASS — 43 tests: 23 authority, 11 evidence/outbox, 9 JCS/type/hash |
| `cargo clippy --all-targets --offline -- -D warnings` | PASS |
| Local Markdown-link check | PASS — 60 Markdown files checked |
| JSON parse check | PASS — 26 JSON files parsed |
| `git diff --check` | PASS |

The generated JCS fixture contains 493 passing assertions.

## Explicit limits and non-claims

- This is not a named-commit artifact. Under the slice 2.4 maturity rules it remains `PLANNED`, even though the current working tree passed builder verification.
- No independent reviewer has evaluated this delta. Builder verification is not acceptance.
- The JavaScript gate remains a TL-PX 0.1 cooperative reference. Its 0.2 policy helpers are contract oracles, not a 0.2 authority or PEP.
- The Rust authority only adds the shared policy-bundle hash helper. `AuthorityConfig` still receives a trusted, format-validated policy hash; it does not load, select, activate, or transactionally publish the new manifest.
- Trusted time is supplied to the selection oracle. A JavaScript `Date` calculation is not claimed as a production trusted or monotonic clock.
- The manifest binds policy content by media type and digest; this increment does not define every policy-language canonicalization profile.
- Authenticated transport, cryptographic operator identity, approval lifecycle, execution receipts, cancellation/reconciliation evidence, revocation evidence, and protected-execution mediation remain outside this increment.
- The base commit `c9bdd0f` still requires its own independent exact-commit review. This report does not broaden or satisfy that gate.

## Next gate

Deliberately commit the bounded slice 2.4 delta, rerun the builder matrix against that exact commit, and obtain an independent exact-commit contract/security/conformance review. Only then may the maturity table move slice 2.4 from `PLANNED` to `IMPLEMENTED`; acceptance remains a separate recorded decision.

**Later note (2026-08-17):** The bounded delta was committed locally as `a87f82271a12843c120d9a1e6ee238957f285c2f`. This report remains evidence from the pre-commit working tree. Exact-commit/full Section 3 verification and independent review are deferred until Section 3 is complete; the slice remains `PLANNED` and unaccepted.
