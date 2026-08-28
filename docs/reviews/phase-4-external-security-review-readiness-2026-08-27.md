# Phase 4 external security review readiness package

**Prepared:** 2026-08-27  
**Review type requested:** independent source, architecture, and adversarial verification  
**Security-code target:** `64d08203c2f30964b176be8872b390ba3c28e5e6`  
**Phase 4 base:** `97e24eefe7786a2c5f88ea4c2d295e6b35e08bd9`  
**Range:** `97e24eefe7786a2c5f88ea4c2d295e6b35e08bd9..64d08203c2f30964b176be8872b390ba3c28e5e6`  
**Disposition before review:** builder-verified local candidates; not independently accepted

This package is an index and challenge brief, not a favorable review. Reviewers should reproduce the checks, inspect the named range rather than trusting builder reports, add adversarial cases, and issue their own findings and disposition.

## 1. Exact review history

The security-code target contains the Phase 4 implementation, tests, profiles, and per-slice builder reports through 4.5. Slice 4.6 adds only this review package, its exact file manifest, and package-integrity checks; its containing commit is recorded separately after exact-commit builder verification. It adds no bearer handoff credential or other authority surface.

| Slice | Commit | Purpose |
| --- | --- | --- |
| Base | `97e24eefe7786a2c5f88ea4c2d295e6b35e08bd9` | Last documentation commit before Phase 4 |
| 4.1 | `980327d76a2b9e157bb52daeef9bf3a05e3e0d34` | Separated role-key lifecycle and authorization proof enforcement |
| 4.1 | `efa7f0f71357e922c7dc65b2a379cbb11d04ff3a` | Isolate restricted-service fixtures for repeatable verification |
| 4.1 report | `fb776786e211523ae0d1697f28475fa4780f12cb` | Builder/red-team evidence and status |
| 4.2 | `833d8d4a01f6a63b8f9ed06c20a531f26bdcc371` | Durable bounded local audit export and reconciliation |
| 4.2 report | `40fb8d8e85aee1e6526b9365c9f3a214fdf7a2ec` | Builder/red-team evidence and status |
| 4.3 | `eb0e624dd9f94672a9dab825f1e01df7c6a0dab8` | Trusted-time, race, deadline, and restart assurance |
| 4.3 report | `74ff502d5e73d20cfb2e857d68b73f4964cf1dc0` | Builder/red-team evidence and status |
| 4.4 | `bdd8a00b7cc6e97b80f36c34ea8dad7e8cfb879b` | Bounded operations, health, capacity, and incident profile |
| 4.4 report | `8ff813b064bd52d3ac7b91c3241fb56da70859d2` | Builder/red-team evidence and status |
| 4.5 | `536111a483b3bece113c7b4e73148a54f74fe9c5` | Authenticated non-transitive two-agent handoff profile |
| Review target | `64d08203c2f30964b176be8872b390ba3c28e5e6` | 4.5 builder report and complete Phase 4 security-code tree |

The exact changed-file allowlist is [`phase-4-review-scope.txt`](./phase-4-review-scope.txt). Run `./tests/check-phase-4-review-package.sh` from the repository root to verify the commit ancestry, target tree, manifest, required reports, reproduction commands, and non-claim language.

## 2. System and trust-boundary map

```text
untrusted request fields
        |
        v
kernel peer identity -> process-owned principal/role map
        |
        v
Switchboard -> exact active policy -> durable SQLite transaction
                                      |       |
                                      |       +-> sealed evidence outbox
                                      |                    |
                                      v                    v
                           signed short authorization   synced local export
                                      |
authenticated exact executor -> atomic claim -> authenticated adapter/PEP
                                                   |
                                                   v
                                      durable execution/reconciliation state

Agent A requester + Agent B executor -> fresh evaluation above
Existing authorization/receipt possession -> never a handoff permission source
```

