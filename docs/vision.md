# Vision

## Problem

AI agents are moving from chat to **action**: tools, money movement, production systems, agent-to-agent workflows. Supervision that only looks at logs **after** execution is structurally late.

Sensitive domains (finance, health, infrastructure, government, regulated industry) need:

1. **Declared intent before action**  
2. **Independent verification** (not only the model judging itself)  
3. **Human escalation** where risk demands it  
4. **Accountability** when something goes wrong — for **human error and machine error**

## Solution thesis

The missing layer is not another model. It is a **trust, verification, and authorization layer before execution**.

```text
Intent → Signal (Prism) → Verify (Switchboard + Glass) → Authorize → Execute → Audit
```

- **Prism** describes intent (metadata only).  
- **Switchboard** identifies who is acting and whether they may act.  
- **Glass / TL-PX** decides ALLOW / REQUIRE_APPROVAL (and DENY for access gates).  
- **Humans** authorize escalations.  
- **Executors** fail closed without AUTHORIZED.  
- **Audit** preserves the evidence graph.

## Product layering (open to an extent)

| Layer | Open? | Purpose |
| --- | --- | --- |
| Prism | Yes | Intent signal standard |
| TL-PX Minimum | Yes (this repo) | Spec + conformance + reference gate |
| APEX-Lite | Yes | Early playable concept |
| Glass enterprise | Optional commercial | Tokens, multi-tenant ops, advanced packs |

Ship a **minimum everyone can implement**; keep differentiated enterprise features as extensions that remain TL-PX-compatible where claimed.

## Design principles

1. **External boundary** — authority outside the model  
2. **Determinism** — same intent + policy ⇒ same decision (modulo ids/timestamps)  
3. **Air-gap capable** — local audit, no cloud required  
4. **Switchboard first** — identity and access before policy nuance  
5. **Transparency-rewarded escalation** — honest high-risk declaration is not punished by silent failure modes  
6. **Dual accountability** — humans and machines both appear on the chain  
7. **Honest scope** — do not claim universal containment of non-mediated processes  

## Target users

| Persona | Job to be done |
| --- | --- |
| Platform / AI engineer | Gate agent tool use before side effects |
| Security / GRC | Policy packs, audit export, proof of authorization |
| Operators | Approve/reject escalations with clear receipts |
| Agent framework authors | Emit Prism; honor Glass decisions |

## Non-goals (near term)

- Replacing LLMs or “ethical reasoning” in weights  
- General agent orchestration  
- Storing prompts / CoT in the intent signal  
- ML-based allow/deny as the primary gate  
- Capturing the open Prism standard under a single vendor  

## Success metrics (draft)

- Time-to-integrate: intent → decision in one local call  
- Determinism of policy outcomes  
- Audit completeness for evaluations and operator outcomes  
- Escalation quality: high-risk paths hit HITL; safe paths stay low-friction  
- Technical test #1 green on mediated executor paths  

## Status snapshot

- TL-PX 0.1 frozen; 0.2 evaluation/authorization contract core through 2.3 and bounded Rust 2.3d scope at `aed80e2` accepted; later local candidates remain unaccepted
- Restricted-marker PEP slice 3.9 passed bounded macOS gate `064717` at source `f025332`, preserved by evidence commit `32c049e`; universal forced mediation and network-egress mediation are not established
- Review disposition remains **changes requested**: the working tree repairs a subsequently found pre-lock deadline defect but needs exact-candidate evidence and independent re-review
- Reference implementation: Switchboard + air-gap + fail-closed policy compile (still emits 0.1)  
- Conformance + formal technical test #1 **passed**  
- A private baseline remote exists; post-baseline hardening remains local until an explicit push is chosen

## Open product questions

1. Receipt naming long-term: `tlpx.*` only vs dual Glass aliases  
2. When to introduce hard policy `DENY` beyond Switchboard — **0.2 contract: first-class `DENY`; 0.1 runtime/schema mismatch remains**  
3. Operator authentication standard  
4. Signed authorization tokens profile  
5. Governance split: standards nonprofit vs commercial Glass ops  

See [Architecture](./architecture.md), frozen [SPEC-v0.1](./standard/SPEC-v0.1.md), and draft [SPEC-v0.2](./standard/SPEC-v0.2.md).
