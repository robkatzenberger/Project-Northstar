# Glossary

## Product names

| Term | Meaning |
| --- | --- |
| **Trust Layer** | Overall product/ecosystem: pre-execution trust, verification, authorization |
| **Northstar** | Codename for this workspace / Glass product path |
| **TL-PX** | Trust Layer Pre-Execution Minimum Standard (this repo’s open minimum) |
| **Glass** | Product / enterprise gate surface; reference impl here implements TL-PX + extensions |
| **Prism** | Open metadata-only intent signal protocol |
| **APEX-Lite / Trust-Engine** | Early OSS minimal policy boundary |
| **Switchboard** | Principal registry and router (whitelist, credibility, approval routes) |

## Decisions & authorization

| Term | Values / meaning |
| --- | --- |
| **decision** | `ALLOW` \| `REQUIRE_APPROVAL` \| `DENY` |
| **authorization_status** | `AUTHORIZED` \| `PENDING_HUMAN_APPROVAL` \| `DENIED` |
| **control_mode** | `ALLOW_OR_ESCALATE` (policy default philosophy) |
| **operator outcome** | `APPROVE` \| `REJECT` |
| **execution status** | `EXECUTED` \| `BLOCKED` \| `FAILED` |

### Mapping (normative for this reference)

| Decision path | authorization_status |
| --- | --- |
| `ALLOW` | `AUTHORIZED` |
| `REQUIRE_APPROVAL` (unresolved) | `PENDING_HUMAN_APPROVAL` |
| Operator `APPROVE` | `AUTHORIZED` |
| Operator `REJECT` | `DENIED` |
| Switchboard `DENY` | `DENIED` |

## Records

| `record_type` | Purpose |
| --- | --- |
| `tlpx.decision` | Gate decision receipt (`glass.decision` accepted as alias) |
| `tlpx.operator_action` | Human resolve of escalation |
| `tlpx.execution` | Execution attempt outcome |
| `tlpx.incident` | Optional incident note |
| `tlpx.accountability_report` | Post-incident evidence summary |

## Parties & roles

| Role | Description |
| --- | --- |
| **declarer** | Who stated the intent |
| **router** | Switchboard identity router |
| **evaluator** | Policy engine identity |
| **authorizer** | Who approved/rejected escalation |
| **executor** | Who attempted the side effect |

Party shape: `{ id: string, type: "human" | "machine" }`.

## Switchboard

| Term | Meaning |
| --- | --- |
| **principal** | Registered agent/machine/human identity |
| **whitelisted** | Access grant; false → DENY |
| **credibility** | Trust score in **[0, 0.99]** inclusive max 0.99 |
| **credibility_band** | `low` \| `medium` \| `high` \| `unknown` |
| **allowed_actions** | Optional action allow-list per principal |
| **approval_route** | Ordered operator ids for HITL |
| **unknown_agent_policy** | `DENY` (default) or `REQUIRE_APPROVAL` |

### Default credibility thresholds

| Band | Default |
| --- | --- |
| low | score &lt; 0.40 |
| high | score ≥ 0.85 |
| medium | between |

## Identifiers

| Field | Meaning |
| --- | --- |
| **intent_id / prism_id** | Id of the intent instance |
| **receipt_id** | Id of a decision evaluation; links the whole chain |
| **correlation** | Optional; can use session ids from host harnesses |

## Air-gap terms

| Term | Meaning |
| --- | --- |
| **auditPath** | Path to append-only JSONL log (required for durable ops) |
| **chain-verified** | Auth derived only from audit history for a receipt |
| **fail-closed** | No side effect unless AUTHORIZED |
| **allowEphemeral** | Test-only escape to skip audit (never use in production adapters) |
| **TCB** | Trusted computing base (e.g. who can write the audit file) |

## Policy

| Term | Meaning |
| --- | --- |
| **first match wins** | Rules evaluated in order; first true `if` applies |
| **require** | Rule field that produces REQUIRE_APPROVAL |
| **policy_id** | Id of matched rule, or Switchboard gate id, or null if ALLOW |

## Testing

| Term | Meaning |
| --- | --- |
| **conformance** | Spec pass/fail suite (`npm run conformance`) |
| **technical test #1** | Formal E2E air-gap test (`node scripts/tech-test.mjs`) |
| **red team** | Adversarial probes (`scripts/adversarial-redteam.mjs`) |
