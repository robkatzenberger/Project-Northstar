# Phase C — Multi-agent receipt / token handoff

**Status:** Goal retained; dependency on the abandoned snapshot token removed — 2026-08-13  
**Current baseline:** [`../BUILD-SPEC-SHEET.md`](../BUILD-SPEC-SHEET.md) §12  
**Disposition:** [`../reviews/build-plan-review-disposition-2026-08-13.md`](../reviews/build-plan-review-disposition-2026-08-13.md)

Handoff still requires a separately evaluated authorization that explicitly names the next executor. Parent permission is never transitive.

Do not implement this phase against `docs/roadmap/phase-b-authz-tokens.md`. That snapshot token is abandoned. Any later portable artifact must be a claim ticket whose single-use consumption remains online and transactional.

The remaining August 7 text below is retained only as historical design context. Its `tlpx.authz_token`, Phase B dependency, diagrams, field names, and offline-verification flow are superseded and must not be used as a current contract or implementation plan.

## Historical goal

**Machine B will not act** on a request from Machine A unless A presents a **valid authorization** from a trusted Trust Layer issuer (token from Phase B, bound to intent).

---

## Depends on

- Phase A: B’s side effects are mediated (PEP)  
- Phase B: portable `tlpx.authz_token`  

Without those, “handoff” is only a polite API convention.

---

## Happy path

```text
Agent A                         Trust Layer                      Agent B
   |                                 |                               |
   |-- intent: call B / do X ------->|                               |
   |<-- AUTHORIZED + authz_token ----|                               |
   |                                                                 |
   |-- Prism summary + authz_token --------------------------------->|
   |                                 |                               |
   |                                 |<-- verify token + Switchboard-|
   |                                 |    (iss trusted? action ok?)  |
   |                                 |                               |
   |                                 |   if ok: execute B's action   |
   |                                 |   audit: link receipt_ids     |
```

---

## Envelope (draft)

```json
{
  "typ": "tlpx.peer_request",
  "v": "0.1",
  "from_agent": "agent.a",
  "to_agent": "agent.b",
  "prism": {
    "prism_id": "…",
    "agent": "agent.a",
    "intent_summary": "Request B to fetch customer profile",
    "prism_version": "prism_v0.1"
  },
  "authz_token": { "...": "tlpx.authz_token" },
  "payload_ref": "opaque-or-encrypted-handle"
}
```

**Rules of thumb:**

- Don’t put secrets in Prism  
- Token authorizes the *kind* of hop; payload may be separate and encrypted  
- B re-checks Switchboard for *itself* as executor + A as peer  

---

## Switchboard roles in M-M

| Principal | Check |
| --- | --- |
| A (caller) | Whitelisted? Allowed to initiate this action? Credibility? |
| B (callee) | Whitelisted as executor? Allowed action? |
| Issuer | Is token `iss` trusted? |

---

## Audit linking

Both sides should record:

- A’s `receipt_id` (issuing gate)  
- B’s local `receipt_id` if B also evaluates  
- `parent_receipt_id` / `peer_receipt_id` field (extension)  

Accountability can then answer: who started the chain, who approved, who executed on B.

---

## Demo script (when built)

1. A intends `agent_chain_step` / `access_api` toward B  
2. Gate escalates or allows; issues token  
3. B receives envelope; verify fails if token missing/expired/wrong action  
4. B executes only on success; both logs show linked ids  

---

## Non-goals for first M-M demo

- Fully decentralized multi-hop DHT of trust  
- Economic staking / slashing  
- Automatic discovery of agents on the public internet  

Start with **two named agents**, one issuer, localhost or air-gapped LAN.

---

## Success metric

> Two agents only complete a sensitive hop if the authz token verifies; failure modes are DENY/BLOCKED with audit evidence on both sides (or clear verify error on B).
