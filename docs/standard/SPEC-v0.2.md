# Trust Layer Pre-Execution Minimum Standard (TL-PX)

**Version:** 0.2.0  
**Status:** Draft contract — slices 2.1–2.2 (decision / error / state / compatibility / JCS hashes)  
**Profile:** Minimum  
**Date:** 2026-08-14  
**Supersedes for new work:** [SPEC-v0.1.md](./SPEC-v0.1.md) (frozen historical evidence)

This document is the normative TL-PX 0.2 **decision, evaluation-error, authorization-state, and compatibility** contract. It is not yet a complete 0.2 implementation specification.

| Later slice | Still required |
| --- | --- |
| 2.2 | Done in this document §14 and `tests/fixtures/tlpx-0.2/jcs/` |
| 2.3 | JSON Schemas under `schemas/tlpx-0.2/`, receipt field schemas, validators, full reason-code catalog, distinct record conformance suite |
| 2.4 | Policy precedence, provenance, trusted ordering, requirements-maturity labels |

**Not legal advice. Not a patent claim set.** Product and protocol language only.

---

## 0. How to read this draft

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, **MAY** are to be interpreted as in RFC 2119.

An implementation MUST NOT claim **“TL-PX 0.2 Minimum conforming”** until slice 2.3 publishes schemas and a distinct conformance suite and that suite is passed. This document is still the normative vocabulary for work that targets 0.2.

The current JavaScript reference remains a TL-PX **0.1** implementation plus Phase 1 fail-closed policy compile. It MUST continue to emit `standard_version: "0.1.0"` until a separately versioned 0.2 adapter exists. It MUST NOT be silently treated as 0.2-conforming.

---

## 1. Relationship to TL-PX 0.1

TL-PX 0.1 and its 47 conformance fixtures are frozen historical evidence. They MUST NOT be rewritten so that 0.1 appears to have meant 0.2.

0.2 is a breaking change under the 0.1 SemVer rule. Material breaks include:

| 0.1 | 0.2 |
| --- | --- |
| Decisions `ALLOW` \| `REQUIRE_APPROVAL` | Decisions `ALLOW` \| `REQUIRE_APPROVAL` \| `DENY` |
| Switchboard `DENY` is a runtime accident that fails the 0.1 schema | `DENY` is a first-class successful evaluation |
| No evaluation-error record | `tlpx.evaluation_error` is a distinct terminal evaluation record |
| `ALLOW` → `AUTHORIZED` (immediately executable in conforming adapters) | `ALLOW` → `AUTHORIZED_UNCLAIMED` (must be claimed once) |
| `PENDING_HUMAN_APPROVAL` | `PENDING_APPROVAL` |
| Operator reject → `DENIED` | Operator reject → `REJECTED` (distinct from evaluation `DENIED`) |
| Single `actor` / declarer | Distinct authenticated requester and executor |
| Caller `risk` used as if trusted | `declared_risk` advisory; `derived_risk` / `effective_risk` are authority-owned |
| Implicit no-match → `ALLOW` for a valid pack | Explicit policy default required |
| Execution record may say `EXECUTED` from `AUTHORIZED` | Execution requires an atomic claim; `AUTHORIZED` is never proof of execution |

### 1.1 Compatibility rules

A 0.2 implementation, reader, or migrator MUST classify every 0.1 artifact as exactly one of:

1. **Read-only historical** — display and verify as 0.1; do not authorize from it.
2. **Explicitly upgraded** — a documented, logged conversion that produces a **new** 0.2 evaluation. Upgrade MUST re-evaluate; it MUST NOT mint a 0.2 authorization from a 0.1 `AUTHORIZED` row.
3. **Unsupported** — refuse to load for authorization purposes.

It MUST NOT:

- pretend a 0.1 `AUTHORIZED` status is a 0.2 claimable authorization;
- validate 0.2 records with 0.1 schemas, or 0.1 records with 0.2 schemas, and call that conformance;
- emit `standard_version: "0.1.0"` for 0.2 record shapes, or `0.2.0` for 0.1 record shapes;
- “fix” 0.1 schemas in place to accept `DENY` or `EVALUATION_ERROR`.

A 0.1 `AUTHORIZED` value means “the 0.1 gate recorded permission under the 0.1 cooperative executor contract.” It is **not** a 0.2 one-time claim.

---

## 2. Foundational invariant

> One authorization permits one authenticated executor to perform one exact action, one time.

