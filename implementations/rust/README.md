# Rust implementation

**Accepted base:** Durable local authority MVP at `aed80e2` — independently exact-commit crosschecked for the bounded evaluate/issue/claim scope

**Evidence/outbox commit:** Canonical schema evidence plus a sealed durable outbox at `c9bdd0f` — builder-verified, not independently accepted
**Slices 2.4/3.1 local commits:** Policy contract `a87f822`; native Rust activation `1addb5c` — pre-commit builder run passed, full Section 3 verification deferred, unaccepted
**Slice 3.2 local commit:** Shared typed-action/hash fixtures and parity tests at `e835c4e` — bounded exact-commit checks passed, not independently accepted
**Slice 3.3 local commit:** Kernel-derived Unix peer identity, authenticated role facades, policy-bound approval routes, and atomic pending cancellation at `c19b1d2` — full exact-commit builder matrix passed, not independently accepted
**Slice 3.4 local commit:** Human-only route approval/rejection, canonical display binding, fresh post-approval issuance, and atomic approval expiry at `133cd94` — full exact-commit builder matrix passed, not independently accepted
**Slices 3.5–3.7 local commits:** Transactional revocation `ad95653`; terminal execution/reconciliation `9028346`; authenticated adapter contract `518899a` — full exact-commit builder matrices passed, not independently accepted
**Slice 3.8 local commit:** bounded `CooperativeShellRunner` plus `tlpx-run-demo` at `7c41450` — full exact-commit builder matrix passed, cooperative only, not independently accepted
**Slice 3.9 unverified candidate:** bounded restricted-marker `tlpx-run` service and separate-identity acceptance harness at `e6f2bb0` — ordinary pre-commit checks passed, required administrator-backed acceptance run not performed, incomplete and unaccepted
**Slice 4.1 local candidate:** separated role keys, verify-only rotation, authorization proofs, and durable key-revocation enforcement at `980327d`, exact-verified through `efa7f0f` — not independently accepted
**Slice 4.2 local candidate:** bounded append-and-sync-before-ack audit export and exact prefix recovery at `833d8d4` — full exact-commit builder/red-team matrix passed, not independently accepted
**Crate:** `tlpx` 0.2.0  
**Not yet:** verified separate-identity enforcement, full Section 3 verification, active post-claim cancellation, portable revocation/expiry evidence, a general hardened service, or acceptance

This is the start of the authoritative 0.2 core. It does not replace the running JavaScript 0.1 gate and does not yet advertise a conforming 0.2 runtime. It is pinned to the accepted 0.2 schemas and must continue to match both the JCS vectors and the typed-action/hash vectors under `tests/fixtures/tlpx-0.2/` exactly.

Builder verification details and non-claims are recorded in [`../../tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md`](../../tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md). Independent exact-commit acceptance is recorded in [`../../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md`](../../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md). The `c9bdd0f` evidence increment and later slice reports are under [`../../tests/reports/`](../../tests/reports/), including the 3.3 builder matrix in [`../../tests/reports/slice-3.3-local-auth-cancellation-builder-verification-2026-08-18.md`](../../tests/reports/slice-3.3-local-auth-cancellation-builder-verification-2026-08-18.md).

The MVP plus the unaccepted 3.1 candidate proves this local authority seam:

```text
validated Submitted Intent
  -> requester authenticated from Unix peer credentials
  -> Switchboard requester/executor scope
  -> exact-match deterministic policy
  -> authority-built Authorized Action
  -> short-lived SQLite authorization
  -> executor authenticated from Unix peer credentials
  -> exact Action Binding + capability/scope checks
  -> one atomic durable claim
```

## Layout

```text
rust/
  examples/local_authority.rs   runnable authority-only pilot
  src/authority.rs              SQLite issuance, idempotency, revocation, claim, outbox coupling
  src/evidence.rs               canonical records, local hash/HMAC envelope, reconciliation
  src/audit_export.rs           protected append-only local export and crash recovery
  src/local_auth.rs             Unix peer credentials, exact identity map, opaque roles
  src/policy.rs                 exact-match policy, Switchboard, capabilities
  src/policy_manifest.rs        manifest parsing, content binding, supersession, activation
  src/shell_runner.rs           bounded direct-argv cooperative command runner
  src/bin/tlpx-run-demo.rs      narrow same-UID marker-creation demonstration
  src/restricted_pep.rs         bounded protected-marker PEP candidate
  src/bin/tlpx-run.rs           serve/request/verify CLI for the 3.9 candidate
  scripts/restricted-agent-acceptance.sh
                                required separate-identity 3.9 acceptance run
  src/jcs.rs                    Northstar RFC 8785 profile
  src/keys.rs                   purpose-bound key registry, rotation, and proof verification
  src/hash.rs                   domain-separated hashes
  src/types.rs                  distinct 0.2 contract objects
  tests/authority_mvp.rs        negative, race, restart, and replay tests
  tests/evidence_outbox.rs      atomicity, sealing, tamper, export, compatibility tests
  tests/action_types.rs         shared typed-object/canonical/hash fixture parity
  tests/jcs_golden.rs           cross-language canonical/hash oracle
  tests/policy_activation.rs    provenance, activation, durable-error, and claim recheck tests
  tests/cooperative_shell_runner.rs
                                mutation, allowlist, output, timeout, spawn, race, and replay tests
  tests/restricted_pep.rs       same-identity service/protocol checks; not 3.9 acceptance
```

