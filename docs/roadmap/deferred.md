# Deferred (explicitly not-now)

Items that matter eventually but should **not** block Phase A/B.

| Item | Why deferred |
| --- | --- |
| Full SSO / OIDC for operators | Allowlist + route enough until real ops product |
| Multi-region HA / active-active | Premature before single-node control plane is used in anger |
| Java sealed-audit full parity | Go `tlpxd` covers service shape |
| Deep intent inspection / DPI | Fights declared-intent model; different product |
| ML risk scoring as primary gate | Breaks determinism story |
| Public open-source marketing launch | Owner decision; private repo is fine |
| Patent claim language in product docs | Legal/counsel; keep engineering language |
| Multi-party quorum / sentinel | After tokens + basic M-M handoff |
| Hardware attestation | After signed tokens exist |
| Perfect multi-tenant policy sandbox | Safe parser helps; isolation is still deploy concern |

---

## Residual WARNs we accept for now

From red team (JS):

- **A12** — declared intent can understate risk (by design metadata trust)  
- **A16** — process that never calls the gate (Phase A addresses for mediated paths)  
- **A17** — `allowEphemeral` exists for tests only  

---

## Revisit triggers

Pick up deferred items when:

- A real customer needs JVM-in-process audit  
- An ops console needs SSO  
- Two production agent fleets need cross-hop tokens  
- Counsel asks for public claim wording  
