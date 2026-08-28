# Northstar Build Specification Sheet

**Status:** Current hardened build baseline  
**Date:** 2026-08-11; disposition recorded 2026-08-13; implementation status updated 2026-08-27
**Repository:** `robkatzenberger/Project-Northstar`  
**Baseline commit:** `7a0b371e1307739e465f8c5bd313ef9372adc9be`  
**Prior tested implementation commit:** `ca05f6996534471e817d11f3c668e38411797fb8`  
**Current accepted implementation commit:** `aed80e2527f05a3730b1057f2d90c55a6c3eb646`
**Current builder-verified evidence/outbox commit:** `c9bdd0fcd2d4c51fda9f3861724db0fd97524003` (not independently accepted)
**Current slice 2.4 candidate:** local commit `a87f82271a12843c120d9a1e6ee238957f285c2f` contains the policy-bundle schema/hash fixture, deterministic precedence/selection oracle, trusted complete-stream sequence verifier, and maturity matrix. Pre-commit builder evidence is recorded in `tests/reports/slice-2.4-policy-ordering-builder-verification-2026-08-17.md`; exact-commit/full Section 3 verification and independent review are deferred, so it remains unaccepted.
**Current slice 3.1 candidate:** local commit `1addb5c6a0ede31d754ac0bd47d7ef1f3a05e6d4` integrates the 2.4 manifest contract into Rust: native manifest parsing/hash, exact-match content binding, deterministic per-evaluation selection, durable unavailable/ambiguous-policy errors, and claim-time active-policy recheck. Pre-commit builder evidence is recorded in `tests/reports/slice-3.1-rust-policy-activation-builder-verification-2026-08-17.md`; exact-commit/full Section 3 verification and independent review are deferred, so it remains unaccepted.
**Current slice 3.2 candidate:** local commit `e835c4e` adds shared schema-bound Submitted Intent, Authorized Action, Executed Action, and Action Binding fixtures; strict JavaScript canonical/hash helpers; Rust parity tests; exhaustive binding-mutation negatives; and effective-risk schema parity. The bounded exact-commit rerun passed and is recorded in `tests/reports/slice-3.2-typed-action-hash-builder-verification-2026-08-17.md`. Full Section 3 verification and independent review remain deferred, so it is not accepted.
**Current slice 3.3 candidate:** local commit `c19b1d2` adds safe kernel-derived Unix peer authentication, exact UID/GID-to-principal role mapping, authenticated requester/executor facades, policy-bound approval routes, and atomic pending cancellation with sealed operator-action evidence. The full exact-commit builder matrix passed and is recorded in `tests/reports/slice-3.3-local-auth-cancellation-builder-verification-2026-08-18.md`. Independent acceptance is deferred.
**Current slice 3.4 candidate:** local commit `133cd94` adds human-only route approval/rejection, canonical Authorized Action display binding, fresh post-approval authorization IDs/nonces, independent approval/claim deadlines, and atomic approval expiry. The full exact-commit builder matrix passed and is recorded in `tests/reports/slice-3.4-approval-expiry-builder-verification-2026-08-18.md`. Independent acceptance is deferred.
**Current slice 3.5 candidate:** local commit `ad95653` adds authenticated durable local revocation state and transactionally checks authorization, principal, policy, tenant, environment, and capability scopes during atomic one-time claim. The full exact-commit builder matrix passed and is recorded in `tests/reports/slice-3.5-transactional-revocation-builder-verification-2026-08-18.md`. Portable revocation evidence and signing-key enforcement remain deferred; independent acceptance is deferred.
**Current slice 3.6 candidate:** local commit `9028346` closes the terminal execution-receipt candidate, binds execution idempotency to authorization/executor/action, adds durable direct and reconciled terminal state, and atomically emits canonical sealed `tlpx.execution` evidence. The full exact-commit builder matrix passed and is recorded in `tests/reports/slice-3.6-execution-reconciliation-builder-verification-2026-08-18.md`. Independent acceptance is deferred.
**Current slice 3.7 candidate:** local commit `518899a` adds a mutually authenticated local Unix adapter session, startup-validated version/digest/capability/action/material-field contracts, execution-start revalidation, and adapter-bound terminal receipts. The full exact-commit builder matrix passed and is recorded in `tests/reports/slice-3.7-authenticated-adapter-builder-verification-2026-08-18.md`. Independent acceptance is deferred.
**Current slice 3.8 candidate:** local commit `7c41450ff4e4bd22155b91149a2c0ef6366cf5f6` adds the bounded same-UID `CooperativeShellRunner` and deliberately named `tlpx-run-demo`, makes public authority mutation paths require authenticated identities, distinguishes fresh execution start from non-permission retries, and makes unknown JavaScript symbolic hash domains fail closed. The full exact-commit builder matrix passed and is recorded in `tests/reports/slice-3.8-cooperative-shell-runner-builder-verification-2026-08-20.md`. This is cooperative execution, not forced mediation; independent acceptance is deferred.
**Current slice 3.9 candidate:** local commit `e6f2bb0a511628dc94f619716c3682b506d5aaf4` adds the bounded `tlpx-run` Unix service and a separate-OS-identity acceptance harness for one protected marker capability. Ordinary pre-commit checks passed, but the defining administrator-backed acceptance script was not run. It remains `PLANNED`, incomplete, and unaccepted; see `tests/reports/slice-3.9-restricted-pep-candidate-unverified-2026-08-27.md`.
**Current slice 4.1 candidate:** local implementation commit `980327d76a2b9e157bb52daeef9bf3a05e3e0d34`, exact-verified with test-isolation follow-up `efa7f0f71357e922c7dc65b2a379cbb11d04ff3a`, adds purpose-bound independent role keys, verify-only rotation, authority-local authorization proofs, and durable key-revocation enforcement. The full exact-commit builder/red-team matrix passed; evidence is in `tests/reports/slice-4.1-separated-key-roles-builder-verification-2026-08-27.md`. Independent acceptance is deferred until Phase 4 is complete.
**Current slice 4.2 candidate:** local commit `833d8d4a01f6a63b8f9ed06c20a531f26bdcc371` adds an append-and-sync-before-ack local audit sink, exact sealed-prefix reconciliation, crash-boundary recovery without duplication, protected file/lock handling, and ordered monotonic acknowledgement checks. The full exact-commit builder/red-team matrix passed; evidence is in `tests/reports/slice-4.2-durable-audit-export-builder-verification-2026-08-27.md`. Independent acceptance is deferred until Phase 4 is complete.
**Live implementation scope:** accepted slices 1.1–2.3 plus independently accepted 2.3d/Rust local-authority commit `aed80e2`: types/hashes, exact-match policy, SQLite decisions/errors/issuance/idempotency/revocation, authority-wide sequence, and atomic claim. Named commit `c9bdd0f` additionally emits the bounded decision/error/authorization/claim record core into a canonical, hash-chained, HMAC-sealed durable outbox; it is builder-verified, not independently accepted. Local candidates through 4.2 add policy activation, typed hashes, authenticated roles, approval/revocation/execution lifecycle, adapter contracts, cooperative and restricted PEP harnesses, separated role-key enforcement, and bounded local durable audit export. None is an accepted 0.2 runtime slice. The required 3.9 separate-identity test, active cancellation, portable revocation/expiry evidence, external audit transport, hardware-backed custody, and independent review remain open. No snapshot token.

**Current evidence-increment gate:** builder verification for `c9bdd0f` is recorded in `tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md`. Independent exact-commit review of that named commit is still required. Do not broaden that gate into authenticated transport, execution receipts, cancellation/reconciliation evidence, revocation evidence, or a PEP.

**Continuity:** [`reviews/build-plan-review-disposition-2026-08-13.md`](./reviews/build-plan-review-disposition-2026-08-13.md)

**Document role:** delivery sequence, acceptance criteria, and maturity labels. Normative protocol semantics live in [`standard/SPEC-v0.2.md`](./standard/SPEC-v0.2.md). Do not expand architecture here unless a delivery slice or implementation evidence requires it.

**2.3 acceptance clarification (2026-08-14):** accepted 2.3 coverage is the object schemas plus decision, evaluation-error, operator-action, authorization, and authorization-claim records, validators, and reason-code catalog. Execution receipt, cancellation/reconciliation, and revocation evidence closure was not exercised by the 2.3 conformance suite. Local slice 3.6 later closes an implementation-driven terminal execution-receipt candidate; it does not retroactively broaden 2.3 acceptance. Portable revocation evidence remains deferred.

**Current requirements maturity**

