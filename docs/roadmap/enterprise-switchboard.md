# Enterprise Switchboard — when it becomes a server

## Today (reference)

Switchboard is a **JSON registry + library** loaded in-process (JS/Go/Java).

Fine for:

- Single app / single team  
- Air-gapped box with one gate process  
- Pilots  

---

## Enterprise needs that outgrow a file

| Need | File-based | Dedicated service |
| --- | --- | --- |
| Many apps share one agent registry | Painful | **Source of truth** |
| Central revoke / offboarding | Manual edit | API + admin + change audit |
| Credibility updates over time | Redeploy JSON | Controlled updates + history |
| Multi-tenant / multi-env | Risky | Isolation |
| HA | N/A | Service design |
| Compliance: who was allowed *when*? | Weak | Registry changelog |

Enterprise doesn’t require a server because of TL-PX math — it requires one because of **identity, scale, and governance**.

---

## Phased shape

| Phase | Shape |
| --- | --- |
| **0 – Pilot** | In-process library + JSON (current) |
| **1 – First enterprise** | **One Trust Layer service** (Switchboard + gate + audit) — e.g. Go `tlpxd` grown up |
| **2 – Scale** | Split Switchboard (IAM-ish) from Gate if needed |
| **3 – Platform** | Multi-tenant Switchboard, signed tokens, admin UI, SCIM/SSO for operators |

---

## Roles

| Component | Enterprise form |
| --- | --- |
| Prism | Standard / SDK |
| TL-PX / Glass gate | Decision service + audit |
| Switchboard | Identity & trust registry service |
| Executor / PEP | Inside each runtime |

**Switchboard = directory + bouncer.**  
**Gate = judge.**

Smart path: **one service first**, extract Switchboard later if many systems depend on it alone.

---

## Suggested Switchboard service API (future)

- `GET /principals/{id}`  
- `POST /route` — given actor + action → gate context / hard DENY  
- Admin CRUD for principals, credibility, routes  
- Audit of config changes (separate from action audit)  

Still feeds the same TL-PX evaluate path and shared schemas.
