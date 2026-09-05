# Exact candidate 82f5cd6 — non-administrator builder verification

**Date:** 2026-09-05

**Builder:** Codex

**Tested source:** `82f5cd6cc505cc64bdea73eebeb535c2a1b4cd11`

**Git tree:** `903ab8661d4916ba26f5168213480d9c664235c9`

**Result:** the full recorded non-administrator matrix passed on a clean checkout of this exact commit. The administrator gate has not run at this hash. Independent disposition remains **CHANGES REQUESTED**.

The independently accepted implementation boundary remains `aed80e2527f05a3730b1057f2d90c55a6c3eb646`. This report is builder evidence for the named candidate, not independent acceptance. It was written outside the tested checkout after verification and is pending inclusion in a separate evidence commit.

## Frozen commit sequence

| Commit | Contents |
| --- | --- |
| `9b68bdb6d08d7dfa6dee865864dcecbfa9b9dd3d` | Rust transition-time sampling after transaction acquisition, durable time floor, retry semantics, and production lock-wait regressions |
| `77d77b8745b342d8f61b326a468c53523fe945a1` | PEP oracle and administrator harness hardening, canonical-byte and linkage checks, negative evidence tests |
| `82f5cd6cc505cc64bdea73eebeb535c2a1b4cd11` | Curated documentation, reviewer disposition, prior builder evidence, and GitHub delivery plan |

The existing history was preserved. The final commit includes all intended runtime, test, harness, and documentation changes. The future egress design draft was excluded. See the [current review packet](../../docs/reviews/CURRENT-REVIEW.md) and [owner-supplied reviewer disposition](../../docs/reviews/restricted-marker-review-disposition-2026-09-05.md).

## Test environment and source integrity

Environment: macOS 26.6.2 (25G83); Rust `1.97.1 (8bab26f4f 2026-07-14)`; Cargo `1.97.1 (c980f4866 2026-06-30)`; Node `v25.5.0`; npm `11.8.0`.

Tests ran in a separate local clone with full history and no hardlinks at `/private/tmp/northstar-exact-82f5cd6-cs3uhsq2/checkout`. Git status was clean before and after the matrix. All 224 tracked file SHA-256 digests were rechecked after verification; none changed. The excluded draft was absent. The clone still resolves to the source and tree hashes above.

Local command logs and the complete source fingerprint manifest are under `/private/tmp/northstar-exact-82f5cd6-cs3uhsq2/logs`. These are temporary diagnostic artifacts, not administrator evidence. Desktop sandbox permission allowed the tests to bind temporary Unix sockets; the tests did not run as root.

## Verification results

Rust commands ran from `implementations/rust`:

| Command | Exit | Result / local log |
| --- | ---: | --- |
| `cargo fmt --all -- --check` | 0 | Formatting passes; `rust-fmt.log` |
| `cargo test --all-targets --offline --features deterministic-time` | 0 | 126 passed, zero failed; one dedicated volume probe ignored; `rust-deterministic.log` |
| `cargo test --all-targets --offline` | 0 | 39 passed, zero failed, including seven production-time tests; `rust-production.log` |
| `cargo clippy --all-targets --offline --features deterministic-time -- -D warnings` | 0 | Strict Clippy passes; `clippy-deterministic.log` |
| `cargo clippy --all-targets --offline -- -D warnings` | 0 | Strict Clippy passes; `clippy-production.log` |
| `cargo build --offline --bin tlpx-run` | 0 | Default production-feature binary rebuilt after the test suites; `production-build.log` |

The Rust modes overlap; the counts are not counts of distinct tests. Coverage includes production claim, approval, and lease deadlines across write-lock waits; replay and single-use claim behavior; durable completion/reconciliation retries; authenticated socket request budgeting; requester/executor handoff; and evidence reconciliation. The ignored volume probe was not run.

JavaScript commands ran from `implementations/javascript`:

| Command | Exit | Result / local log |
| --- | ---: | --- |
| `npm test` | 0 | 951 checks: 935 existing checks plus one positive and 15 negative PEP oracle cases; `js-test.log` |
| `npm run conformance` | 0 | Frozen TL-PX 0.1: 47/47; `js-conformance.log` |
| `npm run tech-test` | 0 | 29/29; `js-tech-test.log` |
| `npm run redteam` | 0 | 16 PASS, zero FAIL, four existing WARN; `js-redteam.log` |
| `npm run test:rust-evidence` | 0 | 10 records, zero failures; `js-rust-evidence.log` |

The combined suite includes 495 JCS checks and 82 TL-PX 0.2 contract checks. The four red-team warnings remain the historical cooperative reference's actor-type default, declared-metadata trust, direct-capability bypass, and test-only ephemeral escape hatch.

Repository-root checks:

| Command or check | Exit | Result / local log |
| --- | ---: | --- |
| `./tests/check-phase-4-review-package.sh` | 0 | Package indexing, ancestry, and prose pass; `package-check.log` |
| `./tests/redteam-phase-4-review-package.sh` | 0 | Four tampering cases rejected; `package-redteam.log` |
| `./tests/check-operational-profile.sh` | 0 | Local operations profile passes; `operations.log` |
| `./tests/redteam-phase-4-remediation.sh` | 0 | Five focused rounds and twenty handoff rounds pass; `remediation-redteam.log` |
| `sh -n run-northstar-3.9.sh` | 0 | Shell syntax only |
| `sh -n implementations/rust/scripts/restricted-agent-acceptance.sh` | 0 | Shell syntax only |
| `./run-northstar-3.9.sh --check` | 0 | Prerequisites report READY; `admin-prerequisites.log`; no administrator gate executed |
| `git diff --check` | 0 | Whitespace passes |
| Changed-document relative links and tracked-input closure | 0 | Targets resolve within the candidate; excluded draft absent |
| Final tracked-file digest comparison and Git status | 0 | 224 digests match; checkout clean |

## Administrator and review boundary

The locally built default-feature `tlpx-run` has SHA-256 `sha256:7c6332d3066b4b0bab5dd12fa0354d9d882a84899a71255494fab5e4673fad85`. This identifies the build checked by `--check`; it is not evidence that the administrator harness staged or executed that binary.

Still required: run `sudo ./run-northstar-3.9.sh` from a clean checkout of the exact source hash, retain its dated report, canonical JSONL, binary/evidence digests, and cleanup result, then give the existing reviewer this source hash and the complete evidence packet. Record the full source hash and new evidence paths in an evidence-only descendant without modifying the tested source. No fourth commit, administrator run, temporary-account creation, independent re-review, or GitHub push was performed in this verification step.

Historical report `064717` and its retained JSONL remain evidence for `f0253328e00fd1868e4de811298b8ad2e91d230d`, preserved by `32c049e03d2c14729a8dc71205aef80e221e30ac`. They do not cover this candidate or close its administrator gate. `4ca86d6` remains a superseded historical candidate.

Any future acceptance must be limited to the named bounded macOS separate-identity marker profile. Phase 4, Section 3, network/egress mediation, hostile same-UID containment, production readiness, portable handoff, and universal forced mediation remain open.
