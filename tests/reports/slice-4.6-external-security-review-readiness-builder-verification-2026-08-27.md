# Slice 4.6 external security review readiness — builder verification

**Date:** 2026-08-27
**Builder:** Codex
**Review-package commit:** `d51e46ce4cf87e4ab339fd1d70aac5e078c3e7e0`
**Security-code target:** `64d08203c2f30964b176be8872b390ba3c28e5e6`
**Disposition:** Phase 4 is builder-complete and ready for independent review; it is not independently accepted

## Scope delivered

- [`../../docs/reviews/phase-4-external-security-review-readiness-2026-08-27.md`](../../docs/reviews/phase-4-external-security-review-readiness-2026-08-27.md) pins the exact Phase 4 base, security-code target, commit history, trust boundaries, attack surfaces, reproduction matrix, known gaps, required reviewer challenges, and expected independent deliverable.
- [`../../docs/reviews/phase-4-review-scope.txt`](../../docs/reviews/phase-4-review-scope.txt) pins the 33 paths changed in the security-code range.
- `tests/check-phase-4-review-package.sh` verifies the named history and manifest, required builder reports, reproduction commands, open-gate language, absence of false acceptance phrases, and a narrow secret-pattern check.
- `tests/redteam-phase-4-review-package.sh` proves the package check rejects a substituted target, removed 3.9 non-claim, incomplete file manifest, and injected false acceptance statement.
- Slice 4.6 changes no authority, schema, protocol, policy, execution, or evidence behavior.

## Review-package red-team results

| Package attack | Result |
| --- | --- |
| Replace the exact security-code target with a fake hash | Rejected |
| Remove the statement that the administrator-backed 3.9 gate was not run | Rejected |
| Omit one changed file from the review manifest | Rejected |
| Append an unqualified Phase 4 acceptance claim | Rejected |
| Remove a required commit, report, command, challenge section, or non-claim | Package check rejects the omission |
| Add an obvious private-key block or GitHub personal token pattern | Package check rejects the package |

## Exact-commit commands and results

From the repository root at `d51e46ce4cf87e4ab339fd1d70aac5e078c3e7e0`:

```text
sh -n tests/check-phase-4-review-package.sh      PASS
sh -n tests/redteam-phase-4-review-package.sh    PASS
./tests/check-phase-4-review-package.sh          PASS
./tests/redteam-phase-4-review-package.sh        PASS (4/4 tampering cases rejected)
sh -n tests/check-operational-profile.sh         PASS
./tests/check-operational-profile.sh             PASS
```

From `implementations/rust` at the same history:

```text
cargo fmt --all -- --check                              PASS
cargo test --all-targets --offline                     PASS (114 tests)
cargo clippy --all-targets --offline -- -D warnings    PASS
sh -n scripts/restricted-agent-acceptance.sh           PASS
```

The ordinary restricted-service integration test ran outside the desktop filesystem sandbox so its Unix socket could bind. This was not the administrator-backed slice 3.9 acceptance run.

From `implementations/javascript` at the same history:

```text
npm test                         PASS (935 checks across the combined suite)
npm run conformance             PASS (47/47 TL-PX 0.1)
npm run tech-test               PASS (29/29)
npm run redteam                 PASS (16 PASS, 0 FAIL, 4 documented WARN)
npm run test:rust-evidence      PASS (10/10 records)
```

The combined JavaScript suite includes 82/82 TL-PX 0.2 contract checks.

## Phase 4 builder disposition

| Slice | Builder status | Independent status |
| --- | --- | --- |
| 4.1 separated role keys | Exact-commit matrix and focused red team passed through `efa7f0f` | Deferred |
| 4.2 durable local audit export | Exact-commit matrix and focused red team passed at `833d8d4` | Deferred |
| 4.3 race/time/crash assurance | Exact-commit matrix and focused red team passed at `eb0e624` | Deferred |
| 4.4 operations/incident profile | Exact-commit matrix and focused red team passed at `bdd8a00` | Deferred |
| 4.5 non-transitive handoff | Exact-commit matrix and focused red team passed at `536111a` | Deferred |
| 4.6 external-review readiness | Exact-commit full matrix and package red team passed at `d51e46c` | Deferred |

This completes the planned Phase 4 build slices at the builder-evidence level. It does not satisfy the build sheet's independent-adversarial completion criterion.

## Explicit limits and non-claims

- The package is builder-authored and cannot independently validate itself. Its checks prove completeness against pinned text and history, not security correctness.
- `sudo ./scripts/restricted-agent-acceptance.sh` was not run. Slice 3.9, forced mediation, alternate-route resistance, and Section 3 completion remain unverified.
- Independent review of Phase 4 has not yet occurred. No Phase 4 slice is accepted by this report.
- The security-code target intentionally ends at `64d0820`; the review-package commit `d51e46c` adds documentation and check scripts only.
- Portable execution-side evidence closure, active post-claim cancellation, production audit transport/rotation/retention, HA/DR automation, KMS/HSM custody, external time attestation, formal checking, and cross-authority handoff remain outside the verified scope.
- Builder testing is not certification or production readiness.

## Next gate

Give the package commit `d51e46ce4cf87e4ab339fd1d70aac5e078c3e7e0` and security-code target `64d08203c2f30964b176be8872b390ba3c28e5e6` to an independent reviewer. Record findings and a per-slice disposition in a new dated report. Do not revise historical builder reports or claim Phase 4 acceptance before that review.