## Commands

```bash
cd implementations/rust
cargo test --all-targets --offline
cargo clippy --all-targets --offline -- -D warnings

# Authority-only smoke run; performs no external side effect.
cargo run --example local_authority -- /tmp/northstar-authority.sqlite request-1

# Emit the bounded canonical record types, including pending cancellation, as JSONL.
cargo run --example local_authority -- :memory: schema-check --evidence-jsonl

# From implementations/javascript: validate those Rust records with the JS oracle.
npm run test:rust-evidence

# Cooperative 3.8 demonstration; creates only a previously absent marker.
cargo run --bin tlpx-run-demo -- /tmp/northstar-runner.sqlite /tmp/northstar-marker request-1

# REQUIRED before claiming slice 3.9: real PEP/agent OS-identity separation.
# On macOS this asks for the administrator password; it uses and cleans a random /tmp tree.
sudo ./scripts/restricted-agent-acceptance.sh
```

Requires a local Rust toolchain (`rustc` / `cargo`). Production dependencies include `sha2`, `hmac`, `getrandom`, safe Unix credential access through `nix`, and bundled SQLite through `rusqlite`. `serde` and `serde_json` are test-only for loading the golden fixtures.

## Enforced by the accepted MVP plus current candidate

- Policy and identity configuration validate before the authority opens.
- Connected Unix peers are authenticated from kernel-supplied UID/GID and resolved through an exact local mapping to opaque principals and roles; unknown peers fail closed.
- Authenticated requester and executor facades reject wrong roles before evaluation or claim mutation.
- Policy manifests bind the exact configured Rust policy content; malformed content, wrong hashes, duplicate identities, cycles, and broken supersession fail startup.
- Evaluation selects one active policy by exact tenant/environment/trusted time. Missing or ambiguous policy commits an evaluation error and issues no authorization.
- A trusted authenticated requester must equal the Submitted Intent requester.
- Switchboard checks both requester and named executor before a new evaluation.
- Policy is deterministic, exact-match only, and has an explicit default.
- Caller-declared risk cannot lower policy-derived risk.
- Submitted Intent, Authorized Action, Executed Action, and Action Binding match the same schema-bound canonical strings, UTF-8 bytes, and domain hashes in JavaScript and Rust.
- The requested capability cannot silently become a different policy capability.
- Authorization IDs, nonces, claim IDs, and normal receipt IDs use OS CSPRNG bytes. If randomness fails while an evaluation error is being recorded, a database-local sequence-derived receipt id preserves durable fail-closed evidence; no authorization is issued.
- The authority computes and stores `intent_hash`, `authorized_action_hash`, and `action_binding_hash`.
- SQLite stores decisions, evaluation errors, and claimable authorizations durably. `(authenticated principal, request_id)` is immutable once a terminal outcome commits.
- Exact error retries return the stored error; changed intent under the same id blocks with `IDEMPOTENCY_CONFLICT`; an eligible successor uses a new id and may link only to a retryable receipt owned by the same authenticated principal.
- One authority-wide transactional sequence orders decisions, evaluation errors, and successful claims.
- Decision, evaluation-error, authorization, and claim JSON is canonical JCS and matches the accepted 0.2 schemas under the JS oracle.
- Decision requester party type is explicit trusted-embedding configuration; use separate authority instances when requester populations have different party types.
- Record JSON and the local storage envelope remain distinct. Record/chain hashes, HMAC seals, key ids, and export state are outbox columns, not private protocol fields.
- State and evidence commit atomically. The bounded local file exporter validates an exact sealed prefix, appends and syncs before internal acknowledgement, recovers complete append-before-ack crashes without duplication, and rejects acknowledgement gaps or time rollback.
- Reconciliation verifies canonical payloads, the record/hash/HMAC chain, envelope-to-record type/source binding, source references, and coverage for the four supported record types.
- Evaluation, claim, pending export reads, and export acknowledgements reconcile existing evidence first and fail closed on corruption.
- Claim checks executor, current Switchboard action scope, the currently active manifest hash, capability, resource scope, exact Action Binding, expiry, and revocation.
- `BEGIN IMMEDIATE` plus a compare-and-set update permits at most one successful claim, including across two database connections.
- Replay, mutation, wrong executor, expiry, and revocation block without reopening authority.
- Pending requester, route-authorized operator, and emergency-authority cancellation is reason-scoped, atomic, and coupled to one schema-valid sealed operator-action row; concurrent cancellation has one winner.
- Human approval/rejection requires a route-authorized human, exact displayed Authorized Action hash, and renderer identity/version. Approval creates a fresh authorization/nonce and starts the claim timer at approval; rejection creates none.
- Approval, rejection, cancellation, and approval expiry are atomic terminal transitions across SQLite connections.
- Adapter sessions authenticate the configured local peer mappings in both directions and verify the activated adapter/version/digest/capability/action/material-field contract before execution start. Same-UID socket-pair tests cover mapping and contract logic, not distinct-peer separation.
- Public authority mutation paths require `AuthenticatedIdentity`; raw principal-string evaluation and claim functions are private internals. Approval expiry and claim recovery likewise require authenticated authority/reconciler roles.
- Only a newly created `ExecutionStart::Started` grants spawn permission. An exact retry or an already-closed attempt returns `NotStarted`, so an `Ok(ExecutionLease)` retry cannot be mistaken for permission.
- The cooperative runner requires a complete issuance-matching Authorized Action, exact direct-argv plan, configured executable plus check-before-spawn digest comparison, activated cwd/environment, bounded output/duration, claim, and fresh durable start before spawn. It clears inherited environment, kills the spawned process group on timeout, and does not write output content into audit evidence.