Authorization and execution are distinct security events. No later object may be inferred merely from the existence of an earlier one.

```text
declared intent
  -> authorized action
  -> claimed execution
  -> observed terminal result
```

An authorization is not an open connection, reusable approval, session permission, group permission, transferable credential, or a doorway for another agent.

---

## 3. What 0.2 guarantees and does not

### 3.1 Guarantees (contract)

A 0.2-conforming authority, once implemented, MUST:

1. Authenticate requester and executor rather than trusting request-body name strings.
2. Run Switchboard (or an equivalent identity and action-scope gate) before policy.
3. Evaluate policy deterministically with no generative model in the `ALLOW` / `REQUIRE_APPROVAL` / `DENY` path.
4. Treat invalid, ambiguous, unavailable, or unverifiable inputs as evaluation errors, not as non-matches.
5. Issue at most one claimable authorization for one exact Authorized Action.
6. Require an atomic online claim before any protected side effect begins.
7. Seal every evaluation, including errors and denials, into the evidence chain.
8. Keep Submitted Intent, Authorized Action, Executed Action, and Execution Receipt as four distinct objects.

### 3.2 Non-guarantees

- Processes that never present work to the authority are not stopped by this contract. Forced mediation is a deployment and PEP property (Phase 3).
- Declared intent may be false or incomplete. This standard does not inspect hidden reasoning.
- A signed portable proof is not sufficient permission. Offline verification cannot establish global non-consumption.
- Records are accountability evidence, not legal judgments.

---

## 4. Four distinct objects

These names are exact. Implementations MUST NOT use “canonical action envelope” as an umbrella type that mixes requester declarations with authority-derived values.

| Object | Who produces it | What it is |
| --- | --- | --- |
| **Submitted Intent** | Requester, then validated by the authority | Fields available at submission time |
| **Authorized Action** | Authority, after authentication, Switchboard, and policy | The exact action that may be claimed |
| **Executed Action** | PEP, immediately before side effect | The exact normalized operation about to be performed |
| **Execution Receipt** | PEP / authority after the attempt | Observed terminal result of one claim |

Hash preimages (algorithm profile is slice 2.2) MUST be computed over the matching object only:

```text
intent_hash              over Submitted Intent
authorized_action_hash   over Authorized Action
executed_action_hash     over Executed Action
```

Before any side effect, the PEP MUST establish `authorized_action_hash == executed_action_hash`. A mismatch MUST block and MUST record `ACTION_MISMATCH`.

### 4.1 Submitted Intent

The requester MAY supply only fields knowable at submission. The requester MUST NOT supply `derived_risk`, `effective_risk`, policy outputs, authorization constraints, authorization ids, or an authorization nonce.

Required conceptual fields:

| Field | Meaning |
| --- | --- |
| `requesting_principal` | Proposed requester identity (authenticated separately) |
| `executing_principal` | Proposed executor identity (authenticated separately) |
| `action` | Machine action code |
| `intent_class` | Action class |
| `target` | Target resource |
| `arguments` | Normalized arguments |
| `environment` | Deployment environment |
| `tenant` | Tenant |
| `declared_risk` | Advisory caller risk |
| `data_classes` | Declared data classes |
| `requested_capability` | Requested capability |
| `resource_scope` | Requested resource scope |
| `payload_hash` | Digest of transferred payload, or `null` |
| `artifact_hash` | Digest of executable/artifact, or `null` |
| `adapter` | Adapter id and version |
| `request_id` | Requester-scoped idempotency / correlation id |

Changing any material submitted-intent field requires a new evaluation. Duplicate `request_id` values within an authenticated requester scope MUST return the existing result and MUST NOT collide globally.

### 4.2 Authorized Action

After authentication, Switchboard, validation, and policy, the authority constructs a separate object. It MUST contain authenticated requester and executor identities, normalized action/target/arguments, validated payload/artifact digests, trusted environment and tenant, policy-derived capability and resource constraints, effective data classifications, `derived_risk`, `effective_risk`, risk reasons/source, policy-bundle hash, and adapter binding.

It MUST exclude requester-only correlation fields unless a 0.2 schema (slice 2.3) explicitly includes them.

Caller-declared risk is advisory. Policy derives authoritative risk. A requester MAY raise and MUST NEVER lower effective risk. Missing required classification MUST fail closed as an evaluation error. Under-declaration is retained as evidence.

