# Slice 4.4 operational readiness and incident response — builder verification

**Date:** 2026-08-27  
**Builder:** Codex  
**Implementation/profile commit:** `bdd8a00b7cc6e97b80f36c34ea8dad7e8cfb879b`  
**Disposition:** full exact-commit builder and focused adversarial matrix passed; independent review and acceptance deferred until Phase 4 is complete

## Scope exercised

- The local operations profile defines a fail-closed readiness decision, aggregate health signals, audit backpressure thresholds, quiesced backup/restore, change/rollback rules, and explicit deployment non-claims.
- The incident-response profile defines first response, severity, evidence/database corruption, stale lock, torn row, capacity hard stop, key compromise, time uncertainty, unknown outcome, identity/alternate-path bypass, recovery, and closure procedures.
- `Authority::operational_snapshot()` returns only aggregate state. It performs SQLite `quick_check`, foreign-key checking, complete sealed evidence reconciliation, and coverage verification before reporting the trusted-time watermark, evidence totals/backlog, pending approvals, unclaimed authorizations, started/unknown/reconciliation-required executions, and active revocation count.
- The snapshot fails closed on an injected foreign-key orphan and does not expose principals, action content, result content, or key bytes.
- The audit exporter now checks projected size before append and will not write a row that crosses `MAX_AUDIT_SINK_BYTES` (64 MiB).
- A repository check pins the required operational topics and rejects destructive filesystem/database instructions in these runbooks.

## Focused red-team results

| Attack or operational mistake | Result |
| --- | --- |
| Treat a failed health call as an empty healthy snapshot | Profile explicitly forbids it; API returns an error |
| Inject a foreign-key orphan | `OPERATIONAL_INTEGRITY_FAILED` |
| Corrupt sealed evidence | Snapshot's reconciliation fails closed through the existing evidence checks |
| Append a row that would cross 64 MiB | Rejected before write |
| Delete a stale lock based only on age | Explicitly forbidden; stop writers, preserve, verify, quarantine with authorization |
| Truncate or add a newline to a torn row | Explicitly forbidden; preserve and investigate/restore |
| Rotate to an empty sink while database rows are acknowledged | Explicitly identified as unsafe and unsupported |
| Manually mark backlog rows exported | Explicitly forbidden; only the verified exporter may acknowledge after sync |
| Restore state to make a consumed authorization usable | Explicitly forbidden |
| Automatically retry an unknown external effect | Explicitly forbidden; authorization remains consumed |
| Delete a key revocation to recover verification | Explicitly forbidden |
| Hide same-user/alternate-path limitations | Readiness and incident profile require separate OS/protected-system controls and retain the 3.9 non-claim |
| Add destructive `rm -rf` or direct TL-PX table mutation instructions | Operational profile check fails |

## Exact-commit commands and results

From `implementations/rust` at `bdd8a00b7cc6e97b80f36c34ea8dad7e8cfb879b`:

```text
cargo fmt --all -- --check                              PASS
cargo test --all-targets --offline                     PASS (107 tests)
cargo clippy --all-targets --offline -- -D warnings    PASS
sh -n scripts/restricted-agent-acceptance.sh           PASS
```

From the repository root:

```text
sh -n tests/check-operational-profile.sh               PASS
./tests/check-operational-profile.sh                   PASS
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

- The profile documents duties and exposes aggregate local health; it does not implement a service manager, alert delivery, online backup API, retention system, remote audit destination, safe segment rotation, disaster-recovery automation, or two-person workflow enforcement.
- The 64 MiB sink remains a hard stop, not a production retention strategy. Operators must stop admission before capacity is exhausted.
- `quick_check` and foreign-key/evidence reconciliation are valuable diagnostics, not proof that storage hardware, the host, keys, or all external systems are uncompromised.
- Stale locks, torn rows, historical audit-key compromise, and corruption deliberately require investigation. No automatic repair is claimed.
- The aggregate snapshot does not prove socket ownership, process isolation, separate OS identity, NTP correctness, disk capacity, or remote monitor availability.
- The administrator-backed separate-OS-identity slice 3.9 acceptance script was not run. This slice does not establish forced mediation, Section 3 completion, production readiness, or acceptance.
- Builder testing is not independent review or acceptance.

## Next gate

Build slice 4.5: explicit non-transitive multi-agent handoff. Bind delegation to authenticated delegator/delegatee, exact reduced scope, expiry, one-time use, and a fresh authority decision; forbid bearer forwarding and implicit authority inheritance.