| Requirement area | Maturity | Acceptance evidence |
| --- | --- | --- |
| Slices 1.1–2.3 evaluation/authorization contract core | **IMPLEMENTED** | Accepted reports and named commits through `03d293b` plus follow-up contract commits |
| 2.3d bounded Rust evaluate/issue/claim authority | **IMPLEMENTED** | Independently accepted at `aed80e2` |
| Bounded Rust schema evidence and sealed local outbox | **IMPLEMENTED** | Builder-verified at `c9bdd0f`; independent acceptance pending |
| Slice 2.4 policy provenance/precedence/ordering contract | **PLANNED** | Local commit `a87f822`; pre-commit builder run passed, exact-commit/full Section 3 verification deferred |
| Slice 3.1 Rust policy-manifest activation boundary | **PLANNED** | Local commit `1addb5c`; pre-commit builder run passed, exact-commit/full Section 3 verification deferred |
| Slice 3.2 typed actions, distinct hashes, and exact Action Binding | **IMPLEMENTED** | Bounded exact-commit checks passed at local commit `e835c4e`; full Section 3 verification and independent review deferred |
| Slice 3.3 authenticated local roles and pending cancellation | **IMPLEMENTED** | Full exact-commit builder matrix passed at local commit `c19b1d2`; full Section 3 and independent review deferred |
| Slice 3.4 human approval resolution and approval expiry | **IMPLEMENTED** | Full exact-commit builder matrix passed at local commit `133cd94`; full Section 3 and independent review deferred |
| Slice 3.5 transactional revocation and one-time claim | **IMPLEMENTED** | Full exact-commit builder matrix passed at local commit `ad95653`; local revocation state is not portable 0.2 evidence; full Section 3 and independent review deferred |
| Slice 3.6 idempotent terminal execution and reconciliation | **IMPLEMENTED** | Full exact-commit builder matrix passed at local commit `9028346`; full Section 3 and independent review deferred |
| Slice 3.7 authenticated adapter contract and integrity checks | **IMPLEMENTED** | Full exact-commit builder matrix passed at local commit `518899a`; bounded local profile only; full Section 3 and independent review deferred |
| Slice 3.8 bounded cooperative shell runner | **IMPLEMENTED** | Full exact-commit builder matrix passed at local commit `7c41450`; same-UID cooperative profile only; full Section 3 and independent review deferred |
| Slice 3.9 restricted-agent authenticated enforcement | **PLANNED** | Candidate at local commit `e6f2bb0`; ordinary checks passed, but the defining separate-identity acceptance script was not run |
| Slice 4.1 separated key roles, rotation, and revocation | **IMPLEMENTED** | Full exact-commit builder/red-team matrix passed through local commit `efa7f0f`; independent Phase 4 review deferred |
| Slice 4.2 durable audit export and reconciliation | **IMPLEMENTED** | Full exact-commit builder/red-team matrix passed at local commit `833d8d4`; bounded local file profile; independent Phase 4 review deferred |
| Active cancellation, portable revocation/expiry evidence, and PEP | **PLANNED** | No accepted implementation |
| M-of-N, anomaly scoring, ZK, dual ledgers, hierarchical authorization | **EXTENSION_EXPERIMENTAL** | Deferred profiles only |

Maturity is not acceptance. `IMPLEMENTED` says a named builder-verified artifact exists; the evidence column separately states whether that artifact is independently accepted.

## 1. Purpose

Northstar is a pre-execution authorization boundary for agentic and human-initiated actions. It is not an AI judge, an agent orchestrator, or a substitute for operating-system containment.

Northstar must determine whether an authenticated principal may perform a declared action before that action reaches a protected capability. It must then enforce the decision, preserve a verifiable evidence trail, and fail closed when identity, policy, authorization, state, or integrity cannot be established.

The system itself is part of the trusted computing base. Incorrect authorization or compromise of Northstar could enable the harm it exists to prevent. The authoritative implementation must therefore remain small, deterministic, fail-closed, independently testable, and replaceable behind language-neutral contracts.

This sheet is the destination architecture, not a single implementation assignment. Specification completeness is genuine progress when it closes ambiguity before code fossilizes it. It is not enforcement progress. Do not implement beyond the live scope recorded in the header.

The August 7 roadmap in `docs/roadmap/priorities.md` is superseded as current direction and retained as history. The August 7 `AUTHORIZED` snapshot token is abandoned; see §11.1.

## 2. Foundational invariant

> One authorization permits one authenticated executor to perform one exact action, one time.

Authorization and execution are distinct security events. Authorization proves only that a precisely defined action may begin; it does not prove that the same action began, completed, or produced the intended real-world effect. Northstar must preserve and verify the complete chain from declared intent through observed execution.

An authorization is not:

- an open connection;
- a reusable approval receipt;
- session-wide permission;
- permission for an agent group;
- a transferable credential;
- a temporarily open doorway for another agent.

Northstar MUST prevent an agent from piggybacking on, forwarding, sharing, replaying, or holding open another agent's authorization.

## 3. Initial success path

The first hardened release must prove one complete path:

```text
Authenticated agent
  -> Submitted Intent
  -> Switchboard identity and action-scope check
  -> strict deterministic policy evaluation
  -> human approval when required
  -> single-use authorization bound to one Authorized Action
  -> atomic claim by the named executor
  -> protected execution recorded as one Executed Action
  -> Execution Receipt
  -> terminal state
  -> sealed evidence
```

Breadth comes after this path survives adversarial, concurrency, replay, mutation, expiration, bypass, and crash-recovery testing.

The hardened path must preserve four distinct objects:

```text
declared intent
  -> authorized action
  -> claimed execution
  -> observed terminal result
```

No later object may be inferred merely from the existence of an earlier one.

## 4. System architecture

```text
Applications and agents
Java | Go | Python | TypeScript
             |
             | language-neutral request protocol
             v
    Rust Northstar authority
    +-------------------------------+
    | authenticated identity        |
    | Switchboard scope             |
    | strict policy                 |
    | human approval state          |
    | one-time authorization        |
    | atomic transactional claim    |
    | protected execution / PEP     |
    | durable state + sealed audit  |
    +-------------------------------+
             |
             v
       Protected capability
```

### 4.1 Authoritative Rust core

The security-critical authority will be implemented in Rust and will own:

- principal authentication;
- Switchboard registration, whitelist, credibility, and action scope;
- strict policy compilation and evaluation;
- canonical action normalization and hashing;
- approval state;
- authorization issuance, expiration, and atomic claim;
- transactional state transitions;
- protected execution;
- sealed audit generation and reconciliation.

Requirements for the authority:

- no `unsafe` Rust in the authorization path;
- no LLM calls in the decision path;
- no arbitrary policy code or dynamic code loading;
- no in-process plugin system;
- minimal, reviewed dependencies;
- explicit error handling;
- deterministic decisions for identical valid inputs and policy;
- fail-closed behavior for all uncertain or invalid states.

### 4.2 Language-neutral contract

JSON Schemas and an API contract define interoperability. All implementations and SDKs must share:

- canonical action serialization;
- action hashing rules;
- decision vocabulary;
- authorization states;
- reason codes;
- receipt and audit record shapes;
- conformance fixtures.

### 4.3 TypeScript reference and test oracle

TypeScript will remain the readable reference, conformance oracle, example client, and adversarial test harness. It is not an alternative production authorization authority.

### 4.4 Client SDKs and adapters

Java, Go, Python, and TypeScript components are clients and adapters. They may collect intent, submit actions, display decisions, and verify portable authorization proofs. Local proof verification does not permit execution without the required online transactional claim. They must not independently weaken or redefine authoritative authorization semantics.

## 5. Decision model

| Decision | Authorization state | Meaning |
| --- | --- | --- |
| `ALLOW` | `AUTHORIZED_UNCLAIMED` | The named executor may claim one exact action before expiration. |
| `REQUIRE_APPROVAL` | `PENDING_APPROVAL` | A permitted authenticated human must decide. |
| `DENY` | `DENIED` | Terminal refusal; it cannot be approved or executed. |

`DENY` means Northstar successfully evaluated the exact request and refused it. It is terminal for that request and is not automatically retryable. Repeating the same idempotent request returns the same refusal; a new evaluation requires a material change such as a new action, principal scope, policy version, revocation state, or environment.

Policy compilation, evaluation, or infrastructure failure produces a terminal `tlpx.evaluation_error` record, not a fourth policy decision and not `DENY`. It creates no authorization and always blocks. Error records declare `retryability` as `NEVER`, `AFTER_CONDITION`, or `IMMEDIATE`; `required_condition` is present exactly for `AFTER_CONDITION`, and optional `retry_after` constrains retries and prevents retry storms. Retryability permits a successor evaluation under a new request id; it never rewrites the errored idempotency slot.