### 4.3 Executed Action and Execution Receipt

The PEP computes `executed_action_hash` over the operation it is about to perform, not over the original request object.

Every successful claim MUST reach one durable terminal Execution Receipt. Slice 2.3 specifies the schema. The receipt MUST at least bind:

- execution and claim ids;
- authorization and decision receipt ids;
- authenticated requester and executor;
- the three action hashes;
- protected target reference;
- policy bundle identity, version, and digest;
- adapter identity and version;
- trusted sequence number;
- start and terminal timestamps;
- terminal state;
- bounded result summary or digest;
- audit integrity metadata.

Receipts prove what the trusted path observed. They do not prove hidden intent or unobserved external effects.

---

## 5. Decision model

### 5.1 Control mode

0.2 Minimum control mode:

```text
ALLOW_ESCALATE_OR_DENY
```

Implementations advertising TL-PX 0.2 Minimum MUST use this control mode on decision records. 0.1 `ALLOW_OR_ESCALATE` remains valid only on 0.1 records.

### 5.2 Decisions

A **decision** is a successful evaluation of an exact request.

| Decision | Authorization state | Meaning |
| --- | --- | --- |
| `ALLOW` | `AUTHORIZED_UNCLAIMED` | The named executor may claim this exact Authorized Action before `claim_expires_at`. |
| `REQUIRE_APPROVAL` | `PENDING_APPROVAL` | A permitted authenticated human must decide. No claim is possible yet. |
| `DENY` | `DENIED` | Terminal refusal. It MUST NOT be approved, claimed, or executed. |

`DENY` means the authority **successfully evaluated** the request and refused it. It is terminal for that request and is **not** automatically retryable. Repeating the same idempotent request MUST return the same refusal. A new evaluation requires a material change such as a new action, principal scope, policy version, revocation state, or environment.

Policy `DENY` and Switchboard `DENY` are both `decision: "DENY"`. They MUST use distinct `policy_id` / reason-code values so the refusing stage is visible. They MUST validate against the 0.2 decision schema (slice 2.3). They MUST NOT be recorded as `tlpx.evaluation_error`.

### 5.3 What is not a decision

Policy compilation failure, evaluation failure, authentication failure, missing action data, unavailable policy, and infrastructure failure MUST produce `tlpx.evaluation_error`, not a fourth decision and not `DENY`.

---

## 6. Evaluation errors

`record_type`: **`tlpx.evaluation_error`**

An evaluation error:

- creates **no** authorization;
- always blocks;
- is terminal for that evaluation attempt;
- MUST still receive a receipt/correlation id, trusted sequence number, timestamp, non-sensitive error code and stage, available policy/adapter identity, and a sealed audit row.

Induced errors MUST NOT create gaps in the evidence sequence.

### 6.1 Retryability

`DENY` is not retryable except by a new, materially different evaluation.

`EVALUATION_ERROR` MUST declare `retryability`:

| Value | Meaning |
| --- | --- |
| `NEVER` | Do not retry this evaluation. |
| `AFTER_CONDITION` | Retry only after `required_condition` is met; honor `retry_after` if present. |
| `IMMEDIATE` | A bounded immediate retry is permitted. |

Optional `retry_after` and `required_condition` constrain retries and MUST be used to prevent retry storms. Implementations MUST NOT treat an error as a policy non-match.

Phase 1 JS compile failures remain process/load failures and MUST NOT emit a 0.2 error record from the 0.1 reference.

---

## 7. Authorization lifecycle

```text
EVALUATED
  |-- DENIED
  |-- PENDING_APPROVAL
  |     |-- REJECTED
  |     |-- APPROVAL_EXPIRED
  |     |-- CANCELLED
  |     `-- AUTHORIZED_UNCLAIMED
  `-- AUTHORIZED_UNCLAIMED
          |-- EXPIRED
          |-- REVOKED
          `-- CLAIMED
                |-- COMPLETED
                |-- FAILED
                |-- CANCELLED
                |-- LEASE_EXPIRED
                `-- EXECUTION_OUTCOME_UNKNOWN
                      `-- RECONCILIATION_REQUIRED
                            |-- COMPLETED_CONFIRMED
                            |-- FAILED_CONFIRMED
                            `-- OUTCOME_UNKNOWN_FINAL
```

`EVALUATED` is the branch point after a successful decision. An evaluation error never enters this tree.

### 7.1 State invariants

