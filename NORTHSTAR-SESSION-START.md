# Northstar Fresh-Context Instructions

Use this file to begin or resume work on Project Northstar in a new model, chat, machine, or context window.

**Local repository:** `/Users/home/projects/northstar` or `~/projects/northstar`  
**Private GitHub:** `robkatzenberger/Project-Northstar`  
**Default branch:** `main`

## 1. How to work with Robert

Treat Robert as a project partner, not merely a requester or code-audit client.

The Human–Machine Trust Layer is reflected in how this project is built:

- Humans and machines may propose, question, disagree, and revise.
- Give fair, evidence-based pushback when a shortcut weakens trust or security.
- Listen seriously to Robert's reasoning and explain tradeoffs plainly.
- Do not manufacture consensus or hide minority concerns.
- Respect does not eliminate scrutiny, and trust does not replace verification.
- Plan and align before making architecture or security-sensitive changes.
- Be candid when an idea, claim, test, or implementation is weak.
- Prefer “no BS” evidence and adversarial testing before celebrating.
- Do not make repository changes unless Robert explicitly asks.
- Ask before committing, pushing, changing remotes, or performing destructive Git operations.

When several frontier models participate, give them distinct roles rather than asking identical questions and treating majority agreement as truth:

```text
Builder       -> proposes or implements
Correctness   -> checks logic and invariants
Security      -> attacks assumptions and boundaries
Architecture  -> checks long-term coherence
Conformance   -> compares code, schemas, and specification
Human partner -> evaluates evidence, purpose, and tradeoffs
```

No model should author and approve its own trust-path change. Preserve disagreements and evidence for Robert.

## 2. What Northstar is

Northstar is a pre-execution authorization boundary for human- and machine-initiated actions.

The stable architectural invariant is:

```text
Intent -> Verification -> Execution
```

- Prism describes intent; it does not judge or authorize.
- Switchboard authenticates and scopes the principal before policy.
- Deterministic policy evaluates the canonical action without an LLM in the allow/deny path.
- Humans approve defined escalations.
- The PEP executes only a valid, exact, one-time authorization.
- Transactional state controls live authority.
- Sealed audit and receipts preserve evidence.

Northstar is not an AI judge, general agent orchestrator, universal containment system, hidden-reasoning detector, proof of truthful intent, or automatic legal-liability engine.

## 3. Foundational security rule

> One authorization permits one authenticated executor to perform one exact action, one time.

Authorization is not an open line, transferable token, shared session, reusable approval, or permission for another agent.

Keep these events distinct:

```text
declared intent
  -> authorized action
  -> claimed execution
  -> observed terminal result
```

`AUTHORIZED` is not `EXECUTED`. The protected executor must prove that the operation presented for execution matches the exact authorized action before any side effect begins.

## 4. Who reads what

| Audience | Route |
| --- | --- |
| Human newcomer | `README.md` → `docs/SHARE.md` |
| Implementer | `README.md` → `docs/README.md` → the relevant guide |
| AI collaborator | this file, then only the docs the task needs |
| Private continuity | `skills.md` (gitignored; preferences and latest handoff only) |

**Document ownership**

| File | Owns |
| --- | --- |
| `docs/standard/SPEC-v0.2.md` | Normative protocol: records, states, hashing, schemas |
| `docs/BUILD-SPEC-SHEET.md` | Delivery sequence, acceptance, maturity |
| this file | Concise status, collaboration rules, reading routes |
| `tests/reports/` and `docs/reviews/` | Immutable point-in-time evidence |
| `skills.md` | Private preferences and latest operational handoff |

Do not treat `skills.md` or this file as a parallel specification. Do not expand architecture unless slice work or implementation evidence requires it.

**Load next, as needed:** `AGENTS.md`, `docs/standard/SPEC-v0.2.md`, `docs/BUILD-SPEC-SHEET.md`, `docs/standard/SPEC-v0.1.md` (frozen), `tests/reports/northstar-two-agent-test-proof.md`, `docs/security.md`, `implementations/javascript/AGENTS.md`.

## 5. Current status

