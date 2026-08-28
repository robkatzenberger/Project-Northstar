# Slice 4.5 multi-agent handoff — builder verification

**Date:** 2026-08-27  
**Builder:** Codex  
**Implementation/profile commit:** `536111a483b3bece113c7b4e73148a54f74fe9c5`  
**Disposition:** full exact-commit builder and focused adversarial matrix passed; independent review and acceptance deferred until Phase 4 is complete

## Scope exercised

- `Authority::evaluate_handoff_authenticated[_at]` requires two distinct authenticated principals: Agent A with the requester role and Agent B with the executor role.
- The submitted intent must name the authenticated A as `requesting_principal` and authenticated B as `executing_principal` before the normal authority evaluation begins.
- The handoff entry point creates no transferable credential. Switchboard, policy, exact Action Binding, short claim deadline, authenticated claim, and atomic one-time consumption remain the source of authority.
- Handoff transport retries reuse the same exact `request_id`; `retry_of_receipt_id` is rejected by this profile so it cannot be presented as a parent-permission link.
- An onward B-to-C hop requires B to possess an independently configured requester role and creates a new receipt, intent hash, and authorization naming C.
- The profile and tests explicitly retain the abandoned August 7 snapshot-token prohibition.

## Focused red-team results

| Attack or confused-deputy attempt | Result |
| --- | --- |
| Substitute a different authenticated executor before evaluation | `HANDOFF_EXECUTOR_MISMATCH`; no evaluation/evidence row created |
| Substitute the intent requester | `HANDOFF_REQUESTER_MISMATCH`; no evaluation/evidence row created |
| Use one principal as both sides of the multi-agent profile | `HANDOFF_REQUIRES_DISTINCT_PRINCIPALS` |
| Present a receiver without the executor role | `AUTHENTICATION_FAILED` |
| Use `retry_of_receipt_id` as implied parent authority | `HANDOFF_RETRY_LINK_FORBIDDEN` |
| Have A claim the A-to-B authorization | `EXECUTOR_MISMATCH` |
| Forward the A-to-B authorization to C | `EXECUTOR_MISMATCH`; parent authorization remains unclaimed |
| Mutate exact arguments before B claims | `ACTION_MISMATCH`; authorization remains unclaimed |
| Replay B's successful claim | `ALREADY_CLAIMED` |
| Use the handoff entry point with a policy-denied action | Ordinary `DENY`; no authorization issued |
| Retry exact request ID and intent | Same receipt and authorization; no duplicate permission |
| Mutate an idempotent request | `IDEMPOTENCY_CONFLICT` |
| Attempt B-to-C onward use | Parent cannot be used; a separate authenticated evaluation and new C-bound authorization are required |

The four focused integration tests were also repeated 20 times (80 test cases) with no failure.

## Exact-commit commands and results

From `implementations/rust` at `536111a483b3bece113c7b4e73148a54f74fe9c5`:

```text
cargo fmt --all -- --check                              PASS
cargo test --all-targets --offline                     PASS (114 tests)
cargo clippy --all-targets --offline -- -D warnings    PASS
```

The ordinary restricted-service integration test ran outside the desktop filesystem sandbox so its Unix socket could bind. This was not the separate administrator-backed slice 3.9 acceptance run.

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

- The profile authenticates both participants through the existing local identity abstraction. Same-UID test fixtures use separate process-owned maps and do not prove separate operating-system identities.
- The profile is a fresh-evaluation API, not a network protocol. It provides no discovery, transport, confidentiality, cross-authority federation, public-key proof of possession, or portable claim ticket.
- No portable handoff record or parent/child receipt-link field is introduced. Ordinary per-hop TL-PX evidence identifies requester, executor, and exact action; portable workflow linkage remains part of deferred evidence-contract closure.
- An independently configured B requester may initiate another evaluation. That authority comes from B's authenticated role and the new policy decision, never from the A-to-B authorization.
- The administrator-backed separate-OS-identity slice 3.9 acceptance script was not run. This slice does not establish forced mediation, Section 3 completion, production readiness, or acceptance.
- Builder testing is not independent review or acceptance.

## Next gate

Build slice 4.6: assemble an external security review readiness package that pins the exact Phase 4 history, trust boundaries, attack surface, required commands, known unverified gates, non-claims, and reviewer questions without pre-answering the independent review.