- Only `AUTHORIZED_UNCLAIMED` MAY transition to `CLAIMED`.
- Claiming consumes the authorization immediately.
- `CLAIMED` MUST NOT return to `AUTHORIZED_UNCLAIMED`.
- Terminal states: `DENIED`, `REJECTED`, `APPROVAL_EXPIRED`, `EXPIRED`, `REVOKED`, `COMPLETED`, `FAILED`, `CANCELLED`, `LEASE_EXPIRED`, `COMPLETED_CONFIRMED`, `FAILED_CONFIRMED`, `OUTCOME_UNKNOWN_FINAL`.
- One authorization MAY produce at most one successful claim.
- One decision receipt MAY have at most one terminal human outcome (`APPROVE` \| `REJECT` \| `CANCEL`).
- One one-time authorization MAY produce at most one terminal execution.
- Completion, failure, cancellation, rejection, or expiration permanently closes permission.
- Approval after `APPROVAL_EXPIRED` or `CANCELLED` requires a new evaluation.
- `EXECUTION_OUTCOME_UNKNOWN` permanently consumes authorization and MUST NEVER trigger automatic re-execution.

### 7.2 `DENIED` versus `REJECTED`

| State | Origin |
| --- | --- |
| `DENIED` | Successful evaluation decided `DENY` (Switchboard or policy). |
| `REJECTED` | A `PENDING_APPROVAL` request was refused by an authenticated operator. |

Readers MUST NOT collapse these. Both are terminal and both refuse execution.

### 7.3 Human outcomes on `PENDING_APPROVAL`

| Outcome | Next state |
| --- | --- |
| `APPROVE` | `AUTHORIZED_UNCLAIMED` |
| `REJECT` | `REJECTED` |
| `CANCEL` | `CANCELLED` |
| clock / authority expiry | `APPROVAL_EXPIRED` |

Late `APPROVE` after `APPROVAL_EXPIRED` or `CANCELLED` MUST be rejected. The operator must cause a new evaluation.

### 7.4 Authorized cancellation

While `PENDING_APPROVAL`, cancellation MAY be initiated only by:

- the authenticated original requester for its own request;
- an authenticated operator authorized for the approval route;
- an authenticated revocation or emergency authority;
- the authority itself for expiry, shutdown, invalidated policy, disabled principal, or revoked scope.

Cancellation and approval race atomically; exactly one transition wins. Unauthenticated cancellation MUST be rejected.

After claim, cancellation is a request to stop work, not proof that an external effect was reversed. Execution evidence MUST distinguish `CANCELLED_BEFORE_SIDE_EFFECT`, `CANCELLATION_REQUESTED`, `CANCELLED_DURING_EXECUTION`, `CANCELLATION_UNSUPPORTED`, and `COMPLETED_BEFORE_CANCELLATION` once those fields are schemed in slice 2.3.

### 7.5 Timing

Authorization lifetime and execution lifetime are separate. The short claim timer begins when authorization is issued, not while a human is deciding.

| Stage | Initial default |
| --- | ---: |
| Human approval request | 5–15 minutes |
| Automatic authorization claim window | 3–5 seconds |
| Post-approval claim window | 5 seconds |
| Local clock-skew tolerance | approximately 1 second |
| Execution lease | action-specific |

Expiration alone is not replay protection. Claim MUST be transactional.

### 7.6 Crash after a possible side effect

If the PEP may have performed the side effect but no durable completion receipt exists, the authority MUST assign `EXECUTION_OUTCOME_UNKNOWN` then `RECONCILIATION_REQUIRED`. Authorization remains consumed. Reconciliation MAY resolve to `COMPLETED_CONFIRMED`, `FAILED_CONFIRMED`, or `OUTCOME_UNKNOWN_FINAL`. It MUST NOT automatically repeat an irreversible action.

---

## 8. Record types

0.2 Minimum record types:

| `record_type` | When emitted |
| --- | --- |
| `tlpx.decision` | Successful evaluation: `ALLOW`, `REQUIRE_APPROVAL`, or `DENY` |
| `tlpx.evaluation_error` | Evaluation could not be established |
| `tlpx.operator_action` | Authenticated `APPROVE`, `REJECT`, or `CANCEL` |
| `tlpx.authorization` | A claimable authorization is issued (`AUTHORIZED_UNCLAIMED`) |
| `tlpx.authorization_claim` | Atomic consumption of one authorization (`CLAIMED`) |
| `tlpx.execution` | Observed execution result after a successful claim |

