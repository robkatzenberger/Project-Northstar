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

## 4. Required reading order

Change into the repository root, then read these files completely in order.

### Tier 1 — mandatory for every substantive session

1. `NORTHSTAR-SESSION-START.md` — this orientation file.
2. `skills.md` — private local session memory, preferences, current status, and gotchas. It is gitignored; if absent, continue with shared docs.
3. `AGENTS.md` — repository-wide working rules.
4. `docs/BUILD-SPEC-SHEET.md` — current hardened build baseline and accepted security requirements.
5. `tests/reports/northstar-two-agent-test-proof.md` — independent baseline evidence and confirmed defects.
6. `docs/reviews/build-spec-review-2026-08-11-model-2.md` — independent specification findings and accepted dispositions.
7. `docs/reviews/build-plan-review-disposition-2026-08-13.md` — implementation boundary: Phase 1 only, abandoned snapshot token, OS-enforced 3.9 bar.
8. `docs/vision.md` — product thesis and honest boundaries.
9. `docs/architecture.md` — current implemented architecture.
10. `docs/security.md` — current security model and limitations.
11. `docs/standard/SPEC-v0.1.md` — current normative TL-PX contract; the hardened plan targets a separately versioned TL-PX 0.2.

### Tier 2 — mandatory before implementation work

12. `implementations/javascript/AGENTS.md`
13. `implementations/javascript/README.md`
14. `docs/testing.md`
15. `docs/configuration.md`
16. `docs/integration.md`
17. `docs/airgap.md`
18. `docs/switchboard.md`
19. `tests/README.md`

### Tier 3 — read for roadmap, protocol, or product decisions

- `docs/roadmap/priorities.md` — superseded as current direction; history only
- `docs/roadmap/phase-a-pep.md` — A16 problem retained; sequence superseded
- `docs/roadmap/phase-b-authz-tokens.md` — abandoned AUTHORIZED snapshot token
- `docs/roadmap/phase-c-mm-handoff.md` — handoff goal retained; do not use the abandoned token
- `docs/roadmap/hm-mm-runtime.md`
- `docs/roadmap/enterprise-switchboard.md`
- `docs/roadmap/deferred.md`
- `docs/roadmap/language-strategy.md` — superseded as current direction
- `docs/concepts.md`
- `docs/glossary.md`
- `docs/SHARE.md`

### Tier 4 — implementation references as relevant

- `implementations/README.md`
- `implementations/go/README.md`
- `implementations/java/README.md`
- `implementations/rust/README.md`
- `implementations/python/README.md`
- `docs/api-reference.md`
- `docs/cli-reference.md`
- `docs/http-api.md`
- `docs/logging.md`
- `docs/getting-started.md`
- `docs/changelog.md`
- `docs/review-2026-07-22.md`

Do not load every file merely to create context. Read Tier 1 first, determine the actual task, then load the relevant lower-tier documents and source files completely.

## 5. Current repository state

At the time this file was last updated (2026-08-14):

- Phase 1 slices 1.1–1.2 are implemented and accepted (`31175c5`, follow-up `b82ae1d`).
- Slice 2.1 SPEC-v0.2 decision/error/state contract is accepted (`593439b`).
- Slice 2.2 JCS/hash oracle and golden fixtures are accepted (`b4fb238`, Unicode follow-up `636637a`).
- Next unopened slice is **2.3** (schemas, validators, 0.2 record conformance). Not started.
- JavaScript is the functioning 0.1 reference plus a 0.2 fixture oracle. It still emits `standard_version: "0.1.0"`.
- Go has a partial control-plane/service implementation.
- Java has policy and Switchboard evaluation.
- Rust and Python are placeholders.
- The agreed future direction is a small Rust authoritative security core and PEP.
- TypeScript remains the readable reference, conformance oracle, and adversarial harness.
- Java, Go, Python, and TypeScript should become SDKs/adapters rather than competing authorization authorities.

Always inspect `git status`, current branch, remote, and recent commits before acting. Preserve unrelated local work. Do not push unless Robert asks.

## 6. Verified baseline and known defects

The independent two-agent test of implementation commit `ca05f6996534471e817d11f3c668e38411797fb8` found:

