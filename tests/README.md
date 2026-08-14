# Northstar test evidence

This directory keeps durable reports from independent or adversarial test runs so future modifications can reference a known baseline.

## Current baseline

- [`reports/northstar-two-agent-test-proof.md`](./reports/northstar-two-agent-test-proof.md) — two-agent requester/execution-gate test of commit `ca05f6996534471e817d11f3c668e38411797fb8`.
- [`reports/phase-1-policy-compile-2026-08-14.md`](./reports/phase-1-policy-compile-2026-08-14.md) — Phase 1 compile suite as first recorded; contained in `31175c5`. Do not treat its “uncommitted / no commit” line as accurate.
- [`reports/phase-1-crosscheck-2026-08-14.md`](./reports/phase-1-crosscheck-2026-08-14.md) — independent crosscheck follow-up: WeakSet compile identity, `policy_pack_id` reject, 0.2 claim-record clarifications.
- [`fixtures/tlpx-0.2/jcs/`](./fixtures/tlpx-0.2/jcs/) — slice 2.2 JCS / `sha256:` golden fixtures (not 0.1 conformance).

## Referencing this baseline

For changes to policy evaluation, Switchboard decisions, audit integrity, approvals, execution enforcement, schemas, or conformance:

1. Run `npm test`, `npm run conformance`, `npm run tech-test`, and `npm run redteam` from `implementations/javascript/`.
2. Re-run the relevant two-party scenario from the report.
3. Record the tested commit SHA, runtime versions, exit codes, receipt outcomes, audit verification, and any changed findings in a new dated report under `tests/reports/`.
4. Do not overwrite historical reports; they are point-in-time evidence.

The reports demonstrate observed behavior, not certification. In particular, mediated execution and unavoidable enforcement are different security claims.

