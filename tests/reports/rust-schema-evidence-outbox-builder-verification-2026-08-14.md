# Rust Schema Evidence + Sealed Outbox — Builder Verification

**Date:** 2026-08-14

**Base commit:** `aed80e2527f05a3730b1057f2d90c55a6c3eb646`

**Artifact reviewed:** uncommitted working-tree delta on the accepted 2.3d/Rust authority

**Status:** BUILDER-VERIFIED; not independently reviewed or accepted at a named commit

## Bounded scope

This increment serializes the Rust authority's already-durable evaluation-side state as canonical, schema-valid TL-PX 0.2 records:

- `tlpx.decision`;
- `tlpx.evaluation_error`;
- `tlpx.authorization`;
- `tlpx.authorization_claim`.

The record JSON remains the accepted protocol shape. Local record hashes, chain links, HMAC seals, key identifiers, and export acknowledgements live in a separate SQLite outbox envelope rather than as private fields on the TL-PX records.

The decision/error row and any authorization row commit in the same SQLite transaction as their outbox evidence. A successful claim and its evidence row also commit atomically. Injected outbox failures therefore leave no evaluation slot, sequence allocation, authorization, or consumed claim behind.

## Implemented behavior

- canonical JCS serialization through the Rust 0.2 value/canonicalization path;
- SHA-256 record and chain hashes with separate internal domain prefixes;
- HMAC-SHA256 sealing with an embedding-supplied key and key id;
- minimum 32-byte sealing-key validation and redacted `Debug` output;
- explicit embedding configuration for decision-record requester party type, including a human-requester test rather than a machine default hidden in the serializer;
- durable pending-row reads and ordered, hash-bound, idempotent export acknowledgement;
- reconciliation of canonical payloads, record hashes, predecessor links, chain hashes, HMAC seals, envelope-to-record type/source binding, source-row references, and evidence coverage for the four supported record types;
- fail-closed reconciliation before each new evaluation/claim transaction, pending export read, and export acknowledgement;
- restart durability and detection of payload tampering;
- explicit rejection of incompatible pre-release SQLite layouts before schema changes;
- a JS-oracle crosscheck that validates records emitted by the Rust example against the canonical 0.2 schemas and confirms that storage-envelope fields do not leak into record JSON.

## Verification environment

| Component | Version |
| --- | --- |
| Rust | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| Cargo | `cargo 1.97.1 (c980f4866 2026-06-30)` |
| Node.js | `v25.5.0` |
| npm | `11.8.0` |

## Executed checks

| Check | Result |
| --- | --- |
| `cargo test --all-targets --offline` | PASS — 23 authority, 11 evidence/outbox, and 8 JCS/type tests |
| `cargo clippy --all-targets --offline -- -D warnings` | PASS |
| `npm run test:rust-evidence` | PASS — four Rust-emitted record types validate and are canonical |
| `npm test` | PASS — includes TL-PX 0.2 schema conformance 45/45 |
| `npm run conformance` | PASS — frozen TL-PX 0.1 conformance 47/47 |
| `npm run tech-test` | PASS — 29/29 |
| `npm run redteam` | PASS/WARN — 16 PASS, 0 FAIL, 4 documented WARN |

## Negative evidence

The new suite verifies that:

- a weak or absent seal key prevents authority startup;
- requester party type is emitted from explicit trusted-embedding configuration;
- acknowledging a later outbox row before the earliest pending row is rejected;
- an acknowledgement with the wrong chain hash is rejected;
- an outbox insert failure rolls back evaluation state and its authority sequence;
- a claim-evidence insert failure does not consume the authorization;
- payload tampering is detected and blocks subsequent authority work without consuming a new evaluation slot;
- swapping unsealed outbox source identifiers between otherwise valid sealed records is detected;
- idempotent evaluation replay does not duplicate evidence; and
- an older incompatible pre-release database is rejected before new tables are created.

## Explicit limits

This is not a complete 0.2 runtime or production audit service. It does not add authenticated transport, human approval resolution, execution receipts, cancellation/reconciliation evidence, revocation or expiration evidence, side effects, or a forced-mediation PEP. The existing `revoke_at` state transition remains internal and deliberately emits no claimed 0.2 revocation record because that contract is deferred.

The embedding still supplies trusted requester/executor strings and the audit HMAC key. Key storage, rotation, recovery, external export transport, and retention are not implemented. The SQLite database and its export-acknowledgement state remain part of the trusted computing base. The example uses an intentionally fixed non-production key solely for local demonstration and schema crosschecking.

## Disposition

The bounded working-tree implementation is builder-verified. Acceptance requires a deliberate named commit followed by an independent exact-commit inspection and isolated rerun. Until then, documentation must distinguish the accepted `aed80e2` authority base from this newer, unaccepted evidence/outbox delta.