Every evaluation error still receives a receipt/correlation ID, trusted sequence number, timestamp, non-sensitive error code and stage, available policy/adapter identity, and sealed audit row when authoritative evidence storage can commit. Storage failure blocks and issues nothing; Northstar must not claim an uncommitted row, sequence, or idempotency slot exists.

## 6. Submitted intent and canonical authorized action

The requester submits an intent object containing only fields available at submission time. That object includes the proposed requester/executor, action, intent class, target, arguments, environment, tenant, `declared_risk`, declared data classes, requested capability, resource scope, payload/artifact digests, adapter identity, requester-scoped `request_id`, and optional non-authorizing `retry_of_receipt_id`. The requester cannot supply trusted `derived_risk`, `effective_risk`, policy outputs, authorization constraints, or an authorization nonce.

Example submitted intent:

```json
{
  "requesting_principal": "agent.a",
  "executing_principal": "agent.a",
  "action": "send_email",
  "intent_class": "external_communication",
  "target": "customer:123",
  "arguments": {
    "template": "invoice-ready",
    "invoice_id": "inv_456"
  },
  "environment": "production",
  "tenant": "tenant_abc",
  "declared_risk": "medium",
  "data_classes": ["PII"],
  "requested_capability": "mailer.send",
  "resource_scope": ["customer:123"],
  "payload_hash": "sha256:...",
  "artifact_hash": null,
  "adapter": {
    "id": "adapter.mailer",
    "version": "1.0.0"
  },
  "request_id": "requester-scoped-idempotency-value"
}
```

After authentication, Switchboard, validation, and policy evaluation, Northstar constructs a separate authority-normalized authorized-action object. It contains authenticated requester/executor identities, normalized action/target/arguments, validated payload/artifact digests, trusted environment and tenant, policy-derived capability/resource constraints, effective data classifications, `derived_risk`, `effective_risk`, risk reasons/source, policy-bundle hash, adapter binding, and other execution constraints. It excludes requester-only correlation fields unless the v0.2 schema explicitly includes them.

`intent_hash` is computed only over the validated submitted-intent schema. `authorized_action_hash` is computed only over the authority-normalized authorized-action schema. `executed_action_hash` is computed by the PEP over the normalized operation it is actually about to perform. These are distinct schema-defined preimages; implementations must not hash a merged convenience object containing both requester declarations and authority-derived values.

Caller-declared risk is advisory only. Trusted policy derives authoritative risk from action, target, capability, data classes, environment, resource scope, and other trusted context. A requester may raise but never lower effective risk. Missing required classification fails closed; under-declaration is retained as evidence and may inform credibility.

### 6.1 Canonicalization and hash representation

Northstar must validate each hashable object against its schema and canonicalize it using RFC 8785 JSON Canonicalization Scheme (JCS), UTF-8 encoding, and the following profile:

- duplicate object keys are rejected;
- floating-point values are prohibited in security-critical envelopes, including `1.0`, scientific notation, non-finite numbers, and `-0`;
- integers are exact JSON integer tokens in the IEEE-754 safe-integer range unless a later schema tightens a field;
- absent and `null` remain distinct;
- array order is preserved unless a field schema explicitly defines canonical sorting;
- object keys are sorted by **unsigned UTF-16 code units** (RFC 8785 §3.2.3), not Unicode code points and not UTF-8/UTF-32;
- lone UTF-16 surrogates terminate canonicalization; valid pairs are allowed;
- Unicode is not NFC/NFD-normalized;
- no implementation may hash its language's default JSON serialization.

Slice 2.2 implemented this profile in the JS oracle (`implementations/javascript/src/jcs.mjs`, `hash.mjs`) and `tests/fixtures/tlpx-0.2/jcs/golden.json`. That oracle is not a 0.2 decision engine. The 0.1 audit helper `canonicalJson` remains the 0.1 seal hasher.

Hashes use domain-separated, versioned preimages:

```text
intent_hash =
  SHA-256(UTF8("northstar:intent:v1\0") || JCS(intent))

authorized_action_hash =
  SHA-256(UTF8("northstar:authorized-action:v1\0") || JCS(action))

executed_action_hash =
  SHA-256(UTF8("northstar:executed-action:v1\0") || JCS(action))
```

The normative string form is ASCII `sha256:` followed by exactly 64 lowercase hexadecimal characters, with no whitespace, truncation, uppercase, base64 alternative, or `0x` prefix. Schema pattern: `^sha256:[0-9a-f]{64}$`. Implementations validate this representation before embedding a hash string in another canonical structure.

Conformance must include cross-language golden fixtures for canonical bytes, raw 32-byte digests (`digest_hex`), exact `sha256:` strings, Unicode, escaping, nested objects, empty values, absent versus `null`, integer boundaries, UTF-16 key order (including astral vs BMP), lone-surrogate rejection, and rejected duplicate keys/floats. Those fixtures exist under `tests/fixtures/tlpx-0.2/jcs/`.

Changing any material submitted-intent field—including requester, proposed executor, action, intent class, target, arguments, environment, tenant, declared risk, data classes, capability, resource scope, payload/artifact digest, or adapter identity—requires a new evaluation and authorization. Every newly issued authorization independently receives a new authority-generated nonce.

Executable artifacts and transferred payloads must be authorized by digest, not only by path, filename, URL, or mutable reference.

## 7. Authorization record

An authorization must bind at least:

```json
{
  "authorization_id": "authz_...",
  "receipt_id": "rcpt_...",
  "requesting_principal": "agent.a",
  "executing_principal": "agent.a",
  "action": "send_email",
  "target": "customer:123",
  "authorized_action_hash": "sha256:...",
  "intent_hash": "sha256:...",
  "environment": "production",
  "tenant": "tenant_abc",
  "issued_at": "2026-08-11T12:00:00.000Z",
  "claim_expires_at": "2026-08-11T12:00:05.000Z",
  "execution_lease_seconds": 30,
  "authorization_nonce": "authority-generated-csprng-value",
  "idempotency_key": "...",
  "state": "AUTHORIZED_UNCLAIMED"
}
```

A later portable **authorization claim ticket** or **capability proof**, if issued, must additionally identify the issuer, ticket type and version, signing algorithm, key ID, trust domain, and signature. It is not a bearer `AUTHORIZED` snapshot.

The authorization-signing key must be separate from audit sealing, service identity, operator authentication, and tenant keys.

The requester may supply only an authenticated-principal-scoped `request_id` for correlation/idempotency. Northstar generates authorization IDs, authorization nonces, and claim IDs. `(authenticated principal, request_id)` is write-once after a terminal decision/error commits. Exact duplicates return the stored result; changed available intent hashes block with `IDEMPOTENCY_CONFLICT`; every retry uses a new request id. Authentication mismatch can occupy only the authenticated caller's slot, never the proposed identity's slot. Duplicate caller request IDs cannot collide globally.

## 8. Authorization lifecycle

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

### 8.1 State invariants

- Only `AUTHORIZED_UNCLAIMED` may transition to `CLAIMED`.
- Claiming consumes the authorization immediately.
- `CLAIMED` may never return to `AUTHORIZED_UNCLAIMED`.
- `DENIED`, `REJECTED`, `APPROVAL_EXPIRED`, `EXPIRED`, `REVOKED`, `COMPLETED`, `FAILED`, `CANCELLED`, `LEASE_EXPIRED`, `COMPLETED_CONFIRMED`, `FAILED_CONFIRMED`, and `OUTCOME_UNKNOWN_FINAL` are terminal.
- One authorization may produce at most one successful claim.
- One receipt may have at most one terminal human outcome.
- One one-time authorization may produce at most one terminal execution.
- Completion, failure, cancellation, rejection, or expiration permanently closes permission.
- Approval after `APPROVAL_EXPIRED` or `CANCELLED` requires a new evaluation.
- `EXECUTION_OUTCOME_UNKNOWN` permanently consumes authorization and must never trigger automatic re-execution.

### 8.2 Authorized cancellation

While `PENDING_APPROVAL`, cancellation may be initiated only by the authenticated original requester for its own request, an authenticated operator authorized for the approval route, an authenticated revocation/emergency authority, or Northstar itself for expiry, shutdown, invalidated policy, disabled principal, or revoked scope.

Cancellation and approval race atomically; exactly one transition wins. The cancellation record binds the receipt, action hash, authenticated canceller, role, stable reason, timestamp, and trusted sequence. Unauthenticated cancellation is always rejected.

After claim, cancellation is a request to stop work rather than proof that an external effect was reversed. Execution evidence distinguishes `CANCELLED_BEFORE_SIDE_EFFECT`, `CANCELLATION_REQUESTED`, `CANCELLED_DURING_EXECUTION`, `CANCELLATION_UNSUPPORTED`, and `COMPLETED_BEFORE_CANCELLATION`.

## 9. Timing and expiration

