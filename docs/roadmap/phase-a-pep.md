# Phase A — Policy Enforcement Point (PEP) / forced mediation

**Status:** Problem statement retained; sequence and acceptance bar SUPERSEDED — 2026-08-13  
**Current baseline:** [`../BUILD-SPEC-SHEET.md`](../BUILD-SPEC-SHEET.md) §14 and §14.1  
**Disposition:** [`../reviews/build-plan-review-disposition-2026-08-13.md`](../reviews/build-plan-review-disposition-2026-08-13.md)

A16 remains real: calling the library is optional, so this is not yet an enforcement boundary. That diagnosis stands.

Do not treat the August 7 first deliverable (`pep-run.mjs` / cooperative CLI wrapper) as the acceptance test. Slice 3.8 now provides the explicitly labeled cooperative `tlpx-run-demo` at local commit `7c41450`; it runs under the caller's UID and is bypassable. Slice 3.9 requires separate OS identities and the eight acceptance assertions in the build sheet. Forced mediation is not current implementation status; only the bounded `aed80e2` Rust evaluate/issue/claim scope is independently accepted.

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
| Read-only search / list | `read` or omit | low |
| Edit / write files | `modify_file` | medium |
| Shell | `shell` | medium–high |
| Network egress | `access_api` | medium–high |
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

### A3 — Local HTTP PEP client → `tlpxd`

```text
Agent tool request
  → POST /v1/evaluate
  → if PENDING: queue human
  → if DENY: reject tool
  → if AUTHORIZED: perform tool
  → POST /v1/execution
```

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

PEP helps for **tool-mediated** steps (HTTP, shell, credential use via harness).  
It does **not** replace sandbox/hypervisor containment if the model escapes without using the tool API.

See [openai-hf-incident-notes.md](./openai-hf-incident-notes.md).