`glass.*` aliases are **not** part of 0.2 Minimum. Product aliases MAY exist as extensions; 0.2 conformance uses `tlpx.*` only.

### 8.1 Decision record (normative fields)

A 0.2 decision record MUST include:

- `record_type` = `tlpx.decision`
- `standard` = `TL-PX`
- `standard_version` = `0.2.0`
- `control_mode` = `ALLOW_ESCALATE_OR_DENY`
- `receipt_id`
- `evaluated_at`
- `decision` = `ALLOW` \| `REQUIRE_APPROVAL` \| `DENY`
- `authorization_state` = the mapped state from §5.2
- `reason` (human-readable, non-sensitive)
- `reason_code` (stable machine code)
- `policy_id` (string or `null` if no rule matched **and** an explicit default produced the decision)
- `policy_bundle_id`, `policy_bundle_version`, `policy_bundle_hash` (hash profile: slice 2.2)
- `intent_hash`
- parties: authenticated requester, evaluator (`type: machine`), router when Switchboard ran
- `sequence` (trusted local sequence number)

`ALLOW` MUST map to `authorization_state = AUTHORIZED_UNCLAIMED`.  
`REQUIRE_APPROVAL` (unresolved) MUST map to `PENDING_APPROVAL`.  
`DENY` MUST map to `DENIED`.

A 0.2 decision MUST NOT use 0.1 field `authorization_status` as its primary state field. Readers of mixed logs MUST key on `standard_version`.

### 8.2 Evaluation error record (normative fields)

MUST include: `record_type`, `standard`, `standard_version`, `receipt_id`, `occurred_at`, `stage`, `error_code`, `retryability`, `sequence`, and non-sensitive `reason`. MAY include `retry_after`, `required_condition`, and available policy/adapter identity.

Sealing is an evidence-storage property, not a field on the record. The error record MUST be appended to the sealed evidence chain with a trusted sequence number. It MUST NOT include an authorization id and MUST NOT set an authorization state other than absent/null.

### 8.3 Authorization record (normative fields)

Issued only for `AUTHORIZED_UNCLAIMED`. MUST bind at least:

```text
authorization_id
receipt_id
requesting_principal
executing_principal
action
target
authorized_action_hash
intent_hash
environment
tenant
adapter.id
adapter.version
issued_at
claim_expires_at
execution_lease_seconds
authorization_nonce          # authority-generated
idempotency_key
state = AUTHORIZED_UNCLAIMED
```

The requester MUST NOT supply `authorization_id` or `authorization_nonce`. Northstar generates authorization ids, nonces, and later claim ids.

### 8.3.1 Portable claim ticket

A later portable **authorization claim ticket** or **capability proof** MAY exist. It is proof of issuance, not permission to act.

Every such ticket MUST:

- bind one authenticated executor;
- bind one `authorized_action_hash`;
- bind one adapter identity and version, environment, and tenant;
- bind one short claim window (`claim_expires_at`);
- identify issuer, ticket type and version, signing algorithm, key id, trust domain, and signature;
- be consumed by exactly one atomic online claim against authoritative state before any side effect begins.

A PEP MAY verify signature, executor binding, `authorized_action_hash`, adapter/environment/tenant scope, and expiry locally. Local signature verification MUST NOT authorize a side effect. Offline verification cannot establish global non-consumption.

The ticket MUST NOT be a bearer `AUTHORIZED` snapshot. Slice 2.1 forbids implementing the abandoned August 7 `tlpx.authz_token` semantics.

### 8.3.2 Claim record

Atomic consumption MUST be represented by a dedicated `tlpx.authorization_claim` record. Implementations MUST NOT invent a private consumption log and MUST NOT treat an intermediate `tlpx.execution` as the claim.

A claim record MUST include at least: `record_type` = `tlpx.authorization_claim`, `standard`, `standard_version`, `claim_id` (authority-generated), `authorization_id`, linked decision `receipt_id`, authenticated `executing_principal`, `authorized_action_hash`, `executed_action_hash` presented at claim, adapter identity and version, `claimed_at`, `lease_expires_at`, `sequence`, and `state` = `CLAIMED`.

Zero updated authoritative rows MUST produce a blocking reason from §9.3 and MUST NOT emit a successful claim record. A successful claim record MUST exist before `tlpx.execution` may record a side effect.

