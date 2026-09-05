# Northstar test evidence

This directory keeps durable reports from independent or adversarial test runs so future modifications can reference a known baseline.

## Current packet

- [Current review packet](../docs/reviews/CURRENT-REVIEW.md) — source/evidence identity and exact-candidate gates.
- [Owner-supplied reviewer disposition, 2026-09-05](../docs/reviews/restricted-marker-review-disposition-2026-09-05.md) — changes requested; frozen hash and new administrator evidence required.
- [Working-tree builder report](./reports/review-followup-builder-verification-2026-09-05.md) — full local matrix before a source-candidate commit; not independent acceptance.

## Historical evidence and reproduction inputs

- [2026-09-05 review follow-up](./reports/review-followup-builder-verification-2026-09-05.md) — reproduced reconciliation-retry/BOM defects, fixes, approval-expiry lock-wait coverage, and the full non-administrator working-tree matrix; no exact-commit or independent acceptance.
- [`reports/northstar-two-agent-test-proof.md`](./reports/northstar-two-agent-test-proof.md) — two-agent requester/execution-gate test of commit `ca05f6996534471e817d11f3c668e38411797fb8`.
- [`reports/phase-1-policy-compile-2026-08-14.md`](./reports/phase-1-policy-compile-2026-08-14.md) — Phase 1 compile suite as first recorded; contained in `31175c5`. Do not treat its “uncommitted / no commit” line as accurate.
- [`reports/phase-1-crosscheck-2026-08-14.md`](./reports/phase-1-crosscheck-2026-08-14.md) — independent crosscheck follow-up: WeakSet compile identity, `policy_pack_id` reject, 0.2 claim-record clarifications.
- [`reports/slice-2.4-policy-ordering-builder-verification-2026-08-17.md`](./reports/slice-2.4-policy-ordering-builder-verification-2026-08-17.md) — pre-commit builder matrix and explicit limits for local slice 2.4 commit `a87f822`; not exact-commit acceptance.
- [`reports/slice-3.1-rust-policy-activation-builder-verification-2026-08-17.md`](./reports/slice-3.1-rust-policy-activation-builder-verification-2026-08-17.md) — pre-commit builder matrix and explicit limits for local slice 3.1 commit `1addb5c`; not exact-commit acceptance.
- [`reports/slice-3.2-typed-action-hash-builder-verification-2026-08-17.md`](./reports/slice-3.2-typed-action-hash-builder-verification-2026-08-17.md) — bounded exact-commit matrix and limits for local 3.2 typed-action/hash commit `e835c4e`; not full Section 3 or acceptance.
- [`reports/slice-3.3-local-auth-cancellation-builder-verification-2026-08-18.md`](./reports/slice-3.3-local-auth-cancellation-builder-verification-2026-08-18.md) — full exact-commit builder matrix and limits for local 3.3 commit `c19b1d2`.
- [`reports/slice-3.4-approval-expiry-builder-verification-2026-08-18.md`](./reports/slice-3.4-approval-expiry-builder-verification-2026-08-18.md) — full exact-commit builder matrix and limits for local 3.4 commit `133cd94`.
- [`reports/slice-3.5-transactional-revocation-builder-verification-2026-08-18.md`](./reports/slice-3.5-transactional-revocation-builder-verification-2026-08-18.md) through [`reports/slice-3.9-restricted-pep-candidate-unverified-2026-08-27.md`](./reports/slice-3.9-restricted-pep-candidate-unverified-2026-08-27.md) — historical Section 3 builder evidence; the 3.9 pre-gate report is superseded for current status.
- [`reports/slice-3.9-administrator-gate-2026-09-04-064717.md`](./reports/slice-3.9-administrator-gate-2026-09-04-064717.md) and its [canonical JSONL](./reports/slice-3.9-administrator-gate-2026-09-04-064717.evidence.jsonl) — source `f025332`, preserved by evidence commit `32c049e`; the bounded marker gate passed, but a later lock-wait deadline finding blocks acceptance and requires a new exact-candidate run.
- [`reports/slice-4.1-separated-key-roles-builder-verification-2026-08-27.md`](./reports/slice-4.1-separated-key-roles-builder-verification-2026-08-27.md) through [`reports/slice-4.6-external-security-review-readiness-builder-verification-2026-08-27.md`](./reports/slice-4.6-external-security-review-readiness-builder-verification-2026-08-27.md) — point-in-time exact-commit Phase 4 builder/red-team evidence; independent review later returned changes requested, so these reports are not acceptance evidence.
- [`../docs/reviews/phase-4-external-security-review-readiness-2026-08-27.md`](../docs/reviews/phase-4-external-security-review-readiness-2026-08-27.md) — pinned independent-review challenge brief for security-code target `64d0820` and package commit `d51e46c`.
- [`../docs/reviews/phase-4-independent-review-disposition-2026-08-28.md`](../docs/reviews/phase-4-independent-review-disposition-2026-08-28.md) — independent **changes requested** disposition plus a finding-by-finding remediation map; not acceptance.
- [`reports/phase-4-remediation-builder-verification-2026-08-28.md`](./reports/phase-4-remediation-builder-verification-2026-08-28.md) — historical exact-commit non-privileged evidence for remediation `ee720d4`; later candidates and findings supersede it for current status.
- [`redteam-phase-4-remediation.sh`](./redteam-phase-4-remediation.sh) — executable focused multipliers for the Phase 4 reviewer reproducers (five security repetitions and twenty handoff repetitions by default); it intentionally does not run the administrator-backed 3.9 identity test.
- [`fixtures/tlpx-0.2/jcs/`](./fixtures/tlpx-0.2/jcs/) — accepted slice 2.2 JCS fixtures. JS `test-jcs.mjs` and Rust `cargo test` must both match.
- [`fixtures/tlpx-0.2/policy/`](./fixtures/tlpx-0.2/policy/) — slice 2.4 candidate policy-manifest/hash fixture. JS and Rust must match; this is not runtime acceptance.
- [`fixtures/tlpx-0.2/actions/`](./fixtures/tlpx-0.2/actions/) — slice 3.2 candidate typed action, canonical byte, distinct hash, and exact Action Binding fixture. JS and Rust must match; this is not runtime acceptance.

## Referencing this baseline

For changes to policy evaluation, Switchboard decisions, audit integrity, approvals, execution enforcement, schemas, or conformance:

1. Run `npm test`, `npm run conformance`, `npm run tech-test`, and `npm run redteam` from `implementations/javascript/`.
2. Re-run the relevant two-party scenario from the report.
3. Record the tested commit SHA, runtime versions, exit codes, receipt outcomes, audit verification, and any changed findings in a new dated report under `tests/reports/`.
4. Do not overwrite historical reports; they are point-in-time evidence.

The reports demonstrate observed behavior, not certification. In particular, mediated execution and unavoidable enforcement are different security claims.
