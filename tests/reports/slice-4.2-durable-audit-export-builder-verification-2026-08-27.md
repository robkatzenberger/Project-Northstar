# Slice 4.2 durable audit export — builder verification

**Date:** 2026-08-27  
**Builder:** Codex  
**Implementation commit:** `833d8d4a01f6a63b8f9ed06c20a531f26bdcc371`  
**Disposition:** full exact-commit builder and focused adversarial matrix passed; independent review and acceptance deferred until Phase 4 is complete

## Scope exercised

- `FileAuditExporter` writes one canonical JCS line per sealed SQLite outbox row in exact outbox order.
- Each export envelope carries the local outbox id, authority sequence and ordinal, complete portable record object, record and chain hashes, predecessor hash, sealing algorithm, sealing key id, and seal.
- The sink and its lock must be regular effective-user-owned files with mode no broader than `0600`; the parent must be an effective-user-owned real directory that is not group/other writable. `O_NOFOLLOW` rejects sink and lock symlinks.
- A sibling exclusive lock rejects concurrent exporters. A pre-existing lock fails closed and requires explicit operator recovery rather than unsafe stale-lock guessing.
- Every new line is appended and `sync_all` completes before its SQLite row may be acknowledged. The directory is synced when the sink is created.
- A crash after a complete synced append but before database acknowledgement is recovered by comparing the exact sink prefix with reconciled sealed rows and acknowledging without a duplicate append.
- A torn final line fails closed for operator recovery. The exporter does not guess whether partial evidence was durable.
- Export acknowledgements are no longer a public authority shortcut. They remain internal to the durable exporter.
- Reconciliation now requires acknowledged rows to form one ordered prefix with nonnegative, nondecreasing export timestamps.
- Reads, lines, batches, and the local sink are bounded. The current local profile accepts batches of 1–1000, lines up to 1 MiB, and a sink up to 64 MiB.

The export envelope is local transport/storage behavior, not a new portable TL-PX record or accepted protocol field set.

## Focused red-team results

| Attack or failure | Result |
| --- | --- |
| Remove an acknowledged sink row | Export fails closed: database acknowledgement has no matching sink row |
| Mutate a sink line | Invalid/canonical mismatch rejected |
| Reorder two valid lines | Exact ordered-prefix comparison rejected |
| Append an attacker-invented extra line | Row absent from authority outbox; rejected |
| Leave an unterminated/torn final line | Rejected; operator recovery required |
| Point sink path at a symlink | `O_NOFOLLOW` open fails closed |
| Broaden sink permissions to `0644` | Rejected |
| Make parent group-writable | Rejected |
| Hold the exporter lock | Second exporter rejected |
| Crash boundary: complete append, missing acknowledgement | Exact row recovered without duplication |
| Mark a later database row exported across a pending gap | Reconciliation rejected |
| Store negative or decreasing export time | Reconciliation rejected |
| Attempt a new acknowledgement with time before the prior acknowledgement | Append remains recoverable; acknowledgement rejected until monotonic time is supplied |
| Restart after a bounded partial export | Remaining rows export in order; repeated export is idempotent |

## Exact-commit commands and results

From `implementations/rust` at `833d8d4a01f6a63b8f9ed06c20a531f26bdcc371`:

```text
cargo fmt --all -- --check                              PASS
cargo test --all-targets --offline                     PASS (100 tests)
cargo clippy --all-targets --offline -- -D warnings    PASS
sh -n scripts/restricted-agent-acceptance.sh           PASS
```

The ordinary restricted-service integration test required running outside the desktop filesystem sandbox so its Unix socket could bind; it then passed without administrator privileges. This is not the separate `sudo` slice 3.9 acceptance run.

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

- This is a bounded local append-only file sink, not remote transport, replicated storage, an external ledger, high availability, retention automation, or a monitoring service.
- SQLite, the protected export directory, effective-user isolation, local filesystem durability, and same-user process behavior remain trusted deployment boundaries.
- The fixed 64 MiB sink bound requires rotation/backpressure procedures before production-shaped use. Those procedures and alert thresholds belong to slice 4.4.
- A stale lock and a torn final row deliberately require operator investigation. Automatic deletion or truncation could hide a live exporter or destroy the only complete copy.
- Same-effective-user malicious mutation cannot be eliminated by file mode alone; deployment isolation and an independent destination remain required.
- The exported local envelope has no accepted cross-implementation schema and does not broaden TL-PX 0.2 record conformance.
- The administrator-backed separate-OS-identity slice 3.9 acceptance script was not run. This slice does not establish forced mediation, Section 3 completion, or acceptance.
- Builder testing is not independent review or acceptance.

## Next gate

Build slice 4.3: concurrency, trusted-time, cancellation-race, and crash-recovery assurance. Reuse the durable exporter in failure sequences and preserve the explicit 3.9 non-claim.
