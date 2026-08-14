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

**Accepted:** slices 1.1–2.2. JS gate is TL-PX 0.1 + fail-closed compile + 0.2 JCS/hash oracle. It still emits `standard_version: "0.1.0"`.

**2.3** schemas + `conformance:0.2` are in this tree. Next implementation evidence: smallest Rust authority, then OS-enforced PEP (3.9).

**3.9** is the system-level claim. 1.1–2.3 are real security proofs; they are not forced mediation.

**Open 0.1 defect:** Switchboard `DENY` still fails the frozen 0.1 decision schema. Fix on the 0.2 line, not by rewriting 0.1.

**Languages:** JS is the 0.1 reference and 0.2 oracle. Rust is the planned authority. Go/Java/Python/TS become adapters.

Inspect `git status` before acting. Do not commit or push unless Robert asks.

Evidence: `tests/reports/` (immutable). Sequence and bars: `docs/BUILD-SPEC-SHEET.md`. Protocol: `docs/standard/SPEC-v0.2.md`.

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
