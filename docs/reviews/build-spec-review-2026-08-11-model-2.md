# Build Specification Review — Independent Model 2

**Date:** 2026-08-11  
**Artifact reviewed:** `docs/BUILD-SPEC-SHEET.md`  
**Review type:** Architecture and specification review; no code execution  
**Status:** Findings accepted and incorporated into the local build baseline

## Verdict

The independent reviewer found the architecture sound: the foundational invariant, fail-closed posture, and phase direction were correct. The review identified specification gaps and ambiguities that must be resolved before TL-PX 0.2 and the hardened PEP acceptance test.

No material disagreement remained after disposition. This record preserves the findings and decisions without presenting model agreement as test evidence.

## Findings and disposition

| # | Finding | Accepted decision |
| --- | --- | --- |
| 1 | Canonicalization was named but undefined. | Use RFC 8785 JCS with a strict Northstar profile, domain-separated/versioned hashes, exact `sha256:` lowercase-hex representation, and cross-language golden fixtures. |
| 2 | Error behavior allowed either an error record or `DENY`. | `DENY` means successfully evaluated and refused. `tlpx.evaluation_error` is a separate terminal evaluation record that creates no authorization and declares retryability. |
| 3 | `PENDING_APPROVAL` had no expiry transition. | Add `APPROVAL_EXPIRED` and authorized `CANCELLED`; late approval requires a new evaluation. |
| 4 | PEP acceptance preceded authenticated executor identity. | Allow an explicitly labeled prototype early, but move authenticated identity before the restricted-agent enforcement acceptance test. |
| 5 | Human approval did not durably bind the action hash displayed. | Operator actions contain `authorized_action_hash`, policy digest, route/quorum, renderer identity/version, and `approval_context_hash`. |
| 6 | Requester-declared risk could be gamed. | Caller risk is advisory; policy derives authoritative risk. Effective risk may be raised by the caller but never lowered. |
| 7 | Breaking contract version was unstated. | Target TL-PX 0.2.0. Freeze v0.1 and its 47 fixtures as historical evidence; create separate 0.2 spec, schemas, and conformance. |
| 8a | Atomic-claim sketch omitted revocation. | Check revocation, principal, policy, signing key, environment, identity, action, expiry, and consumption in one transaction. |
| 8b | Phase ownership between reference and Rust was unclear. | Correct policy and fixtures first in JS/TS; use it as reference, then implement the Rust authority against the stable 0.2 contract. |
| 8c | Crash after real side effect but before receipt was undefined. | Consume authorization and enter `EXECUTION_OUTCOME_UNKNOWN → RECONCILIATION_REQUIRED`; resolve honestly without automatic replay. |
| 8d | Caller-supplied nonce allowed collision gaming. | Caller supplies only a scoped request/idempotency ID. Northstar generates authorization IDs, nonces, and claim IDs. |

## Follow-up refinements

The reviewer accepted the dispositions and requested four additional clarifications, also adopted:

1. Hash strings are exactly `sha256:` plus 64 lowercase hexadecimal characters, never truncated or represented another way.
2. `DENY` is terminal and not automatically retryable; `EVALUATION_ERROR` blocks but may declare bounded retry conditions.
3. Every evaluation error receives a receipt, sequence, and sealed audit row even though it creates no authorization.
4. Cancellation requires an authenticated authorized requester, routed operator, revocation/emergency authority, or Northstar system authority; approval and cancellation race atomically.

## Important design consequences

- Authorization and execution remain distinct evidence events.
- Unknown external outcomes are represented honestly rather than fabricated as success or failure.
- Cross-language conformance is defined at canonical-byte and digest levels, not merely JSON field equivalence.
- A prototype PEP is not security proof until executor identity and direct-capability restriction are real.
- TL-PX 0.1 history is preserved rather than rewritten to match the new design.

## Evidence classification

This is a design-review artifact. It records independent reasoning and architectural disposition, not executed test evidence. Runtime proof belongs under `tests/reports/`.