Authorization lifetime and execution lifetime are separate.

| Stage | Initial default |
| --- | ---: |
| Human approval request | 5-15 minutes |
| Automatic authorization claim window | 3-5 seconds |
| Post-approval claim window | 5 seconds |
| Local clock-skew tolerance | approximately 1 second |
| Execution lease | action-specific |

Example execution leases:

| Action | Initial lease |
| --- | ---: |
| Modify one file | 10 seconds |
| Send email or make API request | 30 seconds |
| Start deployment | 60 seconds |
| Complete deployment workflow | 5-15 minutes |
| Start long-running job | authorize job creation only |

The short claim timer begins when authorization is issued, not while a human is deciding.

## 10. Atomic claim

Expiration alone does not prevent two executors from using the same permission during the valid window. Claiming must therefore be transactional and compare the authenticated executor and the **Action Binding** digest. The authorization row MUST store an authority-computed `action_binding_hash` at issuance. It MUST NOT compare `authorized_action_hash` to `executed_action_hash`.

Conceptual operation:

```sql
UPDATE authorizations
SET
  state = 'CLAIMED',
  claimed_by = :authenticated_executor,
  claimed_at = :now,
  lease_expires_at = :lease_expiration
WHERE authorization_id = :authorization_id
  AND executing_principal = :authenticated_executor
  AND action_binding_hash = :presented_binding_hash
  AND state = 'AUTHORIZED_UNCLAIMED'
  AND claim_expires_at > :now
  AND revoked_at IS NULL
  AND principal_status = 'ACTIVE'
  AND policy_status = 'ACTIVE'
  AND signing_key_status = 'ACTIVE'
  AND environment_status = 'ACTIVE';
```

`:presented_binding_hash` is the PEP-computed digest of the Executed Action / Action Binding under `northstar:executed-action:v1`. It MUST equal the stored `action_binding_hash`. The stored digest is authority-computed from the validated Authorized Action at issuance; a requester-supplied value MUST NOT be accepted.

Capability and resource-scope checks MUST run before this update. If they fail, the row MUST remain `AUTHORIZED_UNCLAIMED`.

Exactly one updated row means the claim succeeded. Zero updated rows must produce a blocking reason such as:

- `AUTHORIZATION_EXPIRED`;
- `ALREADY_CLAIMED`;
- `EXECUTOR_MISMATCH`;
- `ACTION_MISMATCH`;
- `AUTHORIZATION_DENIED`;
- `AUTHORIZATION_TERMINAL`.

Authorization validation, revocation checks, identity/action comparison, and consumption occur in the same transaction. There is no separate check-then-claim gap.

## 11. Idempotency and retries

One-time authorization must not cause duplicate actions when a process loses a response.

- Execution requests carry an idempotency key bound to the authorization ID, authenticated executor, and authorized-action hash.
- Retrying the identical request returns the existing execution state or result.
- A retry must not repeat the side effect.
- A different action hash or executor is rejected.
- Failed or completed authorization cannot be reopened.
- A genuinely new attempt requires a new evaluation and authorization.

Idempotency permits recovery; it does not make authorization reusable.

## 11.1 Portable proof versus transactional consumption

For Northstar’s ordinary software threat model:

> Authorization proof may be portable; single-use consumption remains online and transactional.

A PEP may verify a claim ticket’s signature, authenticated executor binding, `authorized_action_hash`, adapter/environment/tenant scope, and expiry locally. It must still atomically claim the authorization from authoritative state before any side effect begins. Offline verification cannot establish global non-consumption.

A valid future claim ticket binds:

- one authenticated executor;
- one `authorized_action_hash`;
- one adapter, environment, and tenant;
- one short claim window;
- one atomic server-side consumption record.

Exotic hardware-backed non-copyable capabilities could change the consumption story someday. They must not complicate this design.

### Abandoned: `AUTHORIZED` snapshot token

The August 7 `tlpx.authz_token` design in `docs/roadmap/phase-b-authz-tokens.md` is **abandoned**, not deferred. Do not implement or revive:

- a bearer token with a minutes-long TTL and no proof-of-possession;
- a single `actor` field instead of requester/executor split;
- binding only `intent_hash`;
- treating offline signature verification as sufficient permission;
- listing one-time use as optional because it is harder offline.

## 12. Multi-agent isolation and handoff

### 12.1 Invalid handoff

```text
Agent A receives authorization
Agent A forwards it to Agent B
Agent B attempts execution
```

This must fail with `EXECUTOR_MISMATCH`.

### 12.2 Valid handoff

Northstar must evaluate the handoff while explicitly naming:

- Agent A as requester;
- Agent B as executor;
- the exact action Agent B will perform;
- the target and normalized arguments.

The resulting authorization names Agent B and may be claimed only by Agent B. Alternatively, Agent A may be authorized only to request a second evaluation for Agent B. A parent authorization never automatically authorizes a child action.

Handoff evidence should link parent and child receipt IDs without making permission transitive.

### 12.3 Authorization-to-execution integrity

Northstar must keep the following digests distinct:

- `intent_hash`: the canonical declaration submitted for evaluation;
- `authorized_action_hash`: the exact action approved by Northstar;
- `executed_action_hash`: the exact normalized operation presented to the PEP;
- optional `result_hash`: a bounded result, artifact, or external confirmation digest.

Before any side effect begins, the PEP must compare **Action Bindings**, not the full-object hashes:

```text
JCS(action_binding(authorized)) == JCS(action_binding(executed))
```

Those bindings are hashed under `northstar:executed-action:v1` to produce `executed_action_hash`. `authorized_action_hash` remains the digest of the full Authorized Action under `northstar:authorized-action:v1` and MUST NOT be compared to `executed_action_hash`. A binding mismatch blocks and records `ACTION_MISMATCH`. An `AUTHORIZED` record is never evidence that execution occurred. An execution attempt is never evidence that the intended external effect completed successfully.

For external systems, the receipt must distinguish observations such as request accepted, API response received, transaction committed, and independently confirmed settlement. Northstar must not claim a stronger result than the evidence supports.

### 12.4 Normative execution receipt

Every claimed authorization must reach one durable terminal execution receipt containing at least:

- execution and claim IDs;
- authorization and decision receipt IDs;
- authenticated requester and executor;
- intent, authorized-action, and executed-action hashes;
- protected target reference;
- policy bundle identity, version, and digest;
- adapter identity/version, authenticated adapter principal, and verified binary/deployment digest for adapter-started work; pre-adapter `LEASE_EXPIRED` evidence carries explicit null provenance;
- trusted sequence number;
- start and terminal timestamps;
- terminal state;
- bounded result summary, digest, or external evidence reference;
- audit-chain integrity and seal/signature metadata.

Receipts prove what the trusted enforcement path observed. They do not prove hidden intent, internal model alignment, or unobserved effects outside the mediated boundary.

If the PEP crashes after the protected system may have accepted the side effect but before a durable completion receipt exists, reconciliation assigns `EXECUTION_OUTCOME_UNKNOWN` and then `RECONCILIATION_REQUIRED`. Authorization remains consumed. The reconciler uses external idempotency keys, transaction/deployment IDs, artifact or target-state verification, protected-system confirmation, or human investigation. It may resolve to `COMPLETED_CONFIRMED`, `FAILED_CONFIRMED`, or the honest terminal `OUTCOME_UNKNOWN_FINAL`; it must never automatically repeat an irreversible action.

### 12.5 Trusted time, ordering, and replay

- Monotonic time controls local claim expiration and execution leases.
- Wall-clock time is retained for human-readable and cross-system evidence.
- One authority-wide transactional sequence allocator orders decisions, evaluation errors, claims, and later execution evidence. Per-table counters are not interchangeable with this sequence.
- Timestamps alone must not resolve concurrent state transitions.
- Portable authorization profiles must define clock-skew and uncertainty limits.
- Excessive time uncertainty fails closed.
- Nonces, one-time state, and idempotency keys prevent replay; timestamps are not the sole replay defense.

### 12.6 Revocation and emergency stop

Northstar must support revocation of an unclaimed authorization, principal, policy bundle, signing key, tenant, environment, or protected capability. Revocation is checked transactionally during claim.

Required behavior:

- unclaimed authorization becomes unusable immediately;
- claimed work may be cancelled only when the executor supports safe cancellation;
- completed work cannot be undone by changing history;
- long-running leases may require periodic validity checks;
- emergency deny has the highest policy precedence;
- revocation creates compensating evidence and never rewrites prior audit records.

The historical revocation-storm result is prototype evidence only. It is not a current capacity target or production scalability claim.

### 12.7 Policy provenance and precedence

Every active policy bundle must declare:

- unique ID and semantic version;
- content digest;
- issuer or owner;
- activation and retirement times;
- environment and tenant scope;
- precedence;
- superseded-policy reference where applicable.