**Accepted contract line:** slices 1.1–2.3, with the 2.3 acceptance scoped to the object plus decision/evaluation-error/operator-action/authorization/claim schema core. Slice 3.6 now supplies an unaccepted implementation-driven terminal execution-receipt contract candidate; it does not retroactively broaden 2.3 acceptance. Portable approval-expiry and revocation evidence remain deferred. Local slice 2.4 commit `a87f822` adds the policy-bundle manifest/hash, deterministic precedence/selection, trusted ordering, and maturity-label candidate. JS gate is TL-PX 0.1 + fail-closed compile + 0.2 schema/policy/typed-action/JCS/hash oracle. It still emits `standard_version: "0.1.0"`.

**2.3d accepted at the exact named commit:** `aed80e2527f05a3730b1057f2d90c55a6c3eb646` pins immutable authenticated idempotency, retry linkage, failure attribution, conditional operator evidence, and authority-wide sequence semantics. Rust `tlpx` matches the JCS golden fixtures and has a durable local authority MVP: exact-match policy, Switchboard checks, authority-generated IDs/nonces, SQLite decisions/errors/idempotency/revocation, and atomic exact-action claim. Builder evidence is in `tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md`; independent acceptance is in `tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md`.

**Named evidence/outbox commit:** `c9bdd0fcd2d4c51fda9f3861724db0fd97524003` serializes schema-valid canonical `tlpx.decision`, `tlpx.evaluation_error`, `tlpx.authorization`, and `tlpx.authorization_claim` records atomically with authority state into a separate hash-chained, HMAC-sealed SQLite outbox. Pending reads, ordered/idempotent export acknowledgement, restart persistence, and reconciliation are builder-verified in `tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md`. This commit is not independently accepted.

**Slice 2.4 local candidate:** commit `a87f82271a12843c120d9a1e6ee238957f285c2f` contains `policy-bundle.schema.json`, the `northstar:policy-bundle:v1\0` cross-language hash, explicit active-window/supersession selection, monotone precedence, complete-stream authority sequence verification, and exact maturity labels. This is a contract/oracle increment, not Rust policy activation, authenticated transport, or a PEP. Pre-commit builder evidence is in `tests/reports/slice-2.4-policy-ordering-builder-verification-2026-08-17.md`; exact-commit Section 3 verification is deferred, so the slice remains `PLANNED` and unaccepted.

**Slice 3.1 local candidate:** commit `1addb5c6a0ede31d754ac0bd47d7ef1f3a05e6d4` makes Rust parse and hash native manifests, verify a reproducible JCS hash of configured exact-match policy content, validate supersession at startup, select by exact tenant/environment/trusted evaluation time, durably record unavailable or ambiguous selection without guessed policy identity, and recheck policy activity at claim. `AuthorityConfig` no longer accepts a caller-supplied arbitrary policy hash. Pre-commit builder evidence is in `tests/reports/slice-3.1-rust-policy-activation-builder-verification-2026-08-17.md`; exact-commit/full Section 3 verification is deferred and the slice remains unaccepted. Manifest issuer assertion and caller principal strings are still trusted local-configuration/embedding inputs, not authenticated transport.

**Slice 3.2 local candidate:** commit `e835c4e` binds the existing Rust `SubmittedIntent`, `AuthorizedAction`, and `ExecutedAction` types to shared schema-valid cross-language fixtures. JavaScript and Rust agree on the exact canonical strings, UTF-8 bytes, distinct intent/authorized/executed hashes, required-null versus absent optional fields, and the nine-field Action Binding projection. The Authorized Action schema rejects `effective_risk` below `derived_risk`, matching Rust. Mutation tests cover every binding component and authority-only fields. The bounded exact-commit rerun passed; evidence and limits are in `tests/reports/slice-3.2-typed-action-hash-builder-verification-2026-08-17.md`. Full Section 3 verification, independent review, acceptance, authenticated execution, and a PEP remain open.

