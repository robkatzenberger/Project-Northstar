# Slice 3.3 local authentication and cancellation — builder verification

**Date:** 2026-08-18  
**Builder:** Codex  
**Initial artifact under test:** uncommitted working tree based on `0a4ddc53f8faf93ea1767c4f530c1e0d65de37e6`
**Named artifact:** local commit `c19b1d2`
**Disposition:** full exact-commit builder matrix passed; not independently reviewed or accepted

## Scope exercised

This slice adds the first authenticated local identity boundary to the Rust authority:

- Unix-domain-socket peer credentials are read from the kernel through safe `nix` APIs on macOS and Linux/Android.
- Exact UID/GID mappings produce opaque requester, operator, executor, or emergency-canceller identities.
- Authenticated requester and executor facades prevent a caller from supplying its own principal string at that boundary.
- `REQUIRE_APPROVAL` policy rules bind a non-empty approval route into the versioned exact-match policy-content hash.
- An authenticated original requester, an operator on the exact policy route, or an emergency authority can atomically cancel a pending approval for its permitted reason.
- Cancellation emits one schema-valid `tlpx.operator_action` row in the same transaction, with one authority-wide sequence number and a sealed outbox envelope.
- Concurrent cancellation through separate SQLite connections has exactly one terminal winner.

## Environment

- Node.js `v25.5.0`
- npm `11.8.0`
- rustc `1.97.1 (8bab26f4f 2026-07-14)`
- cargo `1.97.1 (c980f4866 2026-06-30)`
- `nix` `0.30.1`

## Commands and results

From `implementations/rust`:

```text
cargo fmt --all -- --check                              PASS
cargo test --all-targets --offline                     PASS (58 tests)
cargo clippy --all-targets --offline -- -D warnings    PASS
```

The 58 Rust tests include 28 authority tests, including kernel peer authentication, role separation, policy-route binding, cancellation authorization, and a two-connection cancellation race. The remaining tests cover typed actions, evidence/outbox behavior, JCS/hash parity, and policy activation.

From `implementations/javascript`:

```text
npm test                         PASS (915 checks across the combined suite)
npm run conformance             PASS (47/47 TL-PX 0.1)
npm run tech-test               PASS (29/29)
npm run redteam                 PASS (16 PASS, 0 FAIL, 4 documented WARN)
npm run test:rust-evidence      PASS (6/6 records)
```

The combined `npm test` includes 64/64 TL-PX 0.2 contract checks. The Rust evidence crosscheck validates two decisions, one authorization, one authorization claim, one evaluation error, and the new cancellation `tlpx.operator_action`.

## Fail-closed and race evidence

- Unknown local peers are rejected rather than assigned a default identity.
- Requester-only and executor-only APIs reject the wrong authenticated role before authority mutation; a wrong-role claim does not consume the authorization.
- Requester cancellation is limited to the original requester and requester-withdrawal reason.
- Operator cancellation requires membership in the policy-bound approval route and the operator-cancellation reason.
- Emergency cancellation requires the emergency-canceller role and emergency-revocation reason.
- Wrong route, wrong owner, wrong role, and wrong reason return `CANCELLATION_UNAUTHORIZED`.
- A duplicate or late transition returns `APPROVAL_TERMINAL`; it does not add a second operator record.
- Two concurrent cancellations produce one `CANCELLED` transition and one operator-action evidence row.

## Explicit limits and non-claims

- This is a bounded Unix local profile, not network authentication, SSO, mTLS, or a general identity protocol.
- The UID/GID map, Unix socket ownership/mode, process configuration, policy catalog, SQLite file, and sealing key remain trusted deployment inputs.
- Developer tests use the current test process UID with separate authenticator configurations. They exercise kernel-derived credentials and role mappings but do not demonstrate distinct operating-system users.
- There is no long-running authority service, listener hardening, or dedicated service identity yet. Those deployment boundaries culminate in slice 3.9.
- The original public Rust methods that accept principal strings remain a trusted-embedding/library seam for compatibility. Untrusted adapters and the eventual PEP must use the authenticated facade.
- This slice cancels only `PENDING_APPROVAL`. It does not implement `APPROVE`, `REJECT`, approval expiry, or post-claim cancellation.
- The accepted portable `CANCEL` operator-action schema does not carry stable cancellation reason or role fields. The implementation stores reason and role transactionally in SQLite and does not invent private protocol fields.
- Execution-receipt, post-claim cancellation/reconciliation, and revocation-evidence contracts remain deferred. No protected side effect is performed.
- The pre-release SQLite schema has no migration promise. An older outbox schema is rejected before mutation; use a fresh database after trust-path schema changes.
- Passing this builder matrix does not make slice 3.3 independently accepted, make the crate a conforming TL-PX 0.2 runtime, or establish forced mediation.

## Exact-commit rerun

After local commit `c19b1d2`, the complete Rust and JavaScript command matrix above was rerun without working-tree changes. Results matched the pre-commit run: 58 Rust tests, clean formatting and clippy, 915 combined JavaScript checks, 47/47 TL-PX 0.1 conformance, 64/64 TL-PX 0.2 contract checks, 29/29 technical checks, 16 PASS / 0 FAIL / 4 documented WARN in the historical JavaScript red team, and 6/6 Rust evidence rows.

## Next gate

Continue to slice 3.4 for approval resolution and expiry. Full Section 3 verification and independent review remain deferred until slices 3.1–3.9 are complete.