Initial deterministic precedence is:

```text
emergency deny
  -> tenant/environment restriction
  -> Switchboard principal and action scope
  -> base policy
  -> action-specific policy
  -> human approval condition
```

Undefined conflicts, ambiguous precedence, or invalid policy provenance fail closed. Decision receipts record the complete effective policy-bundle digest.

Slice 2.4 makes this executable as a language-neutral contract:

- one schema-valid manifest binds bundle identity/version, authenticated issuer, exact content type/hash, active interval, exact tenant/environment, the fixed precedence order, explicit default, and optional predecessor ID/version/hash;
- `policy_bundle_hash` is JCS plus `northstar:policy-bundle:v1\0` over that complete manifest;
- one exact tenant/environment/time scope selects one active bundle; overlap is legal only when one acyclic explicit supersession chain names a unique later-activated same-scope winner and binds every predecessor hash;
- semantic version numbers never decide precedence implicitly;
- stages are monotone: later `ALLOW` cannot weaken earlier `REQUIRE_APPROVAL` or `DENY`; and
- absence, invalid provenance, ambiguous overlap, or invalid stage outcomes block without authorization.

The complete-stream ordering oracle starts at sequence 1 by default. Verification of a bounded complete segment must receive its authenticated expected starting sequence; it must not infer that start from the first event and thereby hide a missing prefix. Partial exports remain separately labeled and may not use the complete-stream claim.

The Phase 2 JavaScript helper is an oracle for these rules, not the production authority. At the accepted `aed80e2` and named `c9bdd0f` baselines, Rust still receives a trusted format-valid policy hash. Local 3.1 commit `1addb5c` replaces that input with a validated policy catalog and performs exact-scope/trusted-time manifest selection inside the authority; its full Section 3 verification remains deferred.

### 12.8 Key separation and lifecycle

Cryptographic authority must be separated across:

- audit sealing;
- authorization signing;
- service identity;
- operator authentication;
- tenant-specific trust domains.

The design must specify secure storage, least-privilege access, rotation, revocation, key versioning, offline verification, tenant isolation, and unavailable-key behavior. Compromise of an audit-sealing key must not grant authorization-signing authority.

### 12.9 Anti-gaming and bypass taxonomy

The threat model must classify at least:

- under-declared risk or omitted action data;
- benign-action substitution after approval;
- argument, target, payload, or artifact mutation;
- requester, executor, operator, or adapter substitution;
- approval, token, or receipt replay;
- split-action and batch-action evasion;
- time-of-check/time-of-use mutation;
- approval flooding, fatigue, and rubber-stamping;
- adapter bypass or direct tool access;
- partial execution before authorization;
- unmediated downstream effects triggered by an authorized first step.

Each category must be labeled as prevented, detected, evidenced, or an accepted limitation. Ambiguous or incomplete executable action data must not be treated as low risk.

### 12.10 Adapter integrity

Adapters are part of the trusted enforcement path when they translate host operations into canonical actions. They must:

- authenticate mutually with the authority;
- identify their version in requests and receipts;
- transmit envelopes over authenticated channels;
- map every material host argument and target deterministically;
- block if translation is incomplete or unsupported;
- prohibit caller-selected downgrade to unmediated execution;
- support binary or deployment digest verification where practical;
- undergo mutation, omission, and mapping tests.

A correct authority behind a compromised or incomplete adapter does not provide reliable enforcement.

The bounded local slice 3.7 candidate uses kernel-derived Unix peer credentials in both directions: the authority authenticates the adapter role and the adapter-side trust mapping authenticates the authority role. Startup requires one activated contract that fixes both principals, adapter id/version, canonical binary digest, capability/action coverage, and the exact ordered material-field projection. Execution start re-hashes the presented Executed Action, matches the consumed claim, and verifies that contract before durable start. This is a local contract/integrity boundary, not proof that the measured executable is isolated from replacement or that callers cannot reach the protected tool around it; those are later PEP/OS-enforcement gates.

### 12.11 Human approval quality and quorum extension

Approval interfaces must display the exact action, executor, target, risk, and material arguments represented by the action hash. Any material change invalidates the approval.

The operator-action record must contain decision receipt ID, authenticated operator subject, effective policy-bundle digest, trusted sequence, outcome, and authenticated-session/signature evidence. `APPROVE` and `REJECT` additionally contain the `authorized_action_hash` the human saw, approval route/quorum rule, and renderer identity/version. `CANCEL` does not invent renderer evidence when no approval representation was shown.

For what-you-see-is-what-you-sign binding:

```text
approval_context_hash =
  SHA-256(
    UTF8("northstar:approval-context:v1\0") ||
    JCS({
      authorized_action_hash,
      policy_bundle_hash,
      approval_route,
      material_display_fields,
      renderer_id,
      renderer_version
    })
  )
```

High-risk profiles may require MFA, rationale capture, rate limits, anti-fatigue controls, or M-of-N approval. Partial approval must produce a newly scoped action envelope and authorization; it may not mutate an existing authorization in place.

M-of-N sentinels are an extension profile, not a requirement for the first hardened path. The state model must allow them later without weakening the single-action invariant.

### 12.12 Privacy and data minimization

- Prism, authorization, and audit records remain metadata-oriented.
- Prompts, chain-of-thought, credentials, secrets, and unrestricted sensitive payloads are prohibited.
- Sensitive arguments should use digests, bounded summaries, or opaque references where possible.
- Audit retention and access are deployment-defined and separately authorized.
- Corrections and deletion obligations use tombstones or compensating records rather than rewriting authorization history.

### 12.13 Operational safety and incident response

Readiness must depend on valid policy, available keys, transactional state, and a healthy audit outbox—not only process liveness. The system requires:

- structured security events and stable reason codes;
- alerts for replay, identity mismatch, action mismatch, repeated claim failure, and suspected bypass;
- emergency deny and operational kill switch;
- non-permissive degraded mode;
- backup, restore, database integrity, and audit reconciliation procedures;
- recovery that never reopens expired, claimed, or terminal authorization;
- severity and response playbooks for trust-boundary failures.

### 12.14 Deferred Glass extension profiles

The following remain optional future profiles rather than initial-core promises:

- M-of-N sentinel consensus;
- anomaly and behavioral-drift scoring;
- hybrid human/machine consensus;
- decentralized attestations and provenance networks;
- zero-knowledge proofs of policy alignment;
- dual-ledger deployments;
- dynamic integrity seals;
- hierarchical multi-agent authorization.

Extensions may add evidence or stricter authorization but must never bypass deterministic policy, exact-action binding, single-use claim, authenticated execution, or forced mediation.

### 12.15 Requirements maturity and claims labels

Each major requirement or feature must be labeled as one of:

- **IMPLEMENTED**: present and builder-verified against a named commit;
- **PLANNED**: accepted for the build but without a named, builder-verified implementation;
- **EXTENSION_EXPERIMENTAL**: future research or optional profile.

These labels do not encode acceptance. Builder verification and independent acceptance must be reported separately with the exact commit. An uncommitted implementation candidate remains `PLANNED` in the maturity table.

Patent and product language must not imply that Northstar observes hidden reasoning, guarantees truthful intent, universally contains autonomous processes, proves internal alignment, or prevents bypass outside capabilities actually placed behind its PEP.

## 13. Strict policy requirements

Policy processing is divided into parse, validate, compile, and evaluate stages. The entire policy pack is rejected if any rule is invalid.

Reject at least:

- malformed expressions;
- unsupported syntax or operators;
- unknown fields;
- missing or duplicate rule IDs;
- missing conditions;
- rules without an effect;
- invalid enums and values;
- ambiguous or unsupported structure;
- empty policy packs unless an explicit default is configured.

Evaluation must distinguish matched, not matched, and error. An error never behaves like a non-match when the eventual default could be permissive.

Policy must be validated at process startup. An activated bundle must be able to construct every authorization it can grant. Duplicate authorization scopes, incomplete `ALLOW` templates, and policy capabilities absent from the active capability registry fail activation with `POLICY_COMPILE_FAILED`. A malformed or internally inconsistent pack fails process start. The previously loaded policy must not remain active accidentally unless an explicitly designed, audited last-known-good mode exists. Phase 1 has no such mode: startup failure is required. Later hot reload must fully compile and cross-validate the replacement, then atomically activate it; never partially load it.

Untrusted callers may not select arbitrary policy files per request. Each decision records policy identity, version, and digest.

### 13.1 Explicit defaults and version discipline

