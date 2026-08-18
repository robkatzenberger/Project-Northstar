# Slice 3.1 Rust policy activation — builder verification

**Local date:** 2026-08-17 (America/Chicago)
**Base commit:** `c9bdd0fcd2d4c51fda9f3861724db0fd97524003`
**Dependency:** uncommitted slice 2.4 working-tree contract candidate
**Artifact reviewed:** uncommitted slice 3.1 working-tree delta on that dependency
**Disposition:** BUILDER-VERIFIED WORKING-TREE CANDIDATE — not committed, independently reviewed, or accepted

## Scope

This increment closes the policy-provenance boundary at the existing Rust authority without adding authenticated transport or execution. It adds:

- native Rust `PolicyBundleManifest` parsing, validation, JCS conversion, and cross-language manifest hashing;
- a deterministic Rust exact-match policy-content profile whose JCS digest must equal the manifest `content_hash`;
- a `PolicyCatalog` that validates all configured manifests, content bindings, duplicate identities, and complete supersession chains before the authority opens;
- exact tenant/environment/trusted-time selection for every new evaluation, with no implicit semantic-version precedence;
- durable `POLICY_UNAVAILABLE` and `POLICY_PRECEDENCE_AMBIGUOUS` evaluation errors that omit guessed policy identity and issue no authorization;
- policy ID/version/hash evidence sourced from the selected manifest rather than caller-supplied arbitrary hash configuration;
- claim-time reselection so retired, unavailable, ambiguous, or replaced policy blocks with `POLICY_INACTIVE`; and
- a pre-release database compatibility guard for evaluation errors that legitimately have no selected `policy_bundle_hash`.

The authority still performs no side effect. This is a policy activation boundary inside the local library, not an authenticated service or PEP.

## Acceptance cases exercised

The dedicated Rust slice suite contains six tests covering:

- native parsing of the shared slice 2.4 golden manifest and exact agreement with the JavaScript hash;
- decision and authorization evidence bound to the authority-selected manifest hash;
- startup rejection of exact-match content mismatch, predecessor-hash mismatch, and supersession cycles;
- durable unavailable-policy evidence with no fabricated policy identity;
- ambiguity when overlapping versions have no explicit supersession winner;
- explicit hash-bound supersession selecting the unique winner; and
- claim rejection after the issuing policy retires, while the claim window itself remains open.

The existing suites additionally exercise malformed policy-content activation, idempotent replay across configuration changes, active-policy claim checks, schema-shaped evidence, pre-release database compatibility, concurrency, restart, and sealed-outbox reconciliation.

## Verification environment

- Node.js `v25.5.0`
- npm `11.8.0`
- rustc `1.97.1 (8bab26f4f 2026-07-14)`
- cargo `1.97.1 (c980f4866 2026-06-30)`

## Executed checks

All commands ran from the local working tree and exited successfully.

| Check | Result |
| --- | --- |
| `npm test` | PASS — 763 assertions across the JS reference and 0.2 contract oracles |
| `npm run conformance` | PASS — TL-PX 0.1 Minimum Profile, 47 passed / 0 failed |
| `npm run tech-test` | PASS — 29 passed / 0 failed |
| `npm run redteam` | PASS for all in-scope attacks — 16 PASS / 0 FAIL / 4 documented deployment-boundary WARN |
| `npm run test:rust-evidence` | PASS — 4 Rust record types validate under the JavaScript 0.2 oracle |
| `cargo fmt --check` | PASS |
| `cargo test --all-targets --offline` | PASS — 50 tests: 23 authority, 12 evidence/outbox, 9 JCS/type/hash, 6 policy activation |
| `cargo clippy --all-targets --offline -- -D warnings` | PASS |
| Local Markdown-link check | PASS — 61 Markdown files checked |
| JSON parse check | PASS — 26 JSON files parsed |
| `git diff --check` | PASS |

## Explicit limits and non-claims

- This is not a named-commit artifact. Under the maturity rules it remains `PLANNED`, even though the current working tree passed builder verification.
- The 3.1 candidate depends on the still-uncommitted 2.4 contract delta. They require deliberate commit boundaries and separate exact-commit verification.
- No independent reviewer has evaluated this delta. Builder verification is not acceptance.
- Manifest issuer identity is a trusted local-configuration assertion. There is no publisher signature, key rotation, authenticated distribution, or revocation mechanism.
- `evaluate_and_issue_at` and `claim_at` accept trusted timestamps from the embedding for deterministic tests. This increment does not establish a protected monotonic production clock.
- Requester and executor strings still come from a trusted embedding. They are not authenticated by a Unix socket, OS peer credentials, mTLS, OIDC, or workload identity.
- Policy and capability configuration remain process-owned rather than transactionally published SQLite state. Multi-process configuration freshness is not established.
- The Rust exact-match content hash binds this implementation profile only. It does not make JavaScript 0.1 YAML the universal 0.2 policy language.
- Evaluation-error persistence now permits a null internal `policy_bundle_hash` when selection never succeeded. Pre-3.1 databases with the old non-null column are rejected before mutation and require a fresh database.
- Authenticated operator/canceller identity, approval lifecycle, execution receipts, cancellation/reconciliation evidence, revocation evidence, adapters, side effects, and forced mediation remain outside this increment.
- The base commit `c9bdd0f` still requires its own independent exact-commit review.

## Next gate

First capture and verify the bounded slice 2.4 contract at an exact named commit. Then capture this 3.1 Rust integration as a distinct named commit, rerun the full builder matrix against it, and obtain an independent contract/security/conformance review. Only then may slice 3.1 move from `PLANNED` to `IMPLEMENTED`; acceptance remains a separate recorded decision.

**Later note (2026-08-17):** The 2.4 dependency was committed locally as `a87f82271a12843c120d9a1e6ee238957f285c2f`, and this bounded Rust delta was committed locally as `1addb5c6a0ede31d754ac0bd47d7ef1f3a05e6d4`. This report remains evidence from the pre-commit working tree. Exact-commit/full Section 3 verification and independent review are deferred until Section 3 is complete; the slice remains `PLANNED` and unaccepted.
