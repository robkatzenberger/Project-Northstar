# Slice 3.4 approval resolution and expiry — builder verification

**Date:** 2026-08-18
**Builder:** Codex
**Initial artifact under test:** uncommitted working tree based on `7bfcc4f`
**Named artifact:** local commit `133cd94`
**Disposition:** full exact-commit builder matrix passed; not independently reviewed or accepted

## Scope exercised

- `REQUIRE_APPROVAL` policy effects now bind both the exact authorization template and ordered approval route into the versioned policy-content hash.
- Pending approvals durably retain canonical Authorized Action JSON, its full hash, Action Binding hash, authenticated requester/executor, resource scope, active policy hash, and approval deadline.
- Only a kernel-authenticated human operator on the policy-bound route can retrieve the approval view or resolve it.
- `APPROVE` and `REJECT` require the Authorized Action hash the operator saw plus non-empty renderer identity/version.
- `APPROVE` creates a new `AUTHORIZED_UNCLAIMED` authorization with an authority-generated ID and nonce. Its short claim window begins at approval time.
- `REJECT` is terminal and creates no authorization.
- Expiry, approve, reject, and cancel use atomic compare-and-set transitions. Late outcomes fail closed.
- `APPROVE`/`REJECT` commit schema-valid sealed `tlpx.operator_action` evidence; approval commits the new schema-valid authorization in the same transaction and authority sequence.

## Environment

- Node.js `v25.5.0`
- npm `11.8.0`
- rustc `1.97.1 (8bab26f4f 2026-07-14)`
- cargo `1.97.1 (c980f4866 2026-06-30)`

## Commands and results

From `implementations/rust`:

```text
cargo fmt --all -- --check                              PASS
cargo test --all-targets --offline                     PASS (63 tests)
cargo clippy --all-targets --offline -- -D warnings    PASS
```

The 33 authority tests include human-only route authorization, display-hash mismatch, fresh post-approval nonce/deadline, reject-without-authorization, explicit and implicit expiry, approve/reject race, and cancellation/expiry race.

From `implementations/javascript`:

```text
npm test                         PASS (919 checks across the combined suite)
npm run conformance             PASS (47/47 TL-PX 0.1)
npm run tech-test               PASS (29/29)
npm run redteam                 PASS (16 PASS, 0 FAIL, 4 documented WARN)
npm run test:rust-evidence      PASS (9/9 records)
```

The combined suite includes 68/68 TL-PX 0.2 contract checks. The Rust evidence crosscheck now validates a real Rust `APPROVE` record and its atomically issued authorization in addition to cancellation and the prior bounded records.

## Fail-closed evidence

- A machine mapped to the operator role is not a human authorizer and cannot view or decide an approval.
- A human operator outside the exact route cannot view, approve, or reject it.
- A validly formatted but wrong displayed action hash leaves the request pending.
- Missing renderer identity/version is rejected.
- Policy inactivity or executor deactivation blocks approval issuance.
- The approval deadline is independent of the later claim deadline; no claim timer elapses while a human is deciding.
- At most one of approve/reject wins across separate SQLite connections.
- At most one of authenticated cancellation/authority expiry wins across separate SQLite connections.
- Approval at or after the deadline atomically records `APPROVAL_EXPIRED`; late approval and cancellation remain terminal.

## Explicit limits and non-claims

- This is authority logic and evidence, not a human-facing approval UI. Renderer identity/version are trusted inputs from the authenticated local approval adapter; renderer binary integrity and authenticated adapter identity are slice 3.7 work.
- No MFA, rationale capture, anti-fatigue control, or M-of-N quorum is implemented.
- Production monotonic elapsed-time enforcement is not implemented. Normal APIs use the local wall clock; deterministic `_at` methods accept trusted test/embedding time.
- The accepted operator-action schema has no `EXPIRE` outcome. Approval expiry is durable transactional authority state with an authority sequence, but it cannot yet emit a portable schema-valid terminal record. This evidence gap is explicit; no private record type was invented.
- The local UID/GID map, Unix socket ownership/mode, policy catalog, SQLite file, sealing key, and raw trusted-embedding methods remain deployment trust boundaries.
- The pre-release SQLite schema rejects earlier incompatible databases before mutation. Use a fresh database after this trust-path schema change.
- There is no execution receipt, protected side effect, post-claim cancellation, revocation evidence, hardened service, or forced-mediation PEP.
- Passing this matrix does not independently accept slice 3.4 or establish a conforming TL-PX 0.2 runtime.

## Exact-commit rerun

After local commit `133cd94`, the complete Rust and JavaScript command matrix above was rerun without working-tree changes. Results matched the pre-commit run: 63 Rust tests, clean formatting and clippy, 919 combined JavaScript checks, 47/47 TL-PX 0.1 conformance, 68/68 TL-PX 0.2 contract checks, 29/29 technical checks, 16 PASS / 0 FAIL / 4 documented WARN in the historical JavaScript red team, and 9/9 Rust evidence rows.

## Next gate

Continue to slice 3.5 for transactional revocation/state closure around the existing one-time claim. Full Section 3 verification and independent review remain deferred until slices 3.1–3.9 are complete.