| Version | Default behavior |
| --- | --- |
| TL-PX 0.1 / Phase 1 | Malformed policy fails pack load or process startup and issues no authorization. It must not emit a v0.2 `EVALUATION_ERROR`. A **valid** pack retains the frozen v0.1 evaluate rule: no matching escalation rule implies `ALLOW`. |
| TL-PX 0.2 | Every policy bundle requires an explicit default outcome. Missing or invalid default prevents bundle activation. If that state is reached at runtime, emit `EVALUATION_ERROR` and issue no authorization. |

An explicit `default: ALLOW` remains legal in the minimum standard after Switchboard has authenticated and scoped the principal. The defect is implicit allowance combined with optional or bypassed scope enforcement.

Hardened profiles MAY require `default: DENY` or `default: REQUIRE_APPROVAL`. The minimum standard MUST NOT assume every deployment has that posture.

## 14. Forced mediation and initial PEP

The first protected surface will be a shell-command policy enforcement point, provisionally named `tlpx-run`.

The PEP must:

- authenticate the executor;
- require the complete authority-issued Authorized Action;
- verify its exact `authorized_action_hash` and compare the resulting Executed Action against it;
- atomically claim one authorization;
- execute normalized commands and arguments without shell interpolation by default;
- restrict executable, working directory, environment, output, and duration;
- enforce the execution lease;
- record a terminal result;
- prevent secrets and unrestricted output from entering the audit.

The protected test agent must not retain another route to the underlying shell. If it retains direct authority, Northstar is cooperative mediation rather than an enforcement boundary.

### 14.1 Restricted-agent acceptance (slice 3.9)

“The PEP identity is the only writer” is necessary and not sufficient. A caller that merely chooses to use the PEP is the already-proven cooperative gate.

The acceptance environment must use separate OS identities for the restricted agent and the PEP. Separate UIDs are necessary. Socket access alone is not impersonation: the agent must be able to contact the PEP to request authorized work.

The test must establish all of the following:

1. The agent identity cannot write the protected target.
2. The agent cannot acquire or impersonate the PEP identity: it cannot read PEP keys, modify PEP code or configuration, or bind a replacement service.
3. The agent UID may connect to the PEP only through a narrowly permitted interface.
4. The PEP service UID owns the protected capability and secrets.
5. The PEP authenticates the caller independently rather than trusting a caller-supplied principal string.
6. The PEP accepts only authenticated, exact, unexpired, unclaimed authorization and cannot be induced to execute an unclaimed or mismatched action.
7. Reuse, mutation, direct access, alternate binaries, alternate paths, and alternate privileged interfaces all fail.
8. The resulting execution and denial evidence is independently verified.

If the suite runs the agent as the PEP service identity, or lets the agent replace the PEP endpoint, read PEP keys, or write the protected target directly, it has not tested impersonation resistance. Permission to connect to the narrowly scoped PEP interface does not by itself violate identity separation.

## 15. Authentication

Northstar derives requester, operator, and executor identity from authenticated context, not request-body strings.

### 15.1 Initial local profile

- Unix domain socket;
- OS peer credentials where supported;
- dedicated Northstar service identity;
- explicit OS-user or workload-to-principal mapping.

### 15.2 Service profile

- TLS;
- mutual TLS, OIDC, or workload identity;
- verified human identity claims;
- separate evaluator, operator, and executor credentials;
- tenant-, environment-, role-, route-, and action-aware authorization.

Credentials and access tokens must never be written to audit records.

## 16. Transactional state and audit

SQLite is the initial operational state store for the local authority. Sealed JSONL remains an evidence and interoperability format, not the unsynchronized live state machine.

Database constraints must enforce:

- unique receipt IDs;
- unique authorization IDs and nonces;
- one decision per receipt;
- at most one terminal operator outcome;
- at most one successful claim;
- at most one execution for a one-time capability;
- no reopening of terminal states;
- exact principal and action-hash matching;
- idempotent duplicate requests.

Audit export must use a durable outbox or equivalent recovery mechanism so state commits and evidence remain reconcilable after a crash.

## 17. Fail-closed requirements

No permission is issued or exercised when Northstar encounters:

- invalid or unavailable policy;
- unknown or unauthenticated principal;
- missing or ambiguous action data;
- principal or executor mismatch;
- action-hash mismatch;
- expired or consumed authorization;
- database contention or failed transaction;
- audit integrity failure;
- authentication failure;
- excessive clock uncertainty;
- internal panic, unavailable dependency, or timeout.

Failures should be observable and auditable without exposing secrets.

## 18. Build phases

### Target contract version

The hardened contract targets **TL-PX 0.2.0**. The new decision/error records, authorization states, canonical hashing, one-time claims, revocation, authenticated identity binding, and execution receipts are breaking changes under the v0.1 SemVer rule.

TL-PX 0.1 remains frozen as historical evidence. Its 47 existing conformance fixtures must not be silently rewritten. Phase 2 creates `docs/standard/SPEC-v0.2.md`, `schemas/tlpx-0.2/`, and a distinct v0.2 conformance suite. Compatibility work must state whether a v0.1 artifact is read-only, explicitly upgraded, or unsupported; it must not pretend v0.1 already meant v0.2.

### Phase 1: strict fail-closed policy

First correct the JavaScript/TypeScript reference: build the strict policy compiler and negative suite, reject the known malformed condition `risk ==`, and issue no authorization. This phase establishes fixtures and reference semantics; it does not expand the reference into the production authority.

Live scope is slices 1.1–1.2 only:

```text
parse → validate → compile → evaluate
```

- The entire pack is compiled before any decision.
- Parse/compile errors are not treated as a non-match.
- A malformed pack fails process startup. No accidental last-known-good remains active.
- Negative tests sit beside the frozen 47 v0.1 fixtures and do not rewrite them.
- This phase does not emit `DENY` or `EVALUATION_ERROR` records, introduce v0.2 schemas, start Rust, issue tokens, or build a PEP.

### Phase 2: one normative contract

Align the v0.2 specification, schemas, validators, error records, reason codes, corrected reference implementation, planned Rust authority, and new conformance suite around `ALLOW`, `REQUIRE_APPROVAL`, and `DENY` plus the authorization lifecycle in this document.

Every emitted record, including Switchboard `DENY`, authorization claim, revocation, and execution receipt, must eventually validate against the canonical schemas. Accepted 2.3 conformance establishes this only for the object and evaluation/authorization record core. Local slice 3.6 adds a schema-valid sealed terminal execution emitter and dedicated contract cases, but it remains an unaccepted candidate pending full Section 3 and independent verification. No portable revocation emitter may claim 0.2 conformance until that contract closes. Local slice 2.4 commit `a87f822` adds policy-manifest/hash, deterministic selection/precedence, complete-stream sequence, and maturity-label oracles; it remains unaccepted until the deferred exact-commit/full Section 3 verification and independent review are recorded.

### Phase 3: authenticated transactional authority and one-time execution

Complete the Rust authority around distinct Submitted Intent, Authorized Action, Executed Action, and Execution Receipt types and hashes, authenticated requester/operator/executor identities, SQLite state, short claim window, atomic one-time claim, revocation check, idempotency, and adapter integrity. The local library MVP supplies part of this path, but trusted embedding strings are not acceptance of authenticated identity and an atomic claim is not an execution receipt.

Local slice 3.1 commit `1addb5c` makes the existing skeleton consume the 2.4 policy contract. Rust computes the manifest hash, verifies a reproducible JCS hash of its exact-match policy content, validates all configured supersession chains at startup, selects one active bundle by exact tenant/environment/trusted evaluation time, records missing or ambiguous selection as durable evaluation error, and rechecks activity at claim. The authority no longer accepts an arbitrary policy hash in `AuthorityConfig`. Manifest issuer identity is still trusted local configuration rather than an authenticated publisher assertion. Exact-commit/full Section 3 verification is deferred until Section 3 is complete.

Local slice 3.2 commit `e835c4e` makes the already-present distinct Rust action types and domain hashes independently reproducible across JavaScript and Rust. Shared fixtures pin schema-valid source objects, canonical strings, UTF-8 bytes, raw SHA-256 digests, domain-separated hashes, explicit nullable digests, optional retry omission, Unicode, and the exact Action Binding projection. JavaScript helpers validate the relevant schema before hashing; Rust tests reconstruct typed values and match the same fixtures. Every binding component is mutation-tested, while authority-only fields must alter the full Authorized Action hash without altering the binding. This is contract/type evidence only: it adds no authenticated transport, claim behavior, execution receipt, or side effect.