**Slice 3.3 local candidate:** commit `c19b1d2` maps kernel-derived Unix peer UID/GID to opaque local requester, operator, executor, and emergency-canceller roles. Authenticated requester/executor facades, policy-bound approval routes, and atomic pending cancellation fail closed and emit a schema-valid sealed `tlpx.operator_action`; a two-connection cancellation race has one winner. The full exact-commit builder matrix passed; evidence and limits are in `tests/reports/slice-3.3-local-auth-cancellation-builder-verification-2026-08-18.md`. Raw principal-string APIs remain a trusted embedding seam. This is not approval resolution, post-claim cancellation, a service, acceptance, or a PEP.

**Slice 3.4 local candidate:** commit `133cd94` requires human-only operators on the exact policy route to bind canonical Authorized Action content, the displayed hash, and renderer identity/version. `APPROVE` creates a fresh authority ID/nonce and starts the short claim window at approval time; `REJECT` creates no authorization. Expiry and terminal human outcomes race atomically. The full exact-commit builder matrix passed; evidence and limits are in `tests/reports/slice-3.4-approval-expiry-builder-verification-2026-08-18.md`. Expiry has durable state/sequence but no portable record because the accepted schema has no expiry outcome.

**Slice 3.5 local candidate:** commit `ad95653` adds authenticated authority-local revocation state for authorization, principal, policy hash, tenant, environment, and capability scopes. Claim checks those scopes inside the same SQLite write transaction as one-time consumption; authorization revocation versus claim has one winner across processes, and revocation survives restart. The full exact-commit builder matrix passed; evidence and limits are in `tests/reports/slice-3.5-transactional-revocation-builder-verification-2026-08-18.md`. There is still no portable or sealed `tlpx.revocation` record; later slice 4.1 now enforces authority-local authorization-signing-key revocation.

**Slice 3.6 local candidate:** commit `9028346` adds an idempotent durable execution lifecycle. The named executor starts exactly one attempt; direct or reconciled terminal state atomically emits one schema-valid, canonical, sealed `tlpx.execution` receipt with bounded result evidence. Unknown outcomes remain consumed, move through authenticated reconciliation, and never replay automatically. The full exact-commit builder matrix passed; evidence and limits are in `tests/reports/slice-3.6-execution-reconciliation-builder-verification-2026-08-18.md`. No protected side effect or forced mediation exists yet.

**Slice 3.7 local candidate:** commit `518899a` adds a mutually authenticated local Unix adapter session and an activated contract for both principals, adapter id/version, canonical binary digest, capability/action coverage, and the exact material-field projection. Execution start re-hashes the Executed Action and verifies the consumed claim plus this contract; adapter-started terminal receipts bind principal and digest. The full exact-commit builder matrix passed; evidence and limits are in `tests/reports/slice-3.7-authenticated-adapter-builder-verification-2026-08-18.md`. Executable measurement/configuration and process isolation remain deployment trust boundaries, and no PEP exists yet.

**Slice 3.8 local candidate:** commit `7c41450ff4e4bd22155b91149a2c0ef6366cf5f6` adds the bounded same-UID `CooperativeShellRunner` and deliberately named `tlpx-run-demo`. It binds an authority-issued action to an exact direct-argv plan, activated executable/digest, cwd, environment names, output/duration limits, authenticated claim, adapter session, and a fresh durable start before one cooperative marker side effect. Public authority mutation paths now require authenticated identities; execution-start retries are non-permission results; child process groups are terminated on timeout/cleanup; unknown JavaScript hash domains fail closed. The full exact-commit builder matrix passed; evidence and limits are in `tests/reports/slice-3.8-cooperative-shell-runner-builder-verification-2026-08-20.md`. Its digest check has a check-to-exec window, argv binding is not operand confinement, and callers with direct capability access can bypass it.

**Slice 3.9 unverified local candidate:** commit `e6f2bb0a511628dc94f619716c3682b506d5aaf4` adds a bounded `tlpx-run` Unix service, kernel peer-credential authentication, process-owned intent/authorization construction, one exact protected-marker operation, strict configuration/permission checks, Rust evidence reconciliation, a JavaScript schema/linkage validator, and a root-run separate-identity acceptance script. The ordinary pre-commit builder matrix passed, but the defining administrator-backed test was deliberately not run. The candidate therefore does not establish forced mediation, slice 3.9 completion, Section 3 completion, or acceptance. See `tests/reports/slice-3.9-restricted-pep-candidate-unverified-2026-08-27.md`.