### 8.4 Operator action record

MUST include authenticated operator subject, `outcome` (`APPROVE` \| `REJECT` \| `CANCEL`), linked decision `receipt_id`, `authorized_action_hash` the human saw (for `APPROVE` / `REJECT`), policy-bundle digest, approval route / quorum rule, renderer identity/version, trusted sequence, and timestamp.

`APPROVE` MUST produce a new `tlpx.authorization` in `AUTHORIZED_UNCLAIMED`. It MUST NOT mutate a previous authorization in place.

---

## 9. Core reason codes

Slice 2.3 publishes the full catalog. The following codes are already normative so 0.2 logs do not invent synonyms for the same event.

### 9.1 Successful evaluation

| Code | Typical decision |
| --- | --- |
| `POLICY_ALLOW` | `ALLOW` via matching or explicit default |
| `POLICY_REQUIRE_APPROVAL` | `REQUIRE_APPROVAL` |
| `POLICY_DENY` | `DENY` from policy |
| `SWITCHBOARD_UNKNOWN_PRINCIPAL` | `DENY` |
| `SWITCHBOARD_NOT_WHITELISTED` | `DENY` |
| `SWITCHBOARD_ACTION_DENIED` | `DENY` |

### 9.2 Evaluation errors

| Code | Typical retryability |
| --- | --- |
| `POLICY_UNAVAILABLE` | `AFTER_CONDITION` |
| `POLICY_COMPILE_FAILED` | `NEVER` |
| `INTENT_INVALID` | `NEVER` |
| `ACTION_DATA_AMBIGUOUS` | `NEVER` |
| `AUTHENTICATION_FAILED` | `AFTER_CONDITION` |
| `AUTHORITY_INTERNAL_ERROR` | `AFTER_CONDITION` or `NEVER` |

### 9.3 Claim and execution blocks

| Code | Meaning |
| --- | --- |
| `AUTHORIZATION_EXPIRED` | Claim window passed |
| `ALREADY_CLAIMED` | Consumed by a prior claim |
| `EXECUTOR_MISMATCH` | Authenticated executor is not the named executor |
| `ACTION_MISMATCH` | Executed Action hash ≠ Authorized Action hash |
| `AUTHORIZATION_DENIED` | No authorization exists because evaluation was `DENIED` |
| `AUTHORIZATION_TERMINAL` | State is already terminal |
| `AUTHORIZATION_REVOKED` | Revoked before or at claim |

Unknown codes MUST fail closed for authorization (do not treat as allow). Display MAY show the raw code.

---

## 10. Policy default (0.2 evaluate rule)

Every 0.2 policy bundle MUST declare an explicit default outcome: `ALLOW`, `REQUIRE_APPROVAL`, or `DENY`.

Missing or invalid default MUST prevent bundle activation. If that state is reached at runtime, emit `tlpx.evaluation_error` and issue no authorization.

An explicit `default: ALLOW` is legal in the Minimum Profile after Switchboard has authenticated and scoped the principal. Hardened profiles MAY require `default: DENY` or `default: REQUIRE_APPROVAL`. The Minimum Profile MUST NOT assume every deployment has that posture.

0.1 valid packs keep implicit no-match → `ALLOW`. That rule remains frozen on the 0.1 line only.

---

## 11. Identity and handoff (contract level)

Requester, operator, executor, and canceller identities MUST come from authenticated context.

Agent B MUST NOT use Agent A’s authorization. A valid handoff is a new evaluation that names B as `executing_principal` for the exact action B will perform. Parent permission is never transitive.

The abandoned August 7 snapshot token MUST NOT be used to carry handoff permission.

---

## 12. Party model

Durable 0.2 records MUST attribute at least:

| Role | Type | Required |
| --- | --- | --- |
| requester | `human` \| `machine` | On decision, authorization, execution |
| executor | `human` \| `machine` | On authorization and execution; proposed on intent |
| evaluator | `machine` | On decision and evaluation error |
| router | `machine` | When Switchboard (or equivalent) ran |
| authorizer | `human` | When an escalation is resolved by `APPROVE` or `REJECT` |
| canceller | `human` \| `machine` | When `CANCEL` is recorded |

---

## 13. Claims discipline

Until 0.2 schemas, conformance, and an independent review exist, implementations of this contract MUST be described as a draft specification or experimental preview — not as production-safe 0.2.

