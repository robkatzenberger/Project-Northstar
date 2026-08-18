# Slice 3.6 execution lifecycle and reconciliation — builder verification

**Date:** 2026-08-18
**Builder:** Codex
**Initial artifact under test:** uncommitted working tree based on `4c6b3d2`
**Named artifact:** local commit `9028346`
**Disposition:** full exact-commit builder matrix passed; not independently reviewed or accepted

## Scope exercised

- The previously provisional `tlpx.execution` schema is narrowed to terminal receipts. `EXECUTION_OUTCOME_UNKNOWN` and `RECONCILIATION_REQUIRED` are durable authority-process states, not receipt states.
- Every terminal receipt requires at least one bounded result summary, canonical result digest, or bounded external evidence reference. Evidence-chain hashes and HMAC seals remain storage-envelope metadata rather than private record fields.
- Cancellation receipts can name the observed cancellation distinction from the normative lifecycle.
- Authorization evidence now carries the execution idempotency key bound to authorization ID, authenticated executor, and Authorized Action hash.
- An authenticated executor durably starts one execution attempt before a protected side effect. Exact retries return existing state; changed keys, executor, or adapter binary digest fail closed.
- Direct `COMPLETED`, `FAILED`, and `CANCELLED` outcomes are one-winner terminal transitions.
- An unstarted expired claim emits `LEASE_EXPIRED`. An expired or crashed started execution becomes `EXECUTION_OUTCOME_UNKNOWN`, then an authenticated reconciler must move it through `RECONCILIATION_REQUIRED` to `COMPLETED_CONFIRMED`, `FAILED_CONFIRMED`, or `OUTCOME_UNKNOWN_FINAL`.
- No unknown outcome reopens authorization or automatically retries a side effect.
- Terminal state and one canonical schema-valid `tlpx.execution` row enter SQLite and the hash-chained HMAC-sealed outbox in the same transaction.

## Environment

- Node.js `v25.5.0`
- npm `11.8.0`
- rustc `1.97.1 (8bab26f4f 2026-07-14)`
- cargo `1.97.1 (c980f4866 2026-06-30)`

## Commands and results

From `implementations/rust`:

```text
cargo fmt --all -- --check                              PASS
cargo test --all-targets --offline                     PASS (73 tests)
cargo clippy --all-targets --offline -- -D warnings    PASS
```

The 42 authority tests cover exact execution retries, changed-retry rejection, executor binding, bounded results, cancellation-state consistency, lease recovery, restart persistence, unknown-outcome reconciliation, and a two-connection terminal race. The 13 evidence tests include injected execution-outbox failure rollback.

From `implementations/javascript`:

```text
npm test                         PASS (927 checks across the combined suite)
npm run conformance             PASS (47/47 TL-PX 0.1)
npm run tech-test               PASS (29/29)
npm run redteam                 PASS (16 PASS, 0 FAIL, 4 documented WARN)
npm run test:rust-evidence      PASS (10/10 records)
```

The combined suite includes 76/76 TL-PX 0.2 contract checks. The cross-language evidence test validates a real canonical Rust terminal execution receipt with the JavaScript 0.2 schema oracle.

## Fail-closed evidence

- Starting with a key not derived from the exact authorization/executor/action binding is rejected.
- Repeating a started or terminal execution returns durable state and never creates a second attempt or receipt.
- A changed terminal retry returns `IDEMPOTENCY_CONFLICT` and cannot rewrite the first result.
- Missing, empty, oversized, or malformed result evidence cannot create a terminal receipt.
- Only the authenticated named executor may directly finish; only a mapped reconciler may establish or resolve reconciliation-required state.
- Direct completion after unknown outcome is rejected with `RECONCILIATION_REQUIRED`.
- An unstarted expired claim is distinguished from a started execution whose external outcome may be unknown.
- Two SQLite connections racing different terminal outcomes produce one winner.
- Injected outbox failure rolls back terminal state and sequence; a later successful attempt creates the first durable terminal outcome.

## Explicit limits and non-claims

- This slice supplies authority state and the execution protocol boundary; it performs no protected side effect and does not force a caller to use it. `tlpx-run` and OS-level mediation remain slices 3.8 and 3.9.
- The optional adapter binary digest is trusted caller input until slice 3.7 authenticates adapter identity and verifies configured integrity material.
- The schema represents terminal cancellation observations, but a live adapter cancellation request/acknowledgement handshake is not implemented here.
- The library exposes explicit expiry/recovery operations but has no background lease sweeper. The later PEP/service must invoke recovery so abandoned claims cannot remain operationally unresolved.
- Deterministic `_at` methods use trusted embedding time for tests. Production monotonic elapsed-time enforcement and uncertainty handling remain hardening work.
- Reconciliation authority is a bounded local UID/GID-mapped role. There is no MFA, external investigator workflow, or independent protected-system confirmation adapter.
- Portable revocation evidence, approval-expiry evidence, and unknown/reconciliation transition records remain absent. Only the final execution receipt is portable and sealed.
- The pre-release SQLite schema requires a fresh database after this trust-path change.
- Passing this matrix does not establish an authenticated adapter, protected-execution PEP, TL-PX 0.2 runtime conformance, independent review, or acceptance.

## Exact-commit rerun

After local commit `9028346`, the complete Rust and JavaScript command matrix above was rerun without source working-tree changes. Results matched the pre-commit run: 73 Rust tests, clean formatting and clippy, 927 combined JavaScript checks, 47/47 TL-PX 0.1 conformance, 76/76 TL-PX 0.2 contract checks, 29/29 technical checks, 16 PASS / 0 FAIL / 4 documented WARN in the historical JavaScript red team, and 10/10 Rust evidence rows.

## Next gate

Continue to slice 3.7 for an authenticated, versioned, integrity-bound adapter contract and mutation/incomplete-translation tests. Full Section 3 verification and independent review remain deferred until slices 3.1–3.9 are complete.
