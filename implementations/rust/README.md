# Rust implementation

**Accepted base:** Durable local authority MVP at `aed80e2` — independently exact-commit crosschecked for the bounded evaluate/issue/claim scope

**Working tree:** Canonical schema evidence plus a sealed durable outbox — builder-verified, uncommitted, and not independently accepted
**Crate:** `tlpx` 0.2.0  
**Not yet:** OS authentication, human approval resolution, execution/cancellation/revocation evidence, side effects, or a forced-mediation PEP

This is the start of the authoritative 0.2 core. It does not replace the running JavaScript 0.1 gate and does not yet advertise a conforming 0.2 runtime. It is pinned to the accepted 0.2 schemas and must continue to match `tests/fixtures/tlpx-0.2/jcs/golden.json` exactly.

Builder verification details and non-claims are recorded in [`../../tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md`](../../tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md). Independent exact-commit acceptance is recorded in [`../../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md`](../../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md). The newer working-tree evidence increment is recorded separately in [`../../tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md`](../../tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md).

The MVP proves the local authority seam:

```text
validated Submitted Intent
  -> authenticated-requester binding supplied by a trusted embedding
  -> Switchboard requester/executor scope
  -> exact-match deterministic policy
  -> authority-built Authorized Action
  -> short-lived SQLite authorization
  -> authenticated-executor binding supplied by a trusted embedding
  -> exact Action Binding + capability/scope checks
  -> one atomic durable claim
```

## Layout

```text
rust/
  examples/local_authority.rs   runnable authority-only pilot
  src/authority.rs              SQLite issuance, idempotency, revocation, claim, outbox coupling
  src/evidence.rs               canonical records, local hash/HMAC envelope, reconciliation
  src/policy.rs                 exact-match policy, Switchboard, capabilities
  src/jcs.rs                    Northstar RFC 8785 profile
  src/hash.rs                   domain-separated hashes
  src/types.rs                  distinct 0.2 contract objects
  tests/authority_mvp.rs        negative, race, restart, and replay tests
  tests/evidence_outbox.rs      atomicity, sealing, tamper, export, compatibility tests
  tests/jcs_golden.rs           cross-language canonical/hash oracle
```

## Commands

```bash
cd implementations/rust
cargo test
cargo clippy --all-targets -- -D warnings

# Authority-only smoke run; performs no external side effect.
cargo run --example local_authority -- /tmp/northstar-authority.sqlite request-1

# Emit only the four bounded canonical record types as JSONL.
cargo run --example local_authority -- :memory: schema-check --evidence-jsonl

# From implementations/javascript: validate those Rust records with the JS oracle.
npm run test:rust-evidence
```

Requires a local Rust toolchain (`rustc` / `cargo`). Production dependencies are `sha2`, `hmac`, `getrandom`, and bundled SQLite through `rusqlite`. `serde` and `serde_json` are test-only for loading the golden fixtures.

## Enforced by this MVP

- Policy and identity configuration validate before the authority opens.
- A trusted authenticated requester must equal the Submitted Intent requester.
- Switchboard checks both requester and named executor before a new evaluation.
- Policy is deterministic, exact-match only, and has an explicit default.
- Caller-declared risk cannot lower policy-derived risk.
- The requested capability cannot silently become a different policy capability.
- Authorization IDs, nonces, claim IDs, and normal receipt IDs use OS CSPRNG bytes. If randomness fails while an evaluation error is being recorded, a database-local sequence-derived receipt id preserves durable fail-closed evidence; no authorization is issued.
- The authority computes and stores `intent_hash`, `authorized_action_hash`, and `action_binding_hash`.
- SQLite stores decisions, evaluation errors, and claimable authorizations durably. `(authenticated principal, request_id)` is immutable once a terminal outcome commits.
- Exact error retries return the stored error; changed intent under the same id blocks with `IDEMPOTENCY_CONFLICT`; an eligible successor uses a new id and may link only to a retryable receipt owned by the same authenticated principal.
- One authority-wide transactional sequence orders decisions, evaluation errors, and successful claims.
- Decision, evaluation-error, authorization, and claim JSON is canonical JCS and matches the accepted 0.2 schemas under the JS oracle.
- Decision requester party type is explicit trusted-embedding configuration; use separate authority instances when requester populations have different party types.
- Record JSON and the local storage envelope remain distinct. Record/chain hashes, HMAC seals, key ids, and export state are outbox columns, not private protocol fields.
- State and evidence commit atomically. Pending export reads are ordered, acknowledgements are chain-hash bound and idempotent, and later rows cannot be acknowledged first.
- Reconciliation verifies canonical payloads, the record/hash/HMAC chain, envelope-to-record type/source binding, source references, and coverage for the four supported record types.
- Evaluation, claim, pending export reads, and export acknowledgements reconcile existing evidence first and fail closed on corruption.
- Claim checks executor, current Switchboard action scope, policy hash, capability, resource scope, exact Action Binding, expiry, and revocation.
- `BEGIN IMMEDIATE` plus a compare-and-set update permits at most one successful claim, including across two database connections.
- Replay, mutation, wrong executor, expiry, and revocation block without reopening authority.

## Deliberate boundary

The caller currently supplies authenticated principal strings through the library API. Only a trusted embedding may do that; a network or shell caller is not authenticated merely because it can type a principal name. The policy profile is intentionally small and configuration remains process-owned. Its policy hash is a trusted, format-validated configuration value rather than a digest derived from a standardized serialized bundle. Principal, policy, and capability activation are not yet shared transactional database state, so multi-process configuration freshness is not proven.

The evidence envelope is local implementation behavior, not an accepted portable envelope contract. The embedding must supply and protect a minimum 32-byte HMAC key. Key storage, rotation, recovery, external export transport, and retention are not implemented. SQLite and export-acknowledgement state remain trusted. The fixed key in `local_authority.rs` is intentionally insecure and exists only for the local example and schema test.

Only `tlpx.decision`, `tlpx.evaluation_error`, `tlpx.authorization`, and `tlpx.authorization_claim` are emitted. Revocation and expiration still change authority state without a claimed 0.2 evidence record; execution receipts and cancellation/reconciliation evidence remain deferred until their contracts close.

The SQLite schema is pre-release and intentionally has no migration-compatibility promise. Use a fresh database after trust-path schema changes until a versioned migration policy is introduced.

There is no protected side effect in this crate. A process that can reach a capability directly can still bypass Northstar. Forced mediation remains slice 3.9 and requires separate OS identities and an OS-protected target.

## Requirements

- Follow [`docs/BUILD-SPEC-SHEET.md`](../../docs/BUILD-SPEC-SHEET.md).
- Conform to [`docs/standard/SPEC-v0.2.md`](../../docs/standard/SPEC-v0.2.md) and `schemas/tlpx-0.2/`.
- No `unsafe` in this crate.
- No LLM or dynamic policy code in the decision path.
- Do not implement the abandoned `AUTHORIZED` snapshot token.

## Next implementation

Create a named commit only when requested, then independently inspect and rerun that exact evidence/outbox artifact. Do not broaden that acceptance gate into authenticated transport, execution receipts, cancellation/reconciliation evidence, revocation evidence, or the PEP.