Local slice 3.3 commit `c19b1d2` adds a bounded Unix local-authentication profile. Rust reads peer UID/GID from a connected Unix-domain socket through safe OS APIs, resolves it through an exact process-owned mapping to an opaque principal and allowed roles, and exposes authenticated requester and executor facades. `REQUIRE_APPROVAL` rules bind their exact approval route into versioned policy content. The original requester, a mapped operator on that route, or a mapped emergency authority may atomically cancel a pending approval for its permitted reason; exactly one cancellation wins and commits one schema-valid sealed `tlpx.operator_action`. At that historical commit, raw principal-string methods remained a trusted-embedding seam; slice 3.8 later made the production mutation paths identity-only. Socket ownership/configuration remain deployment responsibilities. The full exact-commit builder matrix passed. This is not a hardened service, approval resolution, post-claim cancellation, acceptance, or a PEP.

Local slice 3.4 commit `133cd94` makes `REQUIRE_APPROVAL` bind an authorization template as well as its route. The authority stores canonical renderable Authorized Action content and hashes until an authenticated human route member supplies the displayed hash and renderer identity/version. `APPROVE` atomically emits operator evidence and a newly issued authorization with authority-generated ID/nonce; `REJECT` emits operator evidence and no authorization. Approval expiry, approve, reject, and cancellation are one-winner transitions. The short claim timer begins only at issuance. The full exact-commit builder matrix passed. Expiry is durable authority state with a trusted sequence, but the accepted operator-action schema has no expiry outcome, so no private portable expiry record is claimed.

Local slice 3.5 commit `ad95653` adds authority-local immutable revocation rows for authorization, principal, policy-bundle hash, signing-key placeholder, tenant, environment, and capability scopes. The authenticated emergency-authority facade is required; authorization, requester/executor principal, policy, tenant, environment, and capability revocations are checked inside the same SQLite write transaction as claim. Revocation persists across restart, duplicates cannot rewrite history, and authorization revocation versus claim has one winner across processes. The full exact-commit builder matrix passed. No portable or sealed `tlpx.revocation` record is claimed, and signing-key enforcement remains deferred until the authority actually has separated authorization-signing keys.

Local slice 3.6 commit `9028346` makes `tlpx.execution` a terminal receipt and keeps `EXECUTION_OUTCOME_UNKNOWN` plus `RECONCILIATION_REQUIRED` as durable authority-process states. Execution idempotency binds authorization ID, authenticated executor, and Authorized Action hash. Exact retries return existing state; changed retries fail. Slice 3.8 later separates a fresh `Started` result from the non-permission retry result so callers cannot treat an existing attempt as spawn authority. Direct completion/failure/cancellation, unstarted lease expiry, and authenticated reconciliation all converge on one terminal SQLite row and one canonical hash-chained HMAC-sealed receipt in the same transaction. Unknown outcomes remain consumed and cannot be directly completed or replayed. The full exact-commit builder matrix passed. This boundary performs no side effect and is not forced mediation.

Local slice 3.7 commit `518899a` adds an activated adapter contract fixing adapter id/version, authenticated adapter and authority principals, canonical binary digest, capability/action coverage, and the exact ordered material-field projection. The local Unix channel authenticates both peers from kernel credentials. Execution start re-hashes the Executed Action and verifies the consumed claim plus the adapter contract before durable start; terminal evidence binds adapter principal and digest. Missing/reordered mapping, peer substitution, digest/version mismatch, incomplete coverage, and action mutation fail closed. The full exact-commit builder matrix passed. This is not independent executable measurement, a protected side effect, or forced mediation.

Local slice 3.8 commit `7c41450` adds a bounded `CooperativeShellRunner` and deliberately named `tlpx-run-demo`. It verifies the complete authority-issued Authorized Action, exact command-plan shape, canonical executable plus a check-before-spawn digest comparison, working directory, environment names, output and duration bounds, execution lease, authenticated executor, adapter session, one-time claim, and a fresh durable `STARTED` transition before direct argv spawn. It clears inherited environment, supplies null stdin, launches a new process group, bounds/drains output without copying content into audit, kills the group on timeout or terminal cleanup, and records direct completion/failure; a terminal-commit failure attempts unknown-outcome state. Unknown JavaScript hash domains now fail closed, raw principal-string evaluate/claim methods are private authority internals, and execution-start retries return a distinct non-permission outcome. Same-UID socket-pair tests cover mapping/contract logic only. The demo creates one new marker under the caller UID and reserves `tlpx-run` for 3.9. It is not separate-identity or alternate-route-resistant, its argv is not an operand sandbox, and its executable digest has a check-to-exec replacement window. No cooperative-runner error strings were added to the portable 0.2 reason-code catalog. The full exact-commit builder matrix is recorded in `tests/reports/slice-3.8-cooperative-shell-runner-builder-verification-2026-08-20.md`; independent acceptance is deferred.

Local slice 3.9 candidate commit `e6f2bb0` adds a narrowly configured `tlpx-run` Unix service for one `CREATE_MARKER` operation. It authenticates peer UID/GID from the kernel, constructs the complete request and authorization inside the PEP, refuses caller-supplied principal/path/argv material, requires one fresh claim and durable start, and emits evidence checked by Rust integrity reconciliation plus the JavaScript schema/linkage oracle. The acceptance harness is designed to prove direct-write failure, key/config/code/endpoint protection, unrelated-identity denial, exact authorized execution, replay/mutation/restart/expiry closure, and evidence validity under distinct PEP and restricted-agent identities.

> **3.9 TEST REQUIRED:** The defining `sudo ./scripts/restricted-agent-acceptance.sh` run was not performed. Ordinary tests do not establish the §14.1 OS-enforcement claim. Slice 3.9 remains `PLANNED`, Section 3 remains incomplete, and neither may be reported as verified or accepted until that separate-identity run passes and is recorded.

An unauthenticated `tlpx-run` may be built earlier only as a prototype. It is not the enforcement acceptance test and must be labeled accordingly.

The `tlpx-run` candidate now exists at `e6f2bb0`, but slice 3.9 is the OS-enforced acceptance test in §14.1, not the presence of a service wrapper. A restricted test agent must be unable to create the protected marker except through one valid authorization and must be unable to reuse, mutate, or transfer that authorization. The required administrator-backed run remains outstanding.

### Phase 4: operational hardening and multi-agent profile

Complete separated key roles, durable audit export, reconciliation, operational readiness, concurrency safety, crash recovery, and explicit non-transitive multi-agent handoff.

Identity spoofing, replay, concurrent claims, retries, and restarts must not create a second authorized action.

Local slice 4.1 implementation commit `980327d`, exact-verified through `efa7f0f`, adds a role-bound key registry for audit sealing, authorization signing, service identity, operator authentication, and tenant trust. It rejects material reuse and role substitution, supports active-to-verify-only rotation, signs authority-local authorization state, selects historical audit keys by id, and makes durable key revocation fail closed. The exact-commit builder and focused red-team matrix is recorded in `tests/reports/slice-4.1-separated-key-roles-builder-verification-2026-08-27.md`. Key custody/KMS integration and independent review remain deferred.

Local slice 4.2 commit `833d8d4` adds a bounded append-only JCS file exporter for the sealed outbox. It validates the complete existing sink as an exact database prefix, appends and synchronizes each new row before internal acknowledgement, recovers a complete append left ahead of acknowledgement without duplication, rejects torn or altered history, and requires protected user-owned sink/directory/lock state. Reconciliation now rejects acknowledgement gaps and time rollback. The exact-commit builder and focused red-team matrix is recorded in `tests/reports/slice-4.2-durable-audit-export-builder-verification-2026-08-27.md`. This is local storage behavior, not remote transport, retention automation, monitoring, or an accepted portable envelope.

## 19. Assurance strategy

The trust path requires:

- unit and integration tests;
- schema and cross-language conformance;
- property-based testing;
- model-based state-machine testing;
- parser and API fuzzing;
- concurrency race tests;
- crash and recovery injection;
- replay and identity-substitution attacks;
- action and argument mutation tests;
- expiration and clock-boundary tests;
- database corruption and recovery tests;
- two-agent requester/gate scenarios;
- independent correctness, security, and conformance reviews.

No agent should author and approve its own trust-path change.

The primary safety property is:

> For every possible event sequence, no protected side effect begins unless one valid authorization for the authenticated executor and exact action is atomically claimed once; the executed action must match the authorized action, and every successful claim must reach one durable terminal outcome.

## 20. Defense in depth

Northstar is not the only control. Production-shaped deployments should also use:

- OS sandboxing and process isolation;
- least-privilege service accounts;
- restricted filesystem and network access;
- external credential brokering;
- rate limiting and resource bounds;
- bounded, monitored evidence storage with retention/export/backpressure controls, because unauthenticated errors and idempotency conflicts are append-only and attacker-triggerable;
- independent audit export and monitoring;
- operational kill switches and revocation controls.

If Northstar fails, these controls should still constrain the blast radius.

## 21. Claims discipline

Until independently reviewed and hardened, Northstar should be described as a reference implementation, experimental enforcement boundary, or security-focused preview.

