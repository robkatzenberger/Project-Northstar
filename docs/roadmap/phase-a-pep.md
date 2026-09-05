# Phase A — Policy Enforcement Point (PEP) / forced mediation

**Status:** Problem statement retained; sequence and acceptance bar SUPERSEDED — 2026-08-13  
**Current baseline:** [`../BUILD-SPEC-SHEET.md`](../BUILD-SPEC-SHEET.md) §14 and §14.1  
**Disposition:** [`../reviews/build-plan-review-disposition-2026-08-13.md`](../reviews/build-plan-review-disposition-2026-08-13.md)

A16 remains real for every capability not placed behind an OS-enforced PEP. The bounded 3.9 marker profile has separate-identity builder evidence; that does not generalize to the shell, browser, or network.

Do not treat the August 7 first deliverable (`pep-run.mjs` / cooperative CLI wrapper) as an acceptance test. Slice 3.8 provides an explicitly cooperative demo. The reviewer later accepted only the bounded macOS marker profile at source `82f5cd6`, evidence child `6bb0a07`, administrator report `172840`; see the [current review packet](../reviews/CURRENT-REVIEW.md). The independently accepted implementation baseline remains the bounded `aed80e2` evaluate/issue/claim scope. No universal forced-mediation, hostile same-UID, production, or network-egress claim follows.

## Goal

Sensitive side effects **cannot complete** unless they go through Switchboard + TL-PX + fail-closed execute.

Today the library is honest; **calling it is still optional**. Phase A makes mediation real for at least one runtime surface.

---

## Problem (A16)

```text
Agent / script
    │
    ├── calls Trust Layer  →  protected path
    └── skips Trust Layer  →  unrestricted side effect
```

Always-approve coding harnesses make this worse: UI prompts are not the control.

---

## Design principles

1. **Fail-closed** on gate errors, timeouts, integrity failures (unlike many harness hooks that fail-open).  
2. **Air-gap OK** — call local library or localhost `tlpxd`, not a cloud dependency.  
3. **Switchboard first** — unknown agents never reach tools.  
4. **Small taxonomy first** — don’t map every tool under the sun on day one.  

---

## Action taxonomy (initial)

| Host action | Intent `action` | Typical risk |
| --- | --- | --- |
| Destination-specific read/list | Named typed operation; never infer read-only from HTTP method | deployment-specific |
| Edit / write files | `modify_file` | medium |
| Shell | `shell` | medium–high |
| Network egress | No generic action; use a destination-specific future profile | high |
| Deploy | `deploy` | high |
| Payments | `transfer_funds` | high |
| Spawn subagent | `spawn_agent` | medium |

Register principals with tight `allowed_actions` on Switchboard.

---

## Adapter options

### A1 — Library mediation (preferred default)

Application / worker only performs side effects inside `executeAuthorized` (or Go ops equivalent).

Already documented in `docs/integration.md` Pattern A.

### A2 — CLI / subprocess gate

Job runner shells out to `glass` / `tlpx` evaluate; only runs payload if AUTHORIZED.

### A3 — Historical HTTP transport sketch, not a PEP

```text
Agent tool request
  → POST /v1/evaluate
  → if PENDING: queue human
  → if DENY: reject tool
  → if AUTHORIZED: perform tool
  → POST /v1/execution
```

This sketch is not implemented by the Rust authority and does not mediate outbound HTTP. The Go `tlpxd` path trusts caller-supplied identities and records caller-reported execution. Do not route 0.2 permission traffic through it or present it as protected egress. A future OS/network boundary remains deferred.

### A4 — Harness hook (e.g. Grok Build PreToolUse)

- **Adapter only**, not the product core  
- Must return explicit deny; handle errors fail-closed for high-risk tools  
- Register `agent.grok.main` (and subagent types) in Switchboard  

Hooks are a distribution channel; air-gapped gate remains the authority.

---

## Minimal first deliverable (suggested)

1. `implementations/javascript/bin/pep-run.mjs` (or similar):  
   - args: agent id, action, summary, command/script  
   - evaluate (+ switchboard) → executeAuthorized only on ALLOW path  
   - escalate path prints receipt and exits non-zero until approved  
2. Switchboard principals for demo agents used by PEP  
3. Doc + tech-test style script: DENY unknown, block pending, allow safe  

**Done when:** high-risk shell/file action through PEP is blocked without AUTHORIZED, with sealed audit evidence.

---

## Non-goals for Phase A

- Supporting every agent framework  
- Perfect semantic detection of lying intents  
- Replacing OS sandbox / container isolation  

---

## Relationship to OpenAI/HF-style incidents

Northstar helps only when the complete capability is forcibly mediated and a trusted adapter constructs its exact semantics. A generic HTTP method/host mapping or optional harness hook does not meet that bar. It does not replace sandbox/hypervisor containment if the model reaches the capability outside the PEP.

See [openai-hf-incident-notes.md](./openai-hf-incident-notes.md).