## Deliberate boundary

The public authority path derives identity from authenticated context; its local profile resolves connected Unix peer UID/GID through a process-owned map. Same-UID unit fixtures intentionally use separate maps to exercise role and contract lookup and do not prove OS identity separation. The UID/GID map, socket ownership/mode, and configuration remain process-owned deployment inputs. Manifest issuer identity is still a trusted configuration assertion, and principal, policy, and capability activation are not shared transactional database state, so authenticated publication and multi-process configuration freshness are not proven. Exact-commit/full Section 3 verification remains deferred.

The evidence envelope is local implementation behavior, not an accepted portable envelope contract. The embedding must supply and protect independent role keys. Slice 4.1 supports purpose binding, verify-only rotation, and durable revocation enforcement. Slice 4.2 adds a protected bounded local file sink, not remote transport, replication, retention automation, monitoring, or same-user tamper resistance. Hardware-backed storage and custody automation remain open; SQLite and the protected local filesystem remain trusted. The fixed role-key bundle in `local_authority.rs` is intentionally insecure and exists only for the local example and schema test.

`tlpx.decision`, `tlpx.evaluation_error`, `tlpx.authorization`, `tlpx.authorization_claim`, pending-cancellation `tlpx.operator_action`, and terminal `tlpx.execution` records are emitted. Revocation and expiration still change authority state without a claimed portable 0.2 evidence record; active post-claim cancellation and portable reconciliation-process evidence remain deferred. Cancellation reason and role are stored transactionally in SQLite because the accepted portable `CANCEL` schema has no stable fields for them.

The SQLite schema is pre-release and intentionally has no migration-compatibility promise. Previous incompatible databases, including the earlier outbox record-type constraint, are rejected before schema mutation. Use a fresh database after trust-path schema changes until a versioned migration policy is introduced.

The slice 3.8 `tlpx-run-demo` can create one marker through `/usr/bin/touch`, but it runs under the caller's UID and authors its own narrow prototype request. A process that can reach a capability directly can still bypass Northstar. The argv allowlist is exact authorization binding, not an operand sandbox; a privileged runner would need capability-specific operand confinement. Executable digest comparison also has a check-to-exec replacement window. Interpreter rejection is defense in depth, not confinement of every executable that can launch another program.

The slice 3.9 candidate adds the reserved `tlpx-run` service for one protected marker capability. The service and acceptance harness are committed, and ordinary tests pass, but the defining separate-identity run has not been performed. Same-identity integration tests do not establish OS enforcement. Until `sudo ./scripts/restricted-agent-acceptance.sh` passes, do not claim forced mediation, alternate-route resistance, 3.9 completion, Section 3 completion, or acceptance. The precise evidence and remaining assertions are recorded in [`../../tests/reports/slice-3.9-restricted-pep-candidate-unverified-2026-08-27.md`](../../tests/reports/slice-3.9-restricted-pep-candidate-unverified-2026-08-27.md).

## Requirements

- Follow [`docs/BUILD-SPEC-SHEET.md`](../../docs/BUILD-SPEC-SHEET.md).
- Conform to [`docs/standard/SPEC-v0.2.md`](../../docs/standard/SPEC-v0.2.md) and `schemas/tlpx-0.2/`.
- No `unsafe` in this crate.
- No LLM or dynamic policy code in the decision path.
- Do not implement the abandoned `AUTHORIZED` snapshot token.

## Next gate

Build slice 4.3 concurrency, trusted-time, cancellation-race, and crash-recovery assurance. The separate-identity slice 3.9 acceptance script remains outstanding; do not broaden Phase 4 checks into that OS-enforcement claim.
