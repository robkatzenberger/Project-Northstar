# Phase 2.3d / Rust Local Authority MVP — Independent Exact-Commit Crosscheck

**Date:** 2026-08-14

**Reviewed commit:** `aed80e2527f05a3730b1057f2d90c55a6c3eb646`

**Commit subject:** `feat: add durable TL-PX 0.2 local authority MVP`

**Review status:** ACCEPTED for the bounded 2.3d/Rust local-authority scope

**Method:** independent static inspection plus isolated exact-commit rerun; the reviewer did not rely on the builder report as proof

## Scope and bar

This crosscheck reviewed the named commit as an isolated artifact. It covers the durable local path:

```text
Submitted Intent
  -> trusted-embedding requester context
  -> Switchboard
  -> deterministic exact-match policy
  -> durable decision or evaluation error
  -> authority-built authorization
  -> exact authenticated executor/action claim
  -> one durable claim winner
```

Acceptance here does not claim authenticated transport, operator authentication, schema-serialized or sealed runtime evidence, execution receipts, forced mediation, a PEP, or production readiness.

## Independent inspection

The review inspected the parent-to-commit diff and exact `aed80e2` contents, including the Rust authority, policy, types, persistence model, schema/validator changes, conformance additions, and adversarial tests.

The following properties were checked directly:

- authenticated idempotency is scoped by `(authenticated principal, request_id)` and stored slots are immutable;
- authentication mismatch cannot occupy the proposed principal's slot;
- invalid intents without a canonical hash replay the stored error rather than being re-evaluated;
- successor retry links are non-authorizing, same-principal, and limited to retryable evaluation errors;
- Switchboard refusal precedes policy and produces durable `DENY` decisions;
- policy activation rejects ambiguous duplicate action rules, incomplete `ALLOW` templates, and unknown capabilities;
- authority-generated receipt, authorization, nonce, and claim identifiers are allocated before use;
- one SQLite transaction owns each decision/error/issuance result, and failed issuance is converted to a durable evaluation error without leaving authorization;
- decision, evaluation-error, and successful-claim rows share one transactional authority sequence;
- claim rechecks executor activity/scope, policy identity, capability, resource scope, exact Action Binding, expiration, and revocation;
- authorization consumption and claim-row insertion are atomic, including across separate SQLite connections;
- the crate performs no protected side effect and does not present caller-supplied identity strings as authentication.

No correctness or security defect was found in the bounded Rust evaluate/issue/claim path.

## Environment

| Component | Version |
| --- | --- |
| macOS | 26.6.1 (25G76) |
| Rust | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| Cargo | `cargo 1.97.1 (c980f4866 2026-06-30)` |
| Node.js | `v25.5.0` |
| npm | `11.8.0` |

## Isolated exact-commit rerun

The commit was exported with `git archive` into temporary directories so working-tree documentation changes and untracked reports could not affect the result.

| Check | Result |
| --- | --- |
| `cargo test --all-targets` | PASS — 23 authority tests and 8 JCS/type tests |
| `cargo clippy --all-targets -- -D warnings` | PASS — no warnings |
| `cargo run --example local_authority -- :memory: independent-crosscheck-aed80e2` | PASS — `ALLOW` → authorization → `CLAIMED`; binding hashes matched |
| `npm test` | PASS — all JS suites; 0.2 schema conformance 43/43 |
| `npm run conformance` | PASS — frozen TL-PX 0.1 conformance 47/47 |
| `npm run tech-test` | PASS — 29/29 |
| `npm run redteam` | PASS/WARN — 16 PASS, 0 FAIL, 4 documented WARN |

## Non-blocking schema finding

The exact commit's 0.2 operator-action schema accepts a machine `operator` for `APPROVE` or `REJECT`, while the normative party model requires a human authorizer. This is a 2.3 schema-completeness issue, not a defect in the Rust evaluate/claim path and not a 2.3d acceptance blocker. The follow-up working-tree schema/docs delta adds the missing conditional type constraint and a negative conformance case.

Execution-receipt, cancellation/reconciliation, and revocation-evidence schema closure also remain outside this acceptance. They must not be inferred from the existing 2.3 conformance suite or from this Rust authority-only review.

## Disposition

Commit `aed80e2527f05a3730b1057f2d90c55a6c3eb646` is independently accepted for 2.3d and the stated Rust local-authority MVP scope. This closes the named-commit crosscheck gate.

The next bounded implementation increment may serialize the authority's durable decisions, evaluation errors, authorizations, and successful claims as schema-valid TL-PX 0.2 evidence, followed by sealing and durable outbox/reconciliation behavior. It must not expand into authenticated transport, execution receipts, revocation evidence, or a protected-execution PEP.