This document does not make the JavaScript reference a 0.2 authority. It does not implement atomic claim, PEP enforcement, or Rust. The 0.2 JCS/hash helpers are a fixture oracle, not a 0.2 decision engine.

---

## 14. Canonicalization and hashes (slice 2.2)

Security-critical 0.2 envelopes MUST be hashed only after they validate against their 0.2 schema (slice 2.3) and are canonicalized with this profile. Implementations MUST NOT hash a language’s default JSON serialization.

### 14.1 Northstar JCS profile

Canonicalization is RFC 8785 JSON Canonicalization Scheme (JCS), UTF-8 encoded, with these additional MUST rules:

- Duplicate object keys are rejected.
- Floating-point values are prohibited, including `1.0`, scientific notation, non-finite numbers, and `-0`.
- Integers MUST be exact JSON integer tokens in the IEEE-754 safe-integer range `[-9007199254740991, 9007199254740991]`. Slice 2.3 schemas MAY impose tighter per-field bounds; they MUST NOT widen this range.
- Absent keys and `null` are distinct: a missing key is omitted; `null` is serialized as `null`.
- Array order is preserved unless a field schema (slice 2.3) explicitly defines canonical sorting.
- Object keys are sorted lexicographically by **unsigned UTF-16 code units** (RFC 8785 §3.2.3), not by Unicode code points and not by UTF-8/UTF-32 code units. U+10000 therefore sorts before U+E000. Unicode is not NFC/NFD-normalized.
- Lone UTF-16 surrogates (unpaired `U+D800`–`U+DFFF`) MUST terminate canonicalization. Valid surrogate pairs remain legal.
- Strings follow RFC 8785 escaping: `"`, `\`, and `U+0000`–`U+001F` only. Other characters, including non-ASCII, appear as UTF-8.

The 0.1 audit helper `canonicalJson` is **not** this profile. 0.1 seals MUST continue to use the 0.1 function. 0.2 hashes MUST use this profile.

### 14.2 Domain-separated hashes

```text
intent_hash =
  SHA-256(UTF8("northstar:intent:v1\0") || JCS(Submitted Intent))

authorized_action_hash =
  SHA-256(UTF8("northstar:authorized-action:v1\0") || JCS(Authorized Action))

executed_action_hash =
  SHA-256(UTF8("northstar:executed-action:v1\0") || JCS(Executed Action))

approval_context_hash =
  SHA-256(UTF8("northstar:approval-context:v1\0") || JCS(approval context))
```

Each prefix includes a trailing NUL (`U+0000`). Implementations MUST hash the prefix bytes and the JCS UTF-8 bytes as a single SHA-256 input. They MUST NOT hash a merged object that mixes requester declarations with authority-derived values.

### 14.3 Hash string representation

The normative string form is ASCII `sha256:` followed by exactly 64 lowercase hexadecimal characters. No whitespace, truncation, uppercase, base64, or `0x` prefix. Pattern:

```text
^sha256:[0-9a-f]{64}$
```

Implementations MUST validate this representation before embedding a hash string in another canonical structure.

### 14.4 Golden fixtures

Cross-language vectors live at `tests/fixtures/tlpx-0.2/jcs/golden.json`. A 0.2 hash implementation MUST match every `accept` vector’s `canonical`, `canonical_utf8_hex`, `digest_hex`, and `sha256` fields, and MUST reject every `reject` vector. `digest_hex` is the raw 32-byte SHA-256 as 64 lowercase hex; `sha256` is `sha256:` plus that hex. Those fixtures are not the 47 frozen 0.1 tests.

---

## 15. Change log

| Version | Notes |
| --- | --- |
| 0.2.0-draft.2.1 | Decision/error/state/compatibility contract. `DENY` first-class. Distinct `EVALUATION_ERROR`. Lifecycle including claim and unknown-outcome. 0.1 frozen. Schemas, hashes, and conformance deferred. |
| 0.2.0-draft.2.1b | Claim ticket restated to §11.1 (adapter binding, short window, atomic online claim). Dedicated `tlpx.authorization_claim`. Sealing described as evidence-chain property, not a record field. |
| 0.2.0-draft.2.2 | Northstar JCS profile, `sha256:` representation, four domain prefixes, and golden fixtures. |
| 0.2.0-draft.2.2b | Key sort is RFC 8785 UTF-16 code units. Lone surrogates rejected. Fixtures include astral/BMP order and raw `digest_hex`. |
