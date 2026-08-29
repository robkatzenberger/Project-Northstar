# Northstar local operations profile

**Status:** slice 4.4 candidate for the bounded Rust local authority  
**Scope:** operational readiness for one SQLite authority and one protected local audit-export file  
**Not:** a production HA, remote-ledger, managed-key, or forced-mediation profile

This profile turns fail-closed behavior into explicit operator duties. It does not convert a local preview into a production service. The separate slice 3.9 administrator-backed enforcement test and Phase 4 independent review remain required.

## Readiness decision

A deployment is ready for bounded testing only when every item below is true:

- The exact build commit, configuration digest, policy manifest digest, adapter digest, and five role-key ids are recorded outside the authority database. Key bytes are never placed in the record.
- The authority database, Unix socket/configuration, audit directory, audit sink, and lock are owned by the intended service identity with the documented restrictive modes.
- The requester cannot write authority configuration, policy, adapter binaries, the database, audit files, or the protected capability through an alternate path.
- Authority startup validation succeeds, and `FileAuditExporter::operational_readiness()` returns successfully before requests are admitted. `Authority::operational_snapshot()` covers SQLite only and is not the deploy/restore readiness boundary.
- SQLite `quick_check`, foreign-key checking, sealed evidence reconciliation, and authority-to-evidence coverage all pass through that snapshot.
- The audit exporter performs an idempotent no-op or a successful bounded export. The persistent lock file remains present, but its nonblocking advisory lock is acquirable; no live competing exporter, torn row, history mismatch, acknowledgement-MAC failure, or sink gap exists.
- A verified offline backup/restore rehearsal has completed on a disposable copy. Live SQLite files were not copied while writes were active.
- Alert ownership, incident commander, security escalation, key custodian, reconciler, and protected-system owner are named.
- The 64 MiB local sink hard stop is acceptable for the test window. There is no safe rotation implementation in this profile.
- Operators accept every non-claim in the applicable slice reports.

If any item is false or unknown, keep protected capability access disabled.

## Aggregate health surface

`Authority::operational_snapshot()` returns only aggregate counts and the sealed evidence reconciliation result. It runs SQLite quick and foreign-key integrity checks plus complete source-bound evidence reconciliation, but it deliberately does not open the audit file. Deployment and restore checks must call `FileAuditExporter::operational_readiness()`, which combines that database snapshot with exact at-rest sink-prefix reconciliation and detects database acknowledgements whose bytes are absent from the sink. A failed call is a failed health check; callers must not turn an error into an empty or healthy snapshot.

| Signal | Normal bounded-test state | Action |
| --- | --- | --- |
| `evidence.pending` | Drains promptly toward zero | Warn on sustained growth; stop admission before capacity is threatened |
| `pending_approvals` | Within declared human queue capacity | Alert on age approaching the approval deadline |
| `unclaimed_authorizations` | Short-lived and bounded | Investigate buildup; never extend/reissue in place |
| `started_executions` | Within executor concurrency limit | Investigate any item beyond its lease |
| `outcome_unknown_executions` | Zero | High-severity reconciliation incident; never auto-retry the side effect |
| `reconciliation_required_executions` | Zero | High-severity protected-system/human investigation |
| `active_revocations` | Matches the change record | Investigate unexpected change; never silently delete a revocation |
| `trusted_time_last_ms` | Nondecreasing and plausible | Stop new transitions on uncertainty or rollback error |
| snapshot/reconciliation error | Never normal | Critical integrity incident; stop admission and preserve evidence |

The snapshot does not inspect disk capacity, socket ownership, process identity separation, NTP state, remote monitoring, or the protected system. Those remain separate deployment checks.

## Audit capacity and backpressure

The local exporter enforces `MAX_AUDIT_SINK_BYTES = 64 MiB` before append. It does not rotate or compact history.

- At 32 MiB: warning; forecast time to the hard stop and schedule an orderly test shutdown.
- At 48 MiB: critical; stop admitting new work and drain/export existing transitions.
- At or near 64 MiB: keep capability admission disabled. Do not delete, truncate, compress in place, or rename the active history and pretend export can continue.

Continuing beyond the hard stop requires a separately designed segmented/remote export increment and review. Increasing or bypassing the bound is not an incident workaround.

## Backup and restore

This profile has no online backup implementation. Use a planned, quiesced procedure:

1. Disable new capability admission and stop the exporter and authority cleanly.
2. Confirm no authority/exporter process remains and preserve the service logs.
3. Copy the database and complete audit sink into a new restricted backup directory using an SQLite-aware backup method or a fully stopped database. Do not copy only the main SQLite file while WAL writes may exist.
4. Record cryptographic digests, byte sizes, ownership/modes, key ids, policy/configuration digests, exact commit, and backup time in a separately protected manifest.
5. Sync the backup media and retain it under the applicable security/retention policy.
6. Restore only into an isolated directory with the original role-key history available for verification.
7. Open the restored authority without capability traffic, run `FileAuditExporter::operational_readiness()`, and compare the backup manifest. Do not substitute the database-only `operational_snapshot()` call.
8. Re-enable traffic only after two-person review records the result.

Never “repair” a backup by removing an inconvenient evidence row, acknowledgement, revocation, unknown outcome, or trusted-time watermark.

## Change and rollback

- Every trust-path change names a commit and runs the full exact-commit matrix before deployment.
- Policy, adapter, identity-map, and key changes are separate reviewed changes with before/after digests.
- Rollback means deploying a previously reviewed binary/configuration that can safely read the current state. It never means restoring authority state to make an already consumed authorization usable.
- The pre-release SQLite schema has no compatibility promise. Test upgrades and rollback on disposable copies; fail closed on incompatibility.
- Key rotation retains the prior verification key. Key revocation is not a reversible rollback tool.

## Incident response

Use [INCIDENT-RESPONSE.md](./INCIDENT-RESPONSE.md). Stop new capability admission, preserve evidence immediately, diagnose on copies, and never replay an uncertain external action.
