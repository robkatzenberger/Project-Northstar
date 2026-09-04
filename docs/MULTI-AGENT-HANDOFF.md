# Bounded multi-agent handoff profile

**Status:** slice 4.5 local preflight candidate at `536111a`; independent Phase 4 review returned changes requested; exact remediation `ee720d4` narrows the claim to optional co-presentation preflight and awaits independent re-review

This optional profile defines a co-present handoff preflight as a fresh TL-PX evaluation for one exact action. It does not transfer an existing authorization and it does not create a bearer handoff token. It is not universal multi-agent mediation: the ordinary authority evaluation API can still name a different executor without proving that executor is present at evaluation time. Deployments must not infer B's participation merely because an authorization names B; B is authenticated later at claim.

## Required flow

1. Agent A authenticates with the requester role.
2. Agent B authenticates with the executor role.
3. A fresh `SubmittedIntent` names A as `requesting_principal` and B as `executing_principal` and binds the exact action, target, normalized arguments, environment, tenant, payload/artifact digests, adapter, capability, and resource scope.
4. Switchboard and policy evaluate the intent normally. A handoff entry point cannot turn a denial or evaluation error into permission.
5. Any resulting authorization names B and retains the normal short claim deadline, exact Action Binding, authenticated claim, and atomic single-use rules.
6. Only B can claim that authorization. A, another agent, a mutated action, an expired authorization, or a replay fails closed.

Transport retries reuse the same `request_id` and exact intent, so normal authority idempotency returns the same decision. A handoff must not use `retry_of_receipt_id` to imply that a parent receipt grants the new action.

## Non-transitivity

An A-to-B authorization gives B no inherited right to delegate. If B is independently configured with the requester role and asks C to act, B-to-C is a new authenticated evaluation with a new request ID, receipt, intent hash, and authorization naming C. C cannot claim the A-to-B authorization.

The initial profile deliberately creates no portable handoff credential. The abandoned August 7 `tlpx.authz_token` is not used. Offline verification, receipt possession, and a parent authorization cannot establish live permission or global non-consumption.

## Evidence boundary

The ordinary decision, authorization, claim, and execution evidence identifies the exact requester, executor, and action for each hop. A portable parent/child handoff-link record is not added in this slice because the execution-side evidence contract remains deferred. Operators may correlate approved application-level workflow identifiers outside security-critical TL-PX records, but that metadata is not authority.

## Bounded local-profile limits

- The `evaluate_handoff_authenticated*` wrapper proves both local identity objects were presented to that call only. Code with direct access to the ordinary `Authority` API can bypass this optional preflight, so it is an API profile, not a security boundary.
- Test identities use the existing authenticated local identity abstraction. Same-UID fixtures do not prove separate operating-system identity or forced mediation.
- This profile does not provide agent discovery, cross-authority federation, network transport, confidentiality, public-key proof of possession, portable claim tickets, or hierarchical authorization.
- It does not complete the separate slice 3.9 administrator-backed acceptance gate.
- Builder testing is not independent review or acceptance; the independent Phase 4 reviewer rejected the stronger handoff-security claim.
