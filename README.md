# Northstar — Glass MVP (local)

**Codename:** `northstar`  
**Product:** **Glass** — human + machine trust layer  
**Status:** runnable local MVP for critique (not on GitHub)

Intent is declared **before** execution. When something goes wrong, an append-only audit trail shows **who declared**, **what Glass decided**, **who approved**, and **who executed** — for humans and machines.

---

## Quick start

```bash
cd ~/projects/northstar

npm test          # 23 assertions
npm run demo      # 3 scenarios → var/demo-audit.jsonl
```

```bash
# evaluate → approve → execute → incident
node bin/glass.mjs evaluate examples/intent-pii-email.json config/policy.yaml --log var/audit.jsonl
node bin/glass.mjs approve <receipt_id> --operator human.ops.alex --log var/audit.jsonl
node bin/glass.mjs execute <receipt_id> --executor runtime.mailer --status EXECUTED --log var/audit.jsonl
node bin/glass.mjs incident <receipt_id> --log var/audit.jsonl --why "Wrong attachment sent"
node bin/glass.mjs chain <receipt_id> --log var/audit.jsonl
```

Zero npm dependencies. Node 18+.

---

## Trust chain

```text
DECLARER (human|machine)  →  Prism signal
EVALUATOR (glass/machine) →  ALLOW | REQUIRE_APPROVAL
AUTHORIZER (human, if escalated) → APPROVE | REJECT
EXECUTOR (human|machine)  →  EXECUTED | BLOCKED | FAILED
AUDIT → optional INCIDENT → accountability report
```

---

## Layout

```text
bin/glass.mjs              CLI
src/prism.mjs              Intent signal
src/policy.mjs             Deterministic rules (APEX-Lite compatible)
src/glass.mjs              Decide / authorize / execute records
src/audit.mjs              JSONL append + read
src/accountability.mjs     Human + machine evidence graph
config/policy.yaml         Demo policy pack
examples/                  Intent fixtures
scripts/test.mjs           Tests
scripts/demo.mjs           Narrative demo
docs/MVP.md                Full handoff brief
docs/vision.md             Product thesis
docs/architecture.md       Layering notes
AGENTS.md                  Collaboration rules
```

---

## Ecosystem map

| Layer | Role | Reference |
| --- | --- | --- |
| **Prism** | Metadata-only intent signal | [prism-protocol](https://github.com/Trust-Layer-AI/prism-protocol) |
| **APEX-Lite** | OSS minimal gate | `~/APEX-Lite`, [Trust-Engine](https://github.com/Trust-Layer-AI/Trust-Engine) |
| **Glass** | Product trust layer (this repo) | `~/projects/northstar` |
| Site | Public story | [Trust Layer AI](https://trust-layer-ai.github.io/Trust-Layer-AI/) |

---

## Read next

1. **[docs/MVP.md](docs/MVP.md)** — what we built, scope, patent-research notes, open questions  
2. Run **`npm run demo`** and read the accountability findings  
3. Upload your additional docs when ready so we can refine against the real framework

---

## License / remote

- Code is **local-only**, `UNLICENSED` until you decide.  
- **No GitHub remote** by design. Public Apache-2.0 refs are separate projects.
