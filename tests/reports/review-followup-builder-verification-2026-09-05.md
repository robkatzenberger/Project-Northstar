# Review follow-up — working-tree builder verification

**Date:** 2026-09-05

**Builder:** Codex

**Base HEAD:** `32c049e03d2c14729a8dc71205aef80e221e30ac`

**Tested state:** uncommitted working tree, including remediation already present when this review began

**Disposition:** recorded non-administrator checks pass; **CHANGES REQUESTED** remains the independent disposition

This is builder evidence for the working tree, not an exact-commit run or independent acceptance. The independently accepted implementation boundary remains `aed80e2527f05a3730b1057f2d90c55a6c3eb646`. No commit, push, administrator gate, temporary-account creation, or independent model review was performed in this follow-up.

## Review scope and changes

Read the session instructions, build sheet, August 20 builder-facing review, August 28 independent-review disposition and remediation evidence, and September 4 exact-candidate and administrator reports. The original review findings already have a remediation/disposition map; historical reports were preserved.

The incoming working tree already contained production-time sampling after SQLite write-transaction acquisition, clock-free runtime execution methods, a direct-completion retry fix, claim/lease lock-wait regressions, administrator-harness cleanup hardening, a stricter five-record evidence oracle, and broad documentation corrections. Those changes were preserved and included in the verification below. They were not authored afresh in this follow-up.

Two additional defects were reproduced and fixed:

| Finding | Reproducer before fix | Change and result |
| --- | --- | --- |
| Production reconciliation retries rejected identical results | Start a claimed execution, mark the outcome unknown, require reconciliation, finalize `COMPLETED_CONFIRMED`, wait 5 ms, and submit the identical result again. `runtime_reconciliation_retry_returns_the_durable_receipt` failed with `IDEMPOTENCY_CONFLICT`: reconciliation compared a fresh server time with the saved terminal timestamp. | Treat the production clock sample as observation metadata, matching direct-completion retry behavior. Explicit times in the deterministic test profile remain bound. The retry returns the exact saved receipt; a changed result still fails with `IDEMPOTENCY_CONFLICT`. No extra evidence row is created, execution remains terminal, and authorization remains consumed. |
| Evidence oracle accepted noncanonical input bytes | Prepend bytes `ef bb bf` to the retained `064717` JSONL and run the validator with the correct binary digest. The validator exited 0 and reported five valid rows because `TextDecoder` silently removed the BOM. | Preserve the BOM during strict UTF-8 decoding so JSON/JCS validation rejects the extra bytes. The canonical fixture still passes; the BOM mutation now fails. The suite contains one positive and 15 negative cases. |

Added `approval_samples_production_time_after_waiting_for_write_transaction`. It obtains a valid human-role approval presentation, holds a separate SQLite `BEGIN IMMEDIATE` across the approval deadline, and then releases the lock. Approval returns `APPROVAL_EXPIRED`, durable approval state is `Expired`, the authorization table contains zero rows, and evidence reconciliation succeeds. This test passed with the incoming deadline remediation; it closes a coverage gap rather than demonstrating another unfixed authorization defect. The fixture uses same-UID role mapping and does not prove OS identity separation or human interaction with an actual UI.

## Verification

Environment: macOS 26.6.2 (25G83); Rust `1.97.1 (8bab26f4f 2026-07-14)`; Cargo `1.97.1 (c980f4866 2026-06-30)`; Node `v25.5.0`; npm `11.8.0`.

Commands in the first group ran from `implementations/rust`:

| Command | Exit | Result |
| --- | ---: | --- |
| `cargo fmt --all -- --check` | 0 | Formatting passes |
| `cargo test --all-targets --offline --features deterministic-time` | 0 | 126 passed; one dedicated volume probe ignored |
| `cargo test --all-targets --offline` | 0 | 39 passed, including seven production-time tests |
| `cargo clippy --all-targets --offline --features deterministic-time -- -D warnings` | 0 | Strict Clippy passes |
| `cargo clippy --all-targets --offline -- -D warnings` | 0 | Strict Clippy passes |

The two Rust modes overlap; their counts are not counts of distinct tests. The initial sandboxed production suite was blocked from binding the local PEP Unix socket (`Operation not permitted`). The complete production and deterministic suites subsequently passed with authorized desktop socket permission. This was not root execution or the separate administrator-backed gate.

Commands in the second group ran from `implementations/javascript`:

| Command | Exit | Result |
| --- | ---: | --- |
| `npm test` | 0 | 935 existing checks plus 16 PEP-evidence cases: 951 total |
| `npm run conformance` | 0 | Frozen TL-PX 0.1: 47/47 |
| `npm run tech-test` | 0 | 29/29 |
| `npm run redteam` | 0 | 16 PASS, 0 FAIL, four existing WARN |
| `npm run test:rust-evidence` | 0 | 10 emitted records checked, zero failures |

The combined JS run includes the 495 JCS checks, 82 TL-PX 0.2 contract checks, policy compilation/ordering and typed-action checks, plus the expanded PEP oracle. The red-team warnings remain the historical cooperative reference's actor-type default, declared-metadata trust, direct-capability bypass, and test-only ephemeral escape hatch.

Commands in the final group ran from the repository root:

| Command | Exit | Result |
| --- | ---: | --- |
| `./tests/redteam-phase-4-remediation.sh` | 0 | Five focused rounds and twenty handoff rounds |
| `./tests/check-phase-4-review-package.sh` | 0 | Package indexing/ancestry/prose check passes |
| `./tests/redteam-phase-4-review-package.sh` | 0 | Four tampering cases rejected |
| `./tests/check-operational-profile.sh` | 0 | Local operations profile check passes |
| `sh -n run-northstar-3.9.sh` | 0 | Syntax only |
| `sh -n implementations/rust/scripts/restricted-agent-acceptance.sh` | 0 | Syntax only |
| `git diff --check` | 0 | Whitespace check passes |

The Rust matrix includes requester/executor handoff isolation, one-time claim races, restart/reconciliation, exact-action mutation, cooperative marker execution, and human-review-to-execution composition. These automated scenarios do not constitute a new independent two-model gate/auditor run. The ignored volume probe, administrator harness execution, and broader deployment/containment claims are outside these results.

## Evidence identity and remaining gate

The adjacent [source fingerprint file](./review-followup-builder-verification-2026-09-05.sha256) records the changed runtime, test, oracle, and harness files at verification. It supplements the base HEAD and does not substitute for committing and rerunning an exact candidate. Local full command logs and the two pre-fix reproducer logs were written under `/tmp/northstar-review-20260905/`; they are temporary debugging artifacts, not committed acceptance evidence.

Next: obtain owner approval for the completed working-tree candidate, commit the intended changes, rerun the full matrix and administrator-backed slice 3.9 gate at that exact commit, preserve the new report and canonical evidence, and obtain independent re-review. Prior administrator evidence `f025332` / `32c049e` / `064717` remains scoped to its observed marker scenario and does not cover this working tree.

No Section 3 or Phase 4 acceptance, production readiness, portable handoff, hostile same-UID containment, universal forced mediation, or HTTP/network-egress PEP is claimed. Portable revocation/expiry/handoff-link records, active cancellation after start, external audit transport and rotation, managed key custody, HA/DR, and formal checking remain open. No TL-PX 0.1 schema or historical evidence was rewritten.
