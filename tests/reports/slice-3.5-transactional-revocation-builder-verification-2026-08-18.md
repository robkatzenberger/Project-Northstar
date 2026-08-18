# Slice 3.5 transactional revocation and claim — builder verification

**Date:** 2026-08-18
**Builder:** Codex
**Initial artifact under test:** uncommitted working tree based on `94b9e97`
**Named artifact:** local commit `ad95653`
**Disposition:** full exact-commit builder matrix passed; not independently reviewed or accepted

## Scope exercised

- SQLite retains one immutable authority-local revocation row with an authority sequence, authenticated revoker, stable scope, stable reason, and trusted timestamp.
- The kernel-authenticated emergency-authority role may revoke; other authenticated local roles fail closed before state changes.
- Authorization, requester/executor principal, policy-bundle hash, tenant, environment, and capability revocations are checked inside the same `BEGIN IMMEDIATE` transaction that validates and consumes an authorization.
- Authorization-specific revocation atomically changes only `AUTHORIZED_UNCLAIMED` to `REVOKED`.
- Exact duplicate revocations fail closed instead of rewriting prior state.
- Revocation survives authority restart.
- Claim versus authorization revocation across separate SQLite connections has exactly one winner. A winning claim remains consumed; a winning revocation makes every later claim return `AUTHORIZATION_REVOKED`.
- The existing executor/action/capability/scope/policy checks and one-time compare-and-set claim remain intact.

## Environment

- Node.js `v25.5.0`
- npm `11.8.0`
- rustc `1.97.1 (8bab26f4f 2026-07-14)`
- cargo `1.97.1 (c980f4866 2026-06-30)`

## Commands and results

From `implementations/rust`:

```text
cargo fmt --all -- --check                              PASS
cargo test --all-targets --offline                     PASS (67 tests)
cargo clippy --all-targets --offline -- -D warnings    PASS
```

The 37 authority tests include all implemented claim-time revocation scopes, unauthorized revocation, duplicate immutability, restart persistence, authorization revocation versus claim, and the existing one-time/concurrent claim cases.

From `implementations/javascript`:

```text
npm test                         PASS (919 checks across the combined suite)
npm run conformance             PASS (47/47 TL-PX 0.1)
npm run tech-test               PASS (29/29)
npm run redteam                 PASS (16 PASS, 0 FAIL, 4 documented WARN)
npm run test:rust-evidence      PASS (9/9 records)
```

The combined suite includes 68/68 TL-PX 0.2 contract checks. The Rust evidence crosscheck remains deliberately limited to the accepted evaluation/authorization record core.

## Fail-closed evidence

- A requester without the emergency-authority role cannot revoke and leaves authorization state unchanged.
- A scoped revocation observed before claim blocks consumption with `AUTHORIZATION_REVOKED`.
- The revocation lookup and claim compare-and-set share one write transaction; there is no revocation check/claim gap.
- An authorization-specific revocation cannot rewrite claimed, expired, or otherwise terminal authority state.
- A repeated exact scope cannot replace the original actor, reason, timestamp, or sequence.
- Separate authority processes sharing one database cannot both win the revocation/claim race.

## Explicit limits and non-claims

- There is still no accepted `tlpx.revocation` schema. `tlpx_revocations` is authority-local transactional state, not a portable TL-PX 0.2 record, and it is not added to the sealed evidence outbox under a private record type.
- Because no portable revocation record exists, the authority sequence allocated to the local row is absent from portable exports. Complete authority-stream evidence remains impossible until the revocation contract closes.
- The local row is protected by SQLite transactionality and API immutability but is not independently HMAC-sealed as a portable record. That evidence-integrity closure remains deferred with the schema.
- The `SIGNING_KEY` scope can be retained for the later key-lifecycle increment, but the current authority issues no signed authorization and therefore has no authorization-signing key ID to check at claim. Slice 4.1 still owns separated signing keys, rotation, and enforceable key revocation.
- Claimed work is never unclaimed. Post-claim cancellation, execution leases, terminal execution outcomes, and reconciliation are slice 3.6 and later work.
- The raw `revoke_at` compatibility method remains a trusted-embedding seam. Untrusted local callers must use the authenticated facade.
- This is not a service, authenticated adapter, execution receipt, protected side effect, forced-mediation PEP, independent review, or acceptance.

## Exact-commit rerun

After local commit `ad95653`, the complete Rust and JavaScript command matrix above was rerun without source working-tree changes. Results matched the pre-commit run: 67 Rust tests, clean formatting and clippy, 919 combined JavaScript checks, 47/47 TL-PX 0.1 conformance, 68/68 TL-PX 0.2 contract checks, 29/29 technical checks, 16 PASS / 0 FAIL / 4 documented WARN in the historical JavaScript red team, and 9/9 Rust evidence rows.

## Next gate

Continue to slice 3.6 for execution idempotency, terminal execution state, receipts, and honest unknown-outcome reconciliation. The execution-side schema contract must be closed with that emitter before any TL-PX 0.2 execution-evidence conformance claim. Full Section 3 verification and independent review remain deferred until slices 3.1–3.9 are complete.