Passing tests alone does not justify claims such as high assurance, tamper-proof, universally secure, or production safe.

Documentation must visibly distinguish implemented behavior, planned requirements, and experimental extension profiles. Historical prototypes and performance artifacts may be cited only with their original scope and methodology limitations.

## 22. Required CI

Changes to the trust path must run:

- Rust unit, integration, property, and concurrency tests;
- TypeScript reference tests;
- conformance suite;
- technical test;
- red-team suite;
- JSON Schema validation;
- RFC 8785 canonical-byte and exact `sha256:` representation fixtures across languages;
- domain-separated intent/authorized/executed/approval-context hash fixtures;
- malformed-policy negative tests;
- `DENY` versus `EVALUATION_ERROR` retry-semantics tests;
- sealed evaluation-error evidence tests;
- approval-expiry and authorized-cancellation race tests;
- advisory declared-risk versus policy-derived-risk tests;
- one-time claim and replay tests;
- executor-binding and piggybacking tests;
- intent/authorized/executed hash consistency tests;
- action, target, payload, artifact, and adapter mutation tests;
- expiration and idempotency tests;
- revocation and emergency-deny tests;
- policy precedence and provenance tests;
- key rotation and revoked-key tests;
- trusted-time and sequence-ordering tests;
- execution-receipt schema and evidence tests;
- human approval display/hash-binding tests;
- crash-recovery tests;
- post-side-effect/pre-receipt crash and unknown-outcome reconciliation tests;
- audit-outbox reconciliation tests;
- two-agent execution-gate test;
- SDK tests where compatibility is claimed.

Major phases add dated evidence under `tests/reports/` without overwriting historical reports.

## 23. Delivery slices

| Slice | Deliverable | Dependency |
| --- | --- | --- |
| 1.1 | Strict policy parser and compiler | None |
| 1.2 | Negative policy suite and startup validation | 1.1 |
| 2.1 | Freeze v0.1 and draft normative TL-PX 0.2 decision/error/state contract | 1.1 |
| 2.2 | JCS profile, hash representation, domain separation, and golden fixtures | 2.1 |
| 2.3 | v0.2 object and evaluation/authorization record schema core, validators, reason codes, and conformance suite; execution-side evidence closure deferred by the 2026-08-14 clarification | 2.2 |
| 2.4 | Policy-bundle manifest/schema/hash, explicit supersession and monotone precedence, trusted complete-stream ordering oracle, and maturity labels | 2.3 |
| 3.1 | Rust authority skeleton/shared types plus native policy-manifest content binding and activation | 2.4 |
| 3.2 | Submitted Intent, Authorized Action, and Executed Action types plus distinct canonical hashing | 3.1 |
| 3.3 | Authenticated local requester, operator, executor, and cancellation | 3.2 |
| 3.4 | Authorization record, authority-generated nonce, and approval expiry | 3.3 |
| 3.5 | SQLite state, revocation, and atomic one-time claim | 3.4 |
| 3.6 | Idempotency, execution receipt, unknown-outcome reconciliation, and terminal state | 3.5 |
| 3.7 | Authenticated adapter contract and integrity tests | 3.6 |
| 3.8 | `tlpx-run` shell PEP prototype | 3.7 |
| 3.9 | Restricted-agent authenticated enforcement acceptance test | 3.8 |
| 4.1 | Separated key roles, rotation, and revocation | 3.5 |
| 4.2 | Durable audit outbox and reconciliation | 4.1 |
| 4.3 | Concurrency, trusted-time, cancellation-race, and crash-recovery suite | 4.2 |
| 4.4 | Operational readiness and incident-response profile | 4.2 |
| 4.5 | Explicit multi-agent handoff profile | 4.3 |
| 4.6 | External security review readiness package | 4.4 |

## 24. Completion criteria

The hardened core is complete when:

- invalid policy cannot authorize;
- all emitted records conform to one normative contract;
- TL-PX 0.1 remains frozen while 0.2 breaking changes are separately versioned and tested;
- canonical bytes and exact hash strings match across every conforming language;
- `DENY` and `EVALUATION_ERROR` have distinct, deterministic retry semantics;
- every evaluation error that authoritative storage can commit is sequenced and sealed without issuing authorization; storage failure still blocks and must not claim ghost evidence or occupancy;
- every authorization names one requester and one executor;
- every authorization binds one exact Authorized Action, payload, artifact, target, and adapter identity where applicable;
- intent, authorized-action, and executed-action hashes remain distinct and verifiably linked;
- authorization must be claimed within a short window;
- pending approval expires and may be cancelled only by an authenticated authorized actor or system authority;
- exactly one authenticated executor can claim it;
- completion, failure, cancellation, rejection, or expiration closes it permanently;
- agents cannot piggyback, transfer, mutate, or replay permission;
- retries cannot duplicate the action;
- requester-declared risk can never lower policy-derived effective risk;
- authority-generated authorization nonces cannot be collision-gamed by callers;
- revocation prevents every later unclaimed use;
- policy precedence and provenance are deterministic and evidenced;
- cryptographic roles are separated and safely rotatable;
- every successful claim reaches one durable execution receipt and terminal outcome;
- uncertain post-side-effect outcomes consume authorization and reconcile honestly without automatic replay;
- a restricted agent cannot bypass the protected PEP;
- compromised or incomplete adapters fail closed rather than silently downgrading mediation;
- operators and executors are authenticated;
- state remains correct through races, crashes, and restarts;
- degraded operation never becomes permissive;
- sealed evidence reconstructs each decision and terminal outcome;
- independent adversarial testing confirms these properties against a named commit.

## 25. Baseline evidence and related documents

- [`../tests/README.md`](../tests/README.md)
- [`../tests/reports/northstar-two-agent-test-proof.md`](../tests/reports/northstar-two-agent-test-proof.md)
- [`../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md`](../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md)
- [`../tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md`](../tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md)
- [`../tests/reports/slice-2.4-policy-ordering-builder-verification-2026-08-17.md`](../tests/reports/slice-2.4-policy-ordering-builder-verification-2026-08-17.md)
- [`../tests/reports/slice-3.1-rust-policy-activation-builder-verification-2026-08-17.md`](../tests/reports/slice-3.1-rust-policy-activation-builder-verification-2026-08-17.md)
- [`../tests/reports/slice-3.2-typed-action-hash-builder-verification-2026-08-17.md`](../tests/reports/slice-3.2-typed-action-hash-builder-verification-2026-08-17.md)
- [`../tests/reports/slice-3.3-local-auth-cancellation-builder-verification-2026-08-18.md`](../tests/reports/slice-3.3-local-auth-cancellation-builder-verification-2026-08-18.md)
- [`../tests/reports/slice-3.4-approval-expiry-builder-verification-2026-08-18.md`](../tests/reports/slice-3.4-approval-expiry-builder-verification-2026-08-18.md)
- [`../tests/reports/slice-3.5-transactional-revocation-builder-verification-2026-08-18.md`](../tests/reports/slice-3.5-transactional-revocation-builder-verification-2026-08-18.md)
- [`../tests/reports/slice-3.6-execution-reconciliation-builder-verification-2026-08-18.md`](../tests/reports/slice-3.6-execution-reconciliation-builder-verification-2026-08-18.md)
- [`../tests/reports/slice-3.7-authenticated-adapter-builder-verification-2026-08-18.md`](../tests/reports/slice-3.7-authenticated-adapter-builder-verification-2026-08-18.md)
- [`../tests/reports/slice-3.8-cooperative-shell-runner-builder-verification-2026-08-20.md`](../tests/reports/slice-3.8-cooperative-shell-runner-builder-verification-2026-08-20.md)
- [`reviews/build-spec-review-2026-08-11-model-2.md`](./reviews/build-spec-review-2026-08-11-model-2.md)
- [`reviews/build-plan-review-disposition-2026-08-13.md`](./reviews/build-plan-review-disposition-2026-08-13.md)
- [`standard/SPEC-v0.1.md`](./standard/SPEC-v0.1.md)
- [`security.md`](./security.md)
- [`architecture.md`](./architecture.md)
- [`roadmap/priorities.md`](./roadmap/priorities.md) — superseded as current direction
- [`roadmap/phase-a-pep.md`](./roadmap/phase-a-pep.md) — problem retained; sequence superseded
- [`roadmap/phase-b-authz-tokens.md`](./roadmap/phase-b-authz-tokens.md) — old snapshot token abandoned
- [`roadmap/phase-c-mm-handoff.md`](./roadmap/phase-c-mm-handoff.md) — handoff goal retained; must not use the abandoned token

This sheet is the current build baseline. Changes to its security invariants require an explicit architecture decision, corresponding contract updates, and adversarial tests.
