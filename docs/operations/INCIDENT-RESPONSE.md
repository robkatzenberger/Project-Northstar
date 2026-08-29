# Northstar local incident response

**Profile:** bounded Rust local authority  
**Default posture:** stop new protected actions, preserve immutable evidence, and fail closed

## First ten minutes

1. Disable new protected-capability admission without deleting authority state.
2. Assign an incident commander, evidence custodian, authority operator, protected-system owner, and communications owner. One person may fill multiple roles in a drill, but no one approves their own recovery.
3. Record discovery time, exact commit, process identities, host, configuration/policy/adapter digests, role-key ids, database and sink paths, and the observed error. Never copy key bytes or secret payloads into the ticket.
4. Preserve service logs and make restricted forensic copies of the database, WAL/SHM state when applicable, audit sink, lock, configuration, and relevant protected-system references. Work from copies.
5. Run no destructive cleanup and do not restart an uncertain side effect.
6. Classify the incident below and follow its containment rule.

## Severity

| Severity | Examples | Required response |
| --- | --- | --- |
| Critical | evidence mismatch, SQLite integrity failure, key compromise, identity bypass, second side effect, forced-mediation claim disproven | Stop admission immediately; security lead and system owner; preserve and investigate |
| High | outcome unknown/reconciliation required, trusted-time rollback, audit hard-stop, repeated exporter failure, advisory lock held by an unclear live owner | Stop affected capability; reconcile or recover before resuming |
| Medium | approval backlog, expected revocation activation, capacity warning, failed readiness rehearsal | Bound/monitor; correct before the next test window |
| Low | documentation drift or non-security observability defect | Track with owner and deadline; do not misstate evidence |

## Incident playbooks

### Evidence, chain, or database integrity failure

- Stop all authority/exporter writers and protected-capability admission.
- Preserve the database, WAL/SHM, sink, logs, configuration digests, and role-key ids.
- Do not use SQLite repair, row deletion, acknowledgement edits, seal replacement, or sink truncation on the original.
- Compare a forensic copy with the last verified backup and exported prefix. Determine whether corruption, wrong key history, wrong configuration, or unauthorized mutation occurred.
- Restore only from a verified copy and only after reconciliation passes. Keep the damaged evidence for investigation.

### Stale exporter lock

- Treat a pre-existing lock as a possible live exporter, not litter.
- Stop capability admission and verify every exporter process is stopped using the service manager and process identity records.
- Preserve the lock, sink, database, and logs. This slice does not encode a trustworthy PID/lease in the lock.
- Only after confirming no writer exists may an authorized operator quarantine the lock out of the active directory. Re-run exact-prefix validation before export resumes.
- Never delete the persistent lock file to bypass a held advisory lock; identify and stop the cooperating exporter that owns the lock.

### Torn or incomplete sink row

- Stop writers and preserve the exact bytes. Do not append, truncate, add a newline, or acknowledge the row.
- Determine whether the database row remains pending and whether a complete copy exists elsewhere.
- Recovery requires an explicitly reviewed forensic procedure or restoration from a verified backup. The exporter intentionally refuses to guess.

### Audit backlog or 64 MiB hard stop

- Stop new admissions no later than the critical threshold in the operations profile.
- Drain only through the verified exporter. Do not mark rows exported manually.
- The local profile cannot safely rotate to an empty file because acknowledged history must remain an exact prefix. Resume only after capacity is restored through a reviewed new export design or a verified restoration preserving the full prefix.

### Authorization-MAC or audit-sealing key compromise

- Disable new admission, isolate the affected service identity, preserve key ids and evidence, and involve the key custodian/security lead.
- Rotate to independently generated role-specific material. Retain uncompromised historical verification keys.
- Durable revocation of a historical audit key deliberately makes reconciliation fail closed. Do not delete the revocation or relabel old evidence as trusted.
- Scope every authorization and evidence row created under the affected key and decide whether protected-system containment is required.

### Trusted-time rollback or uncertainty

- Stop new time-bearing transitions. Do not backdate, rewrite the watermark, or extend deadlines.
- Verify host time source, virtualization/suspend events, service restart time, and the last durable watermark.
- Correct infrastructure time under the host runbook. Resume only when time is plausible and nondecreasing. Forward-jump uncertainty bounds are not automated in this slice; suspicious jumps also require review.

### Outcome unknown or reconciliation required

- Keep the authorization consumed and disable automatic retry.
- Use the protected system's idempotency key, transaction/deployment id, target state, independently confirmed settlement, or human investigation.
- Resolve only to the evidence-supported terminal state: confirmed completion, confirmed failure, or honest final unknown.
- Never infer completion from an authorization, claim, process exit alone, or absence of an error.

### Identity, configuration, adapter, or alternate-path bypass

- Revoke/disable the affected principal, capability, policy, adapter, or key where the trusted authority can do so safely.
- Remove alternate access at the OS/protected-system boundary; Northstar cannot contain a path it does not mediate.
- Preserve peer-credential, socket ownership, binary digest, configuration, and protected-system logs.
- Treat any claim of forced mediation as disproven until the separate environment is rebuilt and independently retested.

## Recovery gate

Do not restore protected-capability admission until:

- the incident cause and affected time/state range are bounded;
- original evidence is preserved;
- replacement configuration, keys, binaries, identity maps, and paths are independently reviewed;
- `FileAuditExporter::operational_readiness()` passes on the recovery state (the database-only `operational_snapshot()` is insufficient);
- unresolved external outcomes are explicitly reconciled or remain safely quarantined;
- the relevant exact-commit tests and a focused regression pass;
- the incident commander and a second reviewer sign the recovery record.

## Closure

The post-incident record must separate observed facts, inferences, unknowns, impact, containment, recovery, evidence locations/digests, control failures, and corrective work. Update the runbook and tests when a new failure mode is learned. A closed incident does not convert builder verification into independent acceptance.