| Boundary | Trusted input or component | Inputs treated as untrusted | Claimed local behavior | Important limit |
| --- | --- | --- | --- | --- |
| Local identity | Kernel UID/GID plus process-owned exact mapping | Caller-supplied principal strings | Public mutation paths require role-bearing `AuthenticatedIdentity` | Same-UID fixtures do not prove separate OS identity or deployment isolation |
| Admission | Switchboard active/action scope | Unknown, inactive, or out-of-scope principals | Switchboard-first refusal cannot be weakened by policy | Principal registry freshness is a deployment input |
| Policy | Activated exact-scope manifest and content hash | Request risk, action, target, capability, policy selection attempts | Deterministic precedence; invalid/unavailable/ambiguous policy fails closed | No dynamic policy language or remote policy distribution in this profile |
| State | SQLite transaction, foreign keys, full synchronization | Races, retries, stale caller state | Atomic issuance/claim/revocation/terminal transitions and durable time watermark | No replicated database, HA, or physical power-loss claim |
| Role keys | Five independently configured key roles | Wrong-purpose proof, reused/revoked/unknown key | Purpose binding, verify-only rotation, durable signing-key revocation checks | Raw local key bytes remain a trusted configuration input; no KMS/HSM custody |
| Evidence | Canonical record plus local hash chain and HMAC seal | Row mutation, deletion, reordering, source swapping | Complete reconciliation fails closed | Portable revocation/expiry/handoff-link evidence remains deferred |
| Audit export | Protected regular file, sibling lock, append and sync | Symlink, permissive modes, torn/mutated/extra rows, concurrent exporter | Exact database-prefix export; sync precedes acknowledgement | Local 64 MiB hard-stop file only; no remote transport, retention, or safe rotation |
| Time | Monotonic process clock and durable nondecreasing watermark | Negative/backward caller time | New time-bearing transitions reject rollback | Public deterministic seams are trusted embedding/test inputs; no external attestation |
| Adapter/PEP | Authenticated mapped adapter and exact activated contract | Version, digest, route, mapping, action mutation | Start revalidates exact adapter/action binding; claim is executor-bound | Cooperative runner can be bypassed; separate-identity 3.9 gate is not verified |
| Operations | Operator follows bounded runbooks and preserves evidence | Pressure to truncate, delete locks, roll back state, replay unknown effects | Readiness/health fail closed; destructive recovery instructions are rejected | No service manager, alerts, online backup, DR automation, or two-person enforcement |
| Handoff | Both distinct authenticated identities and a fresh intent | Forwarded authorization, substituted party, retry link, mutated action | New policy decision names exactly one executor; onward hop needs a new decision | No network protocol, federation, portable ticket, discovery, or parent/child record |

## 3. Attack-surface inventory

Review at least these areas in the named target:

- `implementations/rust/src/local_auth.rs`: peer credential mapping, role construction, and same-identity assumptions.
- `implementations/rust/src/policy_manifest.rs`, `policy.rs`, and `authority.rs`: selection order, idempotency, time observation, SQL transaction boundaries, issuance, approval, cancellation, revocation, claim, execution, reconciliation, and aggregate health.
- `implementations/rust/src/keys.rs` and proof call sites: key material reuse, purpose substitution, historical verification, rotation, and revocation timing.
- `implementations/rust/src/evidence.rs`: canonical payload reconstruction, source coverage, chain/seal verification, and acknowledgement ordering.
- `implementations/rust/src/audit_export.rs`: path/permission/ownership checks, lock behavior, append/sync/ack ordering, capacity boundary, restart recovery, and platform assumptions around `O_NOFOLLOW`.
- `implementations/rust/src/adapter.rs`, `shell_runner.rs`, and `restricted_pep.rs`: authenticated routes, mapping completeness, check-to-use windows, subprocess confinement, direct capability bypass, socket ownership, and protocol bounds.
- `implementations/rust/src/handoff.rs`: validation order, two-party identity binding, idempotent retries, lack of bearer authority, and attempted onward delegation.
- `docs/operations/` and `tests/check-operational-profile.sh`: fail-closed operator choices, incomplete automation, and misleading production-readiness language.
- All new Phase 4 tests: assertions that could pass without exercising the claimed security boundary, shared fixtures that hide isolation faults, flaky race coverage, and missing failure injection.

## 4. Reproduction matrix

Start from a clean checkout. Record OS, filesystem, SQLite/Rust/Node versions, the exact command output, and any deviations.

```text
git rev-parse HEAD
git diff --name-only 97e24eefe7786a2c5f88ea4c2d295e6b35e08bd9..64d08203c2f30964b176be8872b390ba3c28e5e6
./tests/check-phase-4-review-package.sh
./tests/redteam-phase-4-review-package.sh

cd implementations/rust
cargo fmt --all -- --check
cargo test --all-targets --offline
cargo clippy --all-targets --offline -- -D warnings
sh -n scripts/restricted-agent-acceptance.sh

cd ../javascript
npm test
npm run conformance
npm run tech-test
npm run redteam
npm run test:rust-evidence

cd ../..
sh -n tests/check-operational-profile.sh
./tests/check-operational-profile.sh
```

The ordinary restricted-service Rust integration test needs permission to bind its local Unix socket. That does not substitute for the separate administrator-backed command below, which was not run:

```text
cd implementations/rust
sudo ./scripts/restricted-agent-acceptance.sh
```

Do not request or treat an administrator password as builder evidence. The product owner deferred this gate.

## 5. Builder evidence to distrust and reproduce

