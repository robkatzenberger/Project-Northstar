# Switchboard — identity router for agents & machines

The **Switchboard** is the directory and router in front of Glass/TL-PX evaluation.

```text
Agent/Machine
    → Switchboard (who are you? allowed? how trusted? who approves?)
    → Policy engine (deterministic rules, can read credibility)
    → ALLOW | REQUIRE_APPROVAL | DENY
    → Human approvers on the routed path (if escalated)
```

## What it stores per principal

| Field | Meaning |
| --- | --- |
| `id` | Stable agent/machine/human identifier |
| `type` | `machine` \| `agent` \| `human` |
| `whitelisted` | Access grant — `false` ⇒ **DENY** |
| `credibility` | Trust score in **`[0, 0.99]`** (never 1.0 — no perfect trust) |
| `allowed_actions` | Optional action allow-list for that principal |
| `approval_route` | Ordered human (or role) ids for escalation |

Reference config: [`implementations/javascript/config/switchboard.json`](../implementations/javascript/config/switchboard.json)

## Credibility bands (defaults)

| Band | Default threshold | Effect |
| --- | --- | --- |
| **low** | `< 0.40` | Flag `LOW_CREDIBILITY` → policy forces human approval |
| **medium** | `0.40 – 0.84` | Normal policy rules |
| **high** | `≥ 0.85` | Flag `HIGH_TRUST` (available to policy; does not skip fund/PII rules) |

Scores are **operator-managed**, not model-judged. Optional helper `suggestCredibilityDelta()` proposes adjustments after outcomes (success/failure/etc.) — persistence is yours.

## Hard gates (before policy)

| Condition | Decision |
| --- | --- |
| Principal not registered | `DENY` (or escalate if `unknown_agent_policy: REQUIRE_APPROVAL`) |
| `whitelisted: false` | `DENY` |
| Action not in `allowed_actions` | `DENY` |

`DENY` is a **Glass/Switchboard extension** beyond TL-PX 0.1’s ALLOW/REQUIRE_APPROVAL pair. Execution of a denied decision is blocked.

## Approval routing

On `REQUIRE_APPROVAL`, the decision includes:

```json
"approval_route": ["human.finance.sam", "human.finance.lead"]
```

That’s the switchboard answering: *if a human must approve, who is on the wire for this agent?*

## CLI

```bash
cd implementations/javascript

# Lookup
node bin/glass.mjs switchboard agent.finance.ops

# Evaluate with switchboard (default config/switchboard.json if present)
node bin/glass.mjs evaluate examples/intent-funds.json config/policy.yaml \
  --log ../../var/tech-test-audit.jsonl

# Evaluate without switchboard
node bin/glass.mjs evaluate examples/intent-safe.json config/policy.yaml \
  --log ../../var/tech-test-audit.jsonl --no-switchboard
```

## Tests

```bash
node scripts/test-switchboard.mjs
```

## Fit with “minimum standard”

| Layer | Open minimum (TL-PX) | Switchboard (product/router) |
| --- | --- | --- |
| Decision enums | ALLOW / REQUIRE_APPROVAL | + DENY for access control |
| Identity | free-string actor | Registry + whitelist |
| Trust signal | none | credibility 0–0.99 |
| Escalation target | unspecified | `approval_route[]` |

Real deployments: Switchboard is where **agent identity and credibility** live; Glass is where **intent meets policy and audit**.
