# Phase 2.3d / Rust Local Authority MVP — Named-Commit Evidence

**Date:** 2026-08-14

**Implementation commit:** `aed80e2527f05a3730b1057f2d90c55a6c3eb646`

**Commit subject:** `feat: add durable TL-PX 0.2 local authority MVP`

**Evidence status:** builder re-verification against the named commit; independently crosschecked and accepted 2026-08-14

**Remote status at verification:** local only; not pushed

## Scope

This report records implementation evidence for the TL-PX 0.2 implementation-driven 2.3d contract/schema hardening and the Rust local-authority MVP. The tested path is:

```text
Submitted Intent
  -> trusted-embedding requester context
  -> Switchboard
  -> deterministic policy
  -> durable decision or evaluation error
  -> authority-built authorization
  -> exact authenticated executor/action claim
  -> one durable claim winner
```

The JavaScript gate remains the frozen TL-PX 0.1 runtime reference. Its 0.2 code is a schema/JCS/hash oracle, not a 0.2 authority.

## Environment

| Component | Version |
| --- | --- |
| macOS | 26.6.1 (25G76) |
| Rust | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| Cargo | `cargo 1.97.1 (c980f4866 2026-06-30)` |
| Node.js | `v25.5.0` |
| npm | `11.8.0` |

## Executed checks

All commands were run with `HEAD` equal to the full implementation commit above.

| Command | Result |
| --- | --- |
| `cargo test --all-targets` | PASS — 23 authority tests and 8 JCS/type tests |
| `cargo clippy --all-targets -- -D warnings` | PASS — no warnings |
| `cargo run --example local_authority -- :memory: evidence-smoke-aed80e2` | PASS — `ALLOW` → authorization → `CLAIMED`; binding hashes matched |
| `npm test` | PASS — 25 unit, 24 Switchboard, 19 air-gap, 11 policy/operator, 86 policy-compile, 413 JCS assertions; 0.2 schema conformance 43/43 |
| `npm run conformance` | PASS — frozen TL-PX 0.1 conformance 47/47 |
| `npm run tech-test` | PASS — 29/29 |
| `npm run redteam` | PASS/WARN — 16 PASS, 0 FAIL, 4 documented WARN |

## Properties evidenced

- Switchboard unknown, inactive, and action-denied results are durable `DENY` decisions with catalogued codes.
- Target, requested-capability, and adapter constraint mismatches are deterministic policy `DENY`, not policy non-matches or internal errors.
- Duplicate authorization scopes, unknown `ALLOW` capabilities, incomplete authorization templates, and invented policy reason-code aliases fail activation.
- `(authenticated principal, request_id)` is immutable after a terminal decision/error commits.
- Authentication mismatch occupies only the authenticated caller's slot; it cannot poison the proposed principal's request id.
- Exact duplicates return the stored result; changed available intent hashes produce `IDEMPOTENCY_CONFLICT` without replacing it.
- Every retry uses a new request id. A retry link is non-authorizing, same-principal, and valid only for a retryable error.
- Invalid intent and `NEVER` errors remain immutable and cannot become retry parents.
- Decisions, evaluation errors, and successful claims use one transactional authority-wide sequence.
- An injected post-decision SQLite failure becomes a durable `AUTHORITY_INTERNAL_ERROR`; it issues no authorization and replays to the same receipt.
- Resource-scope order survives durable idempotent reload.
- Claim checks current executor scope, policy identity, capability, resource scope, exact Action Binding, expiry, and revocation before consumption.
- Concurrent claim tests permit one winner across threads and separate SQLite connections.
- Concurrent duplicate evaluation across separate connections returns one stored result.
- TL-PX 0.1 remains unchanged at 47/47.

## Deliberate limits

This evidence does **not** establish:

- authenticated local transport or OS-derived requester/executor identity;
- schema-serialized and sealed TL-PX 0.2 runtime records;
- an audit outbox or storage-retention/backpressure implementation;
- human approval lifecycle or operator authentication;
- execution receipts, unknown-outcome reconciliation, or a protected side effect;
- forced mediation or the slice 3.9 restricted-agent PEP claim;
- production readiness, HA, or multi-process configuration freshness.

The Rust API still accepts authenticated-context strings from a trusted embedding. A caller able to reach a protected capability directly can still bypass Northstar.

## Red-team warnings retained

The four JavaScript reference warnings remain expected and unresolved by this Rust authority-only slice:

- omitted actor type defaults in the 0.1 reference;
- declared-metadata honesty is not content inspection;
- a process can bypass an optional library/executor path (A16);
- the 0.1 test-only `allowEphemeral` escape hatch exists.

## Disposition

Implementation and builder verification are complete at `aed80e2`. Pre-commit independent reviews examined the equivalent working-tree implementation and found the original durable-refusal defect closed; their final documentation and pre-release migration cleanups were applied before this commit. The required independent reviewer subsequently inspected and re-ran the exact named commit. The acceptance evidence is recorded in `phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md`.

After that gate, the next implementation focus is schema-valid, sealed 0.2 decision/error/authorization/claim evidence with durable outbox behavior. Authenticated local transport and execution receipts follow before forced mediation.