| Slice | Builder report |
| --- | --- |
| 4.1 | [`../../tests/reports/slice-4.1-separated-key-roles-builder-verification-2026-08-27.md`](../../tests/reports/slice-4.1-separated-key-roles-builder-verification-2026-08-27.md) |
| 4.2 | [`../../tests/reports/slice-4.2-durable-audit-export-builder-verification-2026-08-27.md`](../../tests/reports/slice-4.2-durable-audit-export-builder-verification-2026-08-27.md) |
| 4.3 | [`../../tests/reports/slice-4.3-race-time-crash-assurance-builder-verification-2026-08-27.md`](../../tests/reports/slice-4.3-race-time-crash-assurance-builder-verification-2026-08-27.md) |
| 4.4 | [`../../tests/reports/slice-4.4-operational-readiness-builder-verification-2026-08-27.md`](../../tests/reports/slice-4.4-operational-readiness-builder-verification-2026-08-27.md) |
| 4.5 | [`../../tests/reports/slice-4.5-multi-agent-handoff-builder-verification-2026-08-27.md`](../../tests/reports/slice-4.5-multi-agent-handoff-builder-verification-2026-08-27.md) |

These reports are builder-authored evidence, not independent findings. A passing rerun is necessary but not sufficient: inspect whether each test actually crosses the intended boundary.

## 6. Known open gates and non-claims

- `sudo ./scripts/restricted-agent-acceptance.sh` was not run. Slice 3.9, forced mediation, alternate-route resistance, and Section 3 completion remain unverified.
- Phase 4 slices are not independently accepted. This package must not be cited as acceptance.
- The evidence/outbox increment at `c9bdd0f` is builder-verified but has not received its deferred independent exact-commit acceptance.
- Active post-claim cancellation and portable revocation/approval-expiry/handoff-link evidence remain deferred.
- There is no production audit transport, safe segment rotation, automated retention, online backup, HA, disaster-recovery automation, managed alerting, or two-person workflow enforcement.
- Role-key separation does not establish KMS/HSM custody, secure provisioning, protected memory, or host-compromise resistance.
- The trusted-time watermark does not establish external clock truth; the assurance suite is not formal model checking or physical power-loss testing.
- The cooperative and bounded restricted runners are not a general hardened service. The separate-identity acceptance gate remains open, and executable check-to-use/confinement questions remain review targets.
- The handoff profile is local fresh evaluation, not portable handoff, federation, discovery, confidentiality, or hierarchical authority.
- Caller-supplied authenticated-context strings in private trusted seams are embedding inputs, not an authentication mechanism.
- The JavaScript implementation remains the TL-PX 0.1 reference plus 0.2 contract oracle, not the planned production authority.

## 7. Required reviewer challenges

The reviewer should answer with evidence, not yes/no intuition:

1. Can any key be accepted for the wrong purpose, remain usable after revocation, or produce history that cannot be verified after rotation?
2. Can an audit row be acknowledged before durable append, duplicated after a crash, skipped, reordered, replaced, truncated, or exported through a path/lock race?
3. Can time rollback, exact-boundary timing, an idempotent replay, or a second SQLite connection reopen or duplicate permission?
4. Can a corruption or health-check failure be reported as healthy, or can an operator follow the runbooks and silently destroy evidence or reopen state?
5. Can A, C, or a mutated action claim an A-to-B authorization? Can receipt possession or a retry link become transitive authority? Does B-to-C truly require a new configured role and decision?
6. Do the ordinary restricted-service tests prove anything beyond same-user local behavior? Identify every remaining direct-capability and filesystem/socket bypass relevant to 3.9.
7. Are schema compatibility and database startup checks sufficient to prevent a pre-release or malformed database from weakening constraints?
8. Do deferred portable evidence gaps make any documentation, reconciliation, or external-audit claim misleading?
9. Can capacity exhaustion, lock contention, huge rows, poisoned mutexes, or expensive reconciliation turn a fail-closed control into an unsafe operational workaround?
10. Which builder tests are tautological, share too much setup, omit a crash point, or need a different harness to be credible?

The reviewer should add cases beyond this list and explicitly distinguish a defect, a missing test, a deployment requirement, a contract gap, and a non-goal.

## 8. Expected independent deliverable

Produce a dated report that contains:

- reviewer identity/model and environment;
- exact security-code target and review-package commit;
- commands actually run and raw pass/fail counts;
- findings with severity, file/function, exploit or failure path, and recommended disposition;
- an explicit decision for each Phase 4 slice: accept bounded scope, accept with conditions, or reject;
- a separate statement for the still-unrun 3.9 gate;
- confirmation that no finding was dismissed merely because a builder test passed;
- remaining risks and the next blocking action.

Do not overwrite builder reports. Add the independent result as a new dated artifact under `tests/reports/` or `docs/reviews/`.
