# Priorities — what to build next

**Status:** SUPERSEDED as current direction — 2026-08-13  
**Retained as:** historical product thinking from 2026-08-07  
**Current baseline:** [`../BUILD-SPEC-SHEET.md`](../BUILD-SPEC-SHEET.md)  
**Disposition:** [`../reviews/build-plan-review-disposition-2026-08-13.md`](../reviews/build-plan-review-disposition-2026-08-13.md)

Do not implement from this file. The live sequence is strict fail-closed policy in the JS/TS reference (slices 1.1–1.2), then a separately versioned TL-PX 0.2 contract, then a Rust authority and OS-enforced PEP. The August 7 PEP → signed `AUTHORIZED` snapshot token → expand Go → Rust-later order is no longer current.

The useful historical rule of thumb remains below. It is now restated as: a portable claim ticket without transactional consumption is theater; handoff without a separately named executor authorization does not travel.

## Rule of thumb

> One killer path end-to-end beats three half-finished layers.  
> Tokens without mediation are theater; handoff without tokens doesn’t travel.

---

## Priority table

| Priority | Item | Do now? | Why |
| --- | --- | --- | --- |
| **1** | Forced mediation (PEP adapters) | **Yes** | Without this, everything else only helps honest callers |
| **2** | Signed short-lived authz tokens | **Yes — design + thin MVP** | Makes AUTHORIZED portable across processes/agents |
| **3** | Multi-agent receipt handoff | **After 1–2** | Natural once tokens exist |

---

## Phase sequence

### Phase A — “Nobody acts without a receipt”

1. Reference PEP for one real surface (CLI wrapper and/or harness hook calling gate/`tlpxd`)  
2. Fail-closed on evaluate errors  
3. Small action taxonomy: `shell`, `modify_file`, `access_api`, `spawn_agent`  

**Done when:** a coding agent or test harness cannot complete a high-risk tool call without AUTHORIZED.

→ Details: [phase-a-pep.md](./phase-a-pep.md)

### Phase B — “Portable authorization”

1. Spec `tlpx.authz_token`  
2. Issue on ALLOW / post-APPROVE; verify offline with public key  
3. Executor checks token (audit remains issuer source of truth)  

**Done when:** a second process can reject action without sharing the audit file.

→ Details: [phase-b-authz-tokens.md](./phase-b-authz-tokens.md)

### Phase C — “Machine B won’t move without A’s auth”

1. Peer envelope: Prism + authz token  
2. B’s Switchboard: trusted issuers + action scope  
3. Demo: A → gate → token → B executes → linked receipt_ids  

**Done when:** two agents only complete a hop if the token verifies.

→ Details: [phase-c-mm-handoff.md](./phase-c-mm-handoff.md)

---

## Explicitly not first

- Full SSO / enterprise IdP for operators (allowlist OK until ops app)  
- Multi-region HA  
- Java full audit parity (Go control plane enough for service shape)  
- Perfect intent-truth / deep content inspection  
- Spreading thin across more languages before PEP + tokens  

See [deferred.md](./deferred.md).

---

## Why this order

```text
Without PEP:     agents with always-approve still act freely
Without tokens:  AUTHORIZED is local to one audit log
Without handoff: M-M is polite cooperation, not a protocol
```

---

## Suggested next work session

1. Draft token fields in `phase-b-authz-tokens.md` (already here) → implement minimal issue/verify in JS  
2. One PEP path (even a `bin/pep-shell.mjs` that wraps shell behind evaluate)  
3. Two-agent demo script once token exists  

Do **not** start Phase C protocol polish before A and B have a walking demo.