- `npm test`: 79 assertions passed.
- conformance: 47/47 passed.
- technical test: 29/29 passed.
- red team: 16 PASS, 0 FAIL, 4 documented WARN.
- sealed nine-record audit verified.
- safe and approved actions executed.
- pending, forged, rejected/denied, and unknown-principal actions were blocked.
- direct action outside the gate succeeded, proving forced mediation is not yet implemented.

Two concrete defects were independently reproduced:

1. Malformed policy can fail open by becoming a non-match and falling through to `ALLOW`.
2. Runtime Switchboard `DENY` does not validate against the current decision schema/validator.

Do not describe the present implementation as production-ready or an unavoidable enforcement boundary.

## 7. Agreed build order

1. Strict policy parsing, validation, and compilation; invalid policy never authorizes. **Done (1.1–1.2).**
2. Freeze TL-PX 0.1 and its 47 fixtures as historical evidence. **Done (2.1).**
3. Draft TL-PX 0.2 decisions, evaluation errors, lifecycle states, receipts, reason codes, and compatibility rules. **Done (2.1).**
4. Define separate submitted-intent and authority-normalized authorized-action schemas. **Field lists in SPEC-v0.2; JSON Schemas are 2.3.**
5. Define RFC 8785 canonicalization, exact lowercase `sha256:` representation, domain-separated hashes, and cross-language golden fixtures. **Done (2.2, UTF-16 sort).**
6. Build TL-PX 0.2 schemas, validators, and a distinct conformance suite. **Next, not started.**
7. Create the Rust authority skeleton and shared language-neutral contract types.
8. Add authenticated requester, operator, executor, canceller, and adapter identities.
9. Add approval expiry/cancellation and SQLite transactional state.
10. Add single-use atomic claim, 3–5 second claim window, revocation, idempotency, and unknown-outcome reconciliation.
11. Build the `tlpx-run` PEP prototype, then run the authenticated restricted-agent enforcement acceptance test.
12. Add operational hardening and an explicit non-transitive multi-agent handoff profile.
13. Complete independent security and conformance review before strong production claims.

## 8. Non-negotiable engineering rules

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

## 9. Historical Trust Layer context

The original Trust Layer work included pre-execution action manifests, Truth Ping, signed consent, tamper-evident ledgers, sentinel/quorum verification, revocation, provenance, multi-agent verification, human/machine consensus, anomaly signals, and advanced cryptographic profiles.

Use the historical invariant to refine the current core, not to indiscriminately add every speculative Glass feature. M-of-N sentinels, anomaly/drift scoring, decentralized attestations, ZK proofs, dual ledgers, and hierarchical authorization remain extension profiles unless Robert explicitly changes priority.

If patent, provenance, or original-design review is required, ask Robert to attach the original PDFs/DOCX/diagram artifacts in the current context. Do not invent patent claims, novelty conclusions, or freedom-to-operate opinions.

## 10. Procedure at the start of a fresh session

1. Read Tier 1 documents.
2. Inspect repository branch, remotes, status, recent commits, and local-only files without changing them.
3. Summarize the current state, relevant evidence, and assumptions.
4. Identify whether the request is review, diagnosis, planning, implementation, testing, or publication.
5. For architecture/security work, propose the plan and surface tradeoffs before editing.
6. For implementation, define acceptance tests and failure cases before writing trust-path code.
7. Use independent frontier models for bounded builder, correctness, security, architecture, and conformance roles when Robert requests or authorizes multi-model participation.
8. Preserve every model's material disagreement and provide Robert the evidence needed to decide.

## 11. Procedure before declaring work complete

- Run tests proportional to the change, including adversarial and negative cases.
- Validate emitted records against schemas.
- Compare implementation behavior with the build spec.
- Re-run the two-agent requester/gate scenario for trust-path changes.
- Record the tested commit, tool versions, commands, exit codes, receipts, state transitions, side-effect evidence, and remaining limitations under `tests/reports/`.
- Do not call a test harness success proof of universal security.
- Update documentation when the public contract changes.
- Update private `skills.md` with durable session decisions and gotchas when the session ends.
- Commit or push only when Robert explicitly asks.

## 12. Collaboration principle

> Northstar treats trust as a disciplined relationship between humans and machines: participants may propose, question, disagree, and revise, while authority is explicit, actions are verified, evidence is preserved, and no participant is trusted beyond what the system can responsibly support.

The technical system creates conditions for good judgment. It does not manufacture wisdom, remove accountability, or replace the need to listen.
