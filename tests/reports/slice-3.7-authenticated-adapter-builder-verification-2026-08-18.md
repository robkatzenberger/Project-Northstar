# Slice 3.7 authenticated adapter contract — builder verification

**Date:** 2026-08-18
**Builder:** Codex
**Initial artifact under test:** uncommitted working tree based on `d9087ea`
**Named artifact:** local commit `518899a`
**Disposition:** full exact-commit builder matrix passed; not independently reviewed or accepted

## Scope exercised

- A startup-validated adapter registry fixes adapter id/version, authenticated adapter and authority principals, canonical binary digest, capability/action coverage, and the exact ordered nine-field Action Binding projection.
- The local Unix profile authenticates the channel in both directions from kernel peer credentials. The authority side requires an adapter role; the adapter-side trust mapping independently requires an authority role.
- Every execution start presents the exact Executed Action again. The authority re-hashes it, compares it with the consumed claim, and verifies adapter session identity, version, digest, capability, and action before durable start.
- Exact retries bind adapter principal, binary digest, executor, action, and idempotency key. A changed retry cannot attach itself to the durable attempt.
- Adapter-started terminal receipts bind `adapter_principal` and `adapter_binary_hash`. A claim recovered as `LEASE_EXPIRED` before adapter start carries explicit null provenance instead of inventing an adapter observation.
- Startup and runtime negatives cover missing/reordered/extended material mappings, incomplete action coverage, wrong role, adapter substitution, authority substitution, digest mismatch, version mismatch, and Executed Action mutation.

## Commands and results

From `implementations/rust`:

```text
cargo fmt --all -- --check                              PASS
cargo test --all-targets --offline                     PASS (77 tests)
cargo clippy --all-targets --offline -- -D warnings    PASS
```

From `implementations/javascript`:

```text
npm test                         PASS (933 checks across the combined suite)
npm run conformance             PASS (47/47 TL-PX 0.1)
npm run tech-test               PASS (29/29)
npm run redteam                 PASS (16 PASS, 0 FAIL, 4 documented WARN)
npm run test:rust-evidence      PASS (10/10 records)
```

The combined suite includes 82/82 TL-PX 0.2 contract checks. The real Rust execution receipt passes the JavaScript 0.2 schema oracle.

## Fail-closed evidence

- Missing, reordered, or extra material mapping fields reject the adapter contract.
- An authorizing policy action without complete adapter coverage rejects authority startup.
- A peer lacking the adapter role cannot establish a session.
- A session naming the wrong adapter or wrong authority principal is rejected.
- A malformed or different binary digest is rejected before execution start.
- A claimed adapter id/version without an exact activated contract is rejected.
- Any different Executed Action binding is rejected as `ACTION_MISMATCH` before durable start.
- Valid terminal evidence records the authenticated adapter principal and verified configured digest.

## Explicit limits and non-claims

- This is a bounded local Unix contract. It is not network mTLS, workload attestation, a production service identity system, or independent binary measurement.
- The presented digest is compared with activated configuration inside an authenticated session. Process isolation, executable measurement, socket ownership/mode, and configuration integrity remain deployment trust boundaries.
- No portable adapter-contract record is standardized; the registry is trusted local authority configuration.
- The authority still performs no protected side effect. Callers can bypass this library until slices 3.8/3.9 supply a PEP and OS-enforced acceptance test.
- Active post-claim cancellation, portable revocation/expiry evidence, and signing-key enforcement remain open.
- The pre-release SQLite schema requires a fresh database after the receipt provenance field addition.
- Passing this matrix does not establish TL-PX 0.2 runtime conformance, independent review, acceptance, or forced mediation.

## Exact-commit rerun

After local commit `518899a`, the complete Rust and JavaScript command matrix above was rerun without source working-tree changes. Results matched the pre-commit run: 77 Rust tests, clean formatting and clippy, 933 combined JavaScript checks, 47/47 TL-PX 0.1 conformance, 82/82 TL-PX 0.2 contract checks, 29/29 technical checks, 16 PASS / 0 FAIL / 4 documented WARN in the historical JavaScript red team, and 10/10 Rust evidence rows.

## Next gate

Continue to slice 3.8 for the `tlpx-run` shell PEP prototype. Slice 3.9 remains the OS-enforced forced-mediation acceptance test. Full Section 3 verification and independent review remain deferred until slices 3.1–3.9 are complete.
