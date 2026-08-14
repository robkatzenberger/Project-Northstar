# Rust implementation

**Status:** Durable local authority MVP — evaluate, authorize, and atomically claim
**Crate:** `tlpx` 0.2.0  
**Not yet:** OS authentication, human approval resolution, execution receipts, side effects, or a forced-mediation PEP

This is the start of the authoritative 0.2 core. It does not replace the running JavaScript 0.1 gate and does not yet advertise a conforming 0.2 runtime. It is pinned to the accepted 0.2 schemas and must continue to match `tests/fixtures/tlpx-0.2/jcs/golden.json` exactly.

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
  src/authority.rs              SQLite issuance, idempotency, revocation, claim
  src/policy.rs                 exact-match policy, Switchboard, capabilities
  src/jcs.rs                    Northstar RFC 8785 profile
  src/hash.rs                   domain-separated hashes
  src/types.rs                  distinct 0.2 contract objects
  tests/authority_mvp.rs        negative, race, restart, and replay tests
  tests/jcs_golden.rs           cross-language canonical/hash oracle
```

## Commands

```bash
cd implementations/rust
cargo test
cargo clippy --all-targets -- -D warnings

# Authority-only smoke run; performs no external side effect.
cargo run --example local_authority -- /tmp/northstar-authority.sqlite request-1
```

Requires a local Rust toolchain (`rustc` / `cargo`). Production dependencies are `sha2`, `getrandom`, and bundled SQLite through `rusqlite`. `serde` and `serde_json` are test-only for loading the golden fixtures.

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
- Claim checks executor, current Switchboard action scope, policy hash, capability, resource scope, exact Action Binding, expiry, and revocation.
- `BEGIN IMMEDIATE` plus a compare-and-set update permits at most one successful claim, including across two database connections.
- Replay, mutation, wrong executor, expiry, and revocation block without reopening authority.

## Deliberate boundary

The caller currently supplies authenticated principal strings through the library API. Only a trusted embedding may do that; a network or shell caller is not authenticated merely because it can type a principal name. The policy profile is intentionally small and configuration remains process-owned. Its policy hash is a trusted, format-validated configuration value rather than a digest derived from a standardized serialized bundle. Principal, policy, and capability activation are not yet shared transactional database state, so multi-process configuration freshness is not proven. Rust records and the local sequence are internal database state, not yet schema-serialized or sealed 0.2 decision/error/authorization/claim/execution evidence.

The SQLite schema is pre-release and intentionally has no migration-compatibility promise. Use a fresh database after trust-path schema changes until a versioned migration policy is introduced.

There is no protected side effect in this crate. A process that can reach a capability directly can still bypass Northstar. Forced mediation remains slice 3.9 and requires separate OS identities and an OS-protected target.

## Requirements

- Follow [`docs/BUILD-SPEC-SHEET.md`](../../docs/BUILD-SPEC-SHEET.md).
- Conform to [`docs/standard/SPEC-v0.2.md`](../../docs/standard/SPEC-v0.2.md) and `schemas/tlpx-0.2/`.
- No `unsafe` in this crate.
- No LLM or dynamic policy code in the decision path.
- Do not implement the abandoned `AUTHORIZED` snapshot token.

## Next implementation

Serialize the durable internal outcomes as schema-valid 0.2 decision, evaluation-error, authorization, and claim evidence, then add sealing/audit-outbox behavior. After that, add a real authenticated local transport and execution receipt lifecycle before the OS-enforced PEP acceptance test.