**Slice 4.1 local candidate:** implementation commit `980327d`, exact-verified through `efa7f0f`, separates five cryptographic roles, rejects reused material and cross-role substitution, supports verify-only historical rotation, signs authority-local authorizations, and enforces durable key revocation. The full exact-commit builder/red-team matrix passed; evidence and limits are in `tests/reports/slice-4.1-separated-key-roles-builder-verification-2026-08-27.md`. Key custody/KMS integration remains deferred; Phase 4 independent review and acceptance are pending.

**Slice 4.2 local candidate:** commit `833d8d4` adds bounded append-and-sync-before-ack audit export, exact sealed-prefix validation, no-duplicate append-before-ack crash recovery, protected sink/lock checks, and fail-closed acknowledgement order/time reconciliation. The full exact-commit builder/red-team matrix passed; evidence and limits are in `tests/reports/slice-4.2-durable-audit-export-builder-verification-2026-08-27.md`. This is a local file profile, not external transport or independent acceptance.

**Slice 4.3 local candidate:** commit `eb0e624` adds a persistent nondecreasing trusted-time watermark, monotonic process time, exact claim/approval deadline edges, repeated claim/revocation and approval/cancellation races, and restart-through-reconciliation assurance. The full exact-commit builder/red-team matrix passed; evidence and limits are in `tests/reports/slice-4.3-race-time-crash-assurance-builder-verification-2026-08-27.md`. This is not formal verification, power-loss testing, external time attestation, or active post-claim cancellation.

**Slice 4.4 local candidate:** commit `bdd8a00` adds integrity-gated aggregate health, pre-append audit capacity enforcement, explicit bounded-test readiness/backup/change rules, fail-closed incident playbooks, and a runbook safety check. The full exact-commit builder/red-team matrix passed; evidence and limits are in `tests/reports/slice-4.4-operational-readiness-builder-verification-2026-08-27.md`. This is a local operations profile, not production automation, safe sink rotation, HA, or acceptance.

**Slice 4.5 local candidate:** commit `536111a` adds an explicit authenticated A-to-B handoff entry point that always performs a fresh ordinary evaluation, names B as the only executor, and creates no bearer handoff credential. Substitution, self-handoff, implied parent authority, mutation, forwarding, and replay fail closed; an onward hop requires a separate requester role and new decision. The full exact-commit builder/red-team matrix passed; evidence and limits are in `tests/reports/slice-4.5-multi-agent-handoff-builder-verification-2026-08-27.md`. Portable parent/child linkage, transport, and federation remain deferred.

**Slice 4.6 local candidate:** commit `d51e46c` adds the external security review readiness package for security-code target `64d0820`, including the exact history/file manifest, trust-boundary and attack-surface map, reproduction matrix, non-claims, reviewer challenges, and tamper checks. The full exact-commit Phase 4 builder matrix passed; evidence is in `tests/reports/slice-4.6-external-security-review-readiness-builder-verification-2026-08-27.md`. Phase 4 is builder-complete and ready for independent review, not accepted.

**Open 0.1 defect:** Switchboard `DENY` still fails the frozen 0.1 decision schema. Fix on the 0.2 line, not by rewriting 0.1.

**Languages:** JS is the 0.1 reference and 0.2 oracle. Rust is the emerging authority. Go/Java/Python/TS become adapters.

**Immediate next gate:** independently review security-code target `64d0820` using review-package commit `d51e46c`, then record a per-slice Phase 4 disposition. The separate `sudo ./scripts/restricted-agent-acceptance.sh` gate for 3.9 remains outstanding and Phase 4 work must not be used to imply that it passed. Active post-claim cancellation, portable revocation/expiry/handoff-link evidence, and production hardening remain open.

Inspect `git status` before acting. Do not commit or push unless Robert asks.

