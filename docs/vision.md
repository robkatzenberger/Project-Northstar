# Glass vision (Northstar)

## Problem

AI agents can act faster than humans can supervise. Logs arrive after the damage. Sensitive systems (finance, health, infra, government) need **authorization before action**, and agent-to-agent workflows need a **shared neutral trust signal**.

## Solution

The Trust Layer puts a checkpoint between **intent** and **execution**:

1. **Prism** — agent emits a lightweight intent signal (metadata only).
2. **Glass** — evaluates policy, identity, risk, and approval requirements.
3. **Authorize or escalate** — then execute; always audit.

Glass is not another model. It is **trust, verification, and authorization infrastructure**.

## Product thesis

APEX-Lite proves the interlock: external, deterministic, inspectable.  
**Glass productizes it** for enterprises that need:

- Operator identity and role-aware gates
- Richer policy packs and environments (dev / staging / prod)
- Human-in-the-loop queues that scale beyond a local console
- Authorization tokens that downstream systems can verify
- Revocation, emergency stop, multi-party / sentinel quorum (roadmap)
- Durable, exportable audit suitable for compliance review
- Deployable service form (not only CLI + local server)

## Target users

| Persona | Job |
| --- | --- |
| Platform / AI eng | Drop a gate in front of agent tool use |
| Security / GRC | Policy packs, audit export, proof of authorization |
| Operators | Approve / reject escalations with a clear receipt trail |
| Agent framework authors | Emit Prism; honor Glass decisions |

## Use cases (from Trust Layer narrative)

- Enterprise internal agents on production systems
- Financial operations before money moves
- Healthcare workflows with required HITL
- Legal & compliance audit trails
- DevOps automation (deploy, migrate, infra)
- AI-to-AI transactions with a shared trust signal
- Government / regulated industry oversight

## Non-goals (for early Glass)

- Replacing the LLM or doing “ethical reasoning” in-model
- Becoming a general agent orchestrator
- Storing prompts or chain-of-thought in the intent signal
- Opaque ML-based allow/deny as the primary decision path
- Capturing the open Prism standard under a single vendor (protocol stays open)

## Success metrics (draft)

- Time-to-integrate: agent emits Prism → gets Glass decision in one HTTP round trip
- Decision determinism: same intent + policy → same decision + stable reason codes
- Audit completeness: every evaluation and operator outcome has a receipt line
- Escalation quality: high-risk paths hit HITL; safe paths stay low-friction

## Open product questions

1. Glass API contract: extend APEX-Lite receipts or version a new `glass.receipt`?
2. AuthN for agents and operators (API keys, mTLS, OIDC)?
3. Multi-tenant policy isolation model?
4. Where signed authorization tokens live (JWT, custom MAC, offline verify)?
5. Relationship to nonprofit standards body vs commercial Glass ops (per site “separation of powers”)?

Resolve these before large implementation surface area.
