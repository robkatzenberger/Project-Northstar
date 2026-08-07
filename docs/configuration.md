# Configuration

## Files overview

| Path | Purpose |
| --- | --- |
| `implementations/javascript/config/policy.yaml` | Deterministic policy rules (JS reference) |
| `implementations/javascript/config/switchboard.json` | Principals, whitelist, credibility, routes |
| `implementations/javascript/examples/*.json` | Intent fixtures |
| `var/*.jsonl` | Runtime audit logs at monorepo root (gitignored) |
| `schemas/tlpx-0.1/*.json` | Record schemas (shared) |

---

## Policy (`implementations/javascript/config/policy.yaml`)

### Format

APEX-Lite compatible, first-match-wins:

```yaml
rules:
  - id: rule_low_credibility
    description: Low credibility principal requires human approval
    if: low_credibility == true
    require: human_approval

  - id: rule_funds
    description: Require human approval for any fund transfer
    if: action == "transfer_funds"
    require: human_approval
```

### Rule fields

| Field | Meaning |
| --- | --- |
| `id` | Becomes decision `policy_id` |
| `description` | Human-readable `reason` |
| `if` | Boolean expression over intent fields |
| `require` | Any truthy value → `REQUIRE_APPROVAL` |
| `deny: true` | Also maps to `REQUIRE_APPROVAL` in policy layer (APEX lineage) |

### Expression language

Supported:

- Equality: `risk == "high"`
- Inequality: `risk != "low"`
- Membership: `"PII" in data_classes`
- Combinators: `and`, `or`

Fields often available after Switchboard enrichment:

- `action`, `target`, `risk`, `data_classes`
- `actor`, `actor_type`
- `whitelisted`, `credibility`, `credibility_band`
- `low_credibility`, `high_trust` (booleans)

**Security:** expression evaluation uses a **safe AST parser** (no `new Function`). Still load only operator-owned policy files.

### Ordering

Put **specific** rules before **broad** catch-alls. Example: `transfer_funds` before `risk == "high"`.

---

## Switchboard (`implementations/javascript/config/switchboard.json`)

### Top-level

| Field | Description |
| --- | --- |
| `switchboard_id` | Router identity on decision parties |
| `unknown_agent_policy` | `DENY` (default) or `REQUIRE_APPROVAL` |
| `thresholds.force_escalate_below` | default `0.4` |
| `thresholds.high_trust_at_or_above` | default `0.85` |
| `thresholds.max_credibility` | max `0.99` |
| `defaults.approval_route` | fallback operator list |
| `principals` | array of principal objects |

### Principal

| Field | Required | Description |
| --- | --- | --- |
| `id` | yes | Stable principal id (must match intent `actor` / `agent`) |
| `type` | yes | `machine` \| `agent` \| `human` |
| `whitelisted` | yes | `true` to allow past access gate |
| `credibility` | yes | number in `[0, 0.99]` |
| `allowed_actions` | no | if present, action must be listed |
| `approval_route` | no | ordered human ids |
| `label` | no | display name |

### Hard gates (before policy)

1. Unknown principal → `switchboard.unknown_deny` (or escalate if configured)  
2. `whitelisted: false` → `switchboard.not_whitelisted`  
3. Action not in `allowed_actions` → `switchboard.action_denied`  

### Demo principals (shipped)

| Id | Credibility | Notes |
| --- | --- | --- |
| `agent.docs.summarizer` | 0.91 | high trust, summarize only |
| `agent.support.mailer` | 0.62 | medium, send_email |
| `agent.finance.ops` | 0.35 | low → always escalate |
| `human.ops.jordan` | 0.88 | human declarer example |
| `agent.shadow.unknown` | 0.5 | **not** whitelisted |

---

## Intent fixtures (`examples/`)

| File | Intent |
| --- | --- |
| `intent-safe.json` | Low-risk summarize |
| `intent-pii-email.json` | Email + PII → escalate |
| `intent-funds.json` | Fund transfer / low credibility |
| `intent-human-deploy.json` | Human deploy to production |
| `intent-unknown-agent.json` | Unregistered principal |
| `intent-not-whitelisted.json` | Registered but disabled |

---

## Environment / runtime

No required environment variables for the reference gate.

| Optional practice | Suggestion |
| --- | --- |
| Audit path | Monorepo `var/tech-test-audit.jsonl` (+ companion `.seal` key file) |
| File mode | `chmod 600` on audit + seal files |
| Node | `>= 18` (JS reference) |

---

## Changing configuration safely

1. Edit policy or Switchboard under `implementations/javascript/config/`.  
2. From that package: `npm test` and `npm run conformance`.  
3. Run `node scripts/tech-test.mjs` or targeted CLI evaluates.  
4. Confirm `policy_id` / Switchboard flags match expectations.  

Do not lower unknown-agent policy to allow in production without a deliberate threat-model review.
