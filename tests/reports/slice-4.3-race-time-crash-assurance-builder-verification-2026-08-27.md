# Slice 4.3 race, trusted-time, and crash assurance — builder verification

**Date:** 2026-08-27  
**Builder:** Codex  
**Implementation commit:** `eb0e624dd9f94672a9dab825f1e01df7c6a0dab8`  
**Disposition:** full exact-commit builder and focused adversarial matrix passed; independent review and acceptance deferred until Phase 4 is complete

## Scope exercised

- Normal in-process authority time is anchored once to wall time and then advances from monotonic elapsed time, preventing a later wall-clock rollback from extending a live claim or execution window.
- A durable SQLite trusted-time watermark participates in authority transactions. Successful time-bearing transitions cannot move behind the last committed authority time, including after reopening the database or through a second authority connection.
- Negative and backward trusted-time inputs return `TRUSTED_TIME_INVALID` without consuming an evaluation slot or advancing the authority sequence.
- An exact idempotent evaluation replay may return its already-committed immutable result even if the supplied display time is older; it creates no new permission or transition.
- Claim and approval deadline tests cover the final valid millisecond and the exact expiration boundary. The exact boundary fails closed.
- Repeated independent-connection races cover claim versus authorization revocation and approval versus requester cancellation. Exactly one terminal transition wins.
- Restart assurance closes and reopens the database after issuance, claim, execution start, lease-expiry recovery, reconciliation requirement, and final unknown-outcome resolution. Permission never reopens and terminal evidence remains reconciled.
- Existing tests continue to cover concurrent idempotent evaluation, double claim, approve/reject, cancel/expire, execution terminal races, SQLite trigger-injected rollback, append-before-ack recovery, replay, and one cooperative side effect.

## Focused red-team results

| Attack or failure | Result |
| --- | --- |
| New evaluation with time before the durable watermark | `TRUSTED_TIME_INVALID`; no event or sequence added |
| Repeat rollback after database restart | `TRUSTED_TIME_INVALID` |
| Negative trusted time | `TRUSTED_TIME_INVALID`; empty authority remains empty |
| Exact immutable replay with an older timestamp | Original result returned; no new transition or authorization |
| Claim one millisecond before expiry | One claim succeeds |
| Claim exactly at expiry | `AUTHORIZATION_EXPIRED` |
| Approve one millisecond before approval expiry | Approval succeeds and issues a fresh authorization |
| Approve exactly at approval expiry | `APPROVAL_EXPIRED`; no authorization |
| Claim versus revocation on separate connections | Exactly one winner; final state is `CLAIMED` or `REVOKED` |
| Approval versus requester cancellation on separate connections | Exactly one winner; final state is `APPROVED` or `CANCELLED` |
| Restart after every execution boundary | Consumed claim reaches `OUTCOME_UNKNOWN_FINAL`; begin retry is non-permission |
| Reconcile after every race/restart sequence | Sealed authority/evidence coverage remains valid |

The repeated focused race command ran five times. Each run executes 12 claim/revocation races and 12 approval/cancellation races, for 120 focused race iterations total, in addition to the full suite's other concurrency cases.

## Exact-commit commands and results

From `implementations/rust` at `eb0e624dd9f94672a9dab825f1e01df7c6a0dab8`:

```text
cargo fmt --all -- --check                              PASS
cargo test --all-targets --offline                     PASS (104 tests)
cargo clippy --all-targets --offline -- -D warnings    PASS
sh -n scripts/restricted-agent-acceptance.sh           PASS
focused race suite, five consecutive runs              PASS (120 race iterations)
trusted-time, deadline, restart focused cases          PASS
```

The ordinary restricted-service integration test ran outside the desktop filesystem sandbox so its Unix socket could bind. This was not the separate administrator-backed slice 3.9 acceptance run.

From `implementations/javascript` at the same history:

```text
npm test                         PASS (935 checks across the combined suite)
npm run conformance             PASS (47/47 TL-PX 0.1)
npm run tech-test               PASS (29/29)
npm run redteam                 PASS (16 PASS, 0 FAIL, 4 documented WARN)
npm run test:rust-evidence      PASS (10/10 records)
```

The combined JavaScript suite includes 82/82 TL-PX 0.2 contract checks.

## Explicit limits and non-claims

- This is deterministic integration and adversarial testing, not exhaustive formal model checking, property-based generation over every state, a concurrency model checker, parser fuzzing, or proof of linearizability.
- Restart tests close/reopen SQLite and use transaction failure injection. They do not simulate kernel panic, torn SQLite pages, storage-controller lies, sudden power loss, or filesystem durability outside SQLite's `FULL` synchronization contract.
- The wall-time anchor is established at process start and the durable watermark detects committed rollback across connections/restarts. External clock attestation, leap-smear policy, forward-jump uncertainty bounds, NTP health, and portable skew negotiation remain deployment/operational work.
- Public deterministic `*_at` APIs remain trusted-embedding/test seams. A service must not expose caller-selected time as authenticated authority time.
- Active post-claim cancellation remains unimplemented. The cancellation races here cover pending approval; execution cancellation fields continue to describe outcomes observed by the executor path.
- The administrator-backed separate-OS-identity slice 3.9 acceptance script was not run. This slice does not establish forced mediation or Section 3 completion.
- Builder testing is not independent review or acceptance.

## Next gate

Build slice 4.4: operational readiness and incident response. Define startup, health, backup/restore, exporter backpressure, stale-lock/torn-row recovery, key compromise, clock uncertainty, evidence corruption, unknown outcome, rollback, and escalation procedures without overstating automated recovery.