Evidence: `tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md`, `tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md`, `tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md`, the dated slice 2.4–3.8 reports, and the explicitly unverified 3.9 candidate report under `tests/reports/`. Sequence and bars: `docs/BUILD-SPEC-SHEET.md`. Protocol: `docs/standard/SPEC-v0.2.md`.

## 6. Non-negotiable engineering rules

- Switchboard runs before policy and hard-denies unknown, untrusted, or out-of-scope principals.
- No LLM decides core `ALLOW`, `REQUIRE_APPROVAL`, or `DENY` outcomes.
- Invalid, ambiguous, unavailable, or unverifiable inputs fail closed.
- `DENY` means evaluated and refused; `EVALUATION_ERROR` means no decision could be established. Both block, but only errors may declare bounded retry conditions.
- Every evaluation error receives a sequence and sealed evidence row even though no authorization exists.
- Security-critical JSON uses the normative JCS profile and exact `sha256:` plus 64 lowercase hex representation.
- Authorization is bound to authenticated requester, authenticated executor, exact target, normalized arguments, payload/artifact digest, environment, tenant, adapter, nonce, and short claim deadline.
- Authorization proof may be portable; single-use consumption remains online and transactional. Offline verification cannot establish global non-consumption.
- Claim is atomic; expiration alone is not replay protection.
- Completion, failure, cancellation, rejection, revocation, or expiration permanently closes permission.
- Pending approval expires; only an authenticated authorized actor or system authority may cancel it.
- Caller-declared risk is advisory and can never lower policy-derived effective risk.
- Authorization nonces and IDs are authority-generated; caller request IDs are scoped and idempotent.
- Retries use idempotency and do not repeat the side effect.
- Agent B cannot use Agent A's authorization. A handoff requires a separate authorization explicitly naming B.
- Transactional state controls live authority; sealed audit provides evidence.
- Cryptographic roles are separated: authorization signing, audit sealing, service identity, operator authentication, and tenant trust.
- Adapters are trusted-path code and must authenticate, version, bind, and fail closed on incomplete translation.
- Prompts, chain-of-thought, credentials, and unrestricted sensitive payloads do not belong in Prism or audit records.
- Do not weaken an invariant to make a demo pass.
- A crash after a possible side effect creates an unknown outcome requiring reconciliation; never automatically replay an irreversible action.

## 7. Historical context

Use the original Trust Layer invariant to refine the core. Do not add M-of-N, anomaly scoring, ZK, dual ledgers, or hierarchical auth unless Robert changes priority. Do not invent patent or FTO claims.

## 8. Procedure at the start of a fresh session

1. Read this file, then only the docs the task needs.
2. Inspect repository branch, remotes, status, recent commits, and local-only files without changing them.
3. Summarize the current state, relevant evidence, and assumptions.
4. Identify whether the request is review, diagnosis, planning, implementation, testing, or publication.
5. For architecture/security work, propose the plan and surface tradeoffs before editing.
6. For implementation, define acceptance tests and failure cases before writing trust-path code.
7. Use independent frontier models for bounded builder, correctness, security, architecture, and conformance roles when Robert requests or authorizes multi-model participation.
8. Preserve every model's material disagreement and provide Robert the evidence needed to decide.

## 9. Procedure before declaring work complete

- Run tests proportional to the change, including adversarial and negative cases.
- Validate emitted records against schemas.
- Compare implementation behavior with the build spec.
- Re-run the two-agent requester/gate scenario for trust-path changes.
- Record the tested commit, tool versions, commands, exit codes, receipts, state transitions, side-effect evidence, and remaining limitations under `tests/reports/`.
- Do not call a test harness success proof of universal security.
- Update documentation when the public contract changes.
- Update private `skills.md` with durable session decisions and gotchas when the session ends.
- Commit or push only when Robert explicitly asks.

## 10. Collaboration principle

> Northstar treats trust as a disciplined relationship between humans and machines: participants may propose, question, disagree, and revise, while authority is explicit, actions are verified, evidence is preserved, and no participant is trusted beyond what the system can responsibly support.

The technical system creates conditions for good judgment. It does not manufacture wisdom, remove accountability, or replace the need to listen.
