# Trust Layer Pre-Execution Minimum Standard (TL-PX)

**Codename:** Northstar  
**What this is:** An **open minimum standard** + **conforming reference implementation** for human + machine trust checkpoints *before* execution.

APEX-Lite was the playable concept. **TL-PX** is the hardened minimum you can implement against, test with a conformance suite, and wire into real applications.

> Intent is declared before action. Decisions are deterministic. Humans and machines are both accountable on the audit trail.

**Local only for now** — no GitHub remote until you say so.  
**License:** Apache-2.0 (prepared for open release).

---

## Quick start

```bash
cd ~/projects/northstar

npm run conformance      # TL-PX 0.1 Minimum Profile pass/fail
npm test                 # unit + switchboard suite
npm run demo             # three trust-chain scenarios
npm run demo:switchboard # whitelist + credibility + routes
```

```bash
node bin/glass.mjs evaluate examples/intent-pii-email.json config/policy.yaml --log var/audit.jsonl
node bin/glass.mjs approve  <receipt_id> --operator human.ops.alex --log var/audit.jsonl
node bin/glass.mjs execute  <receipt_id> --executor runtime.mailer --status EXECUTED --log var/audit.jsonl
node bin/glass.mjs incident <receipt_id> --log var/audit.jsonl --why "What went wrong"
node bin/glass.mjs chain    <receipt_id> --log var/audit.jsonl
```

Zero runtime npm dependencies. Node 18+.

---

## Open-source layers (recommended)

| Layer | Open? | Role |
| --- | --- | --- |
| **Prism** | Yes (existing) | Metadata-only intent signal |
| **TL-PX Minimum** | Yes (this) | Spec + schemas + conformance + reference gate |
| **APEX-Lite** | Yes (existing) | Early concept playground |
| **Glass enterprise** | Your call | Tokens, multi-tenant, ops UI, advanced policy — *extends* TL-PX |

“Open to an extent” = ship a **minimum everyone can implement**, keep differentiated enterprise value optional on top.

---

## Standard documents

| Path | Contents |
| --- | --- |
| [`docs/standard/SPEC-v0.1.md`](docs/standard/SPEC-v0.1.md) | Normative MUST/SHOULD minimum profile |
| [`docs/standard/README.md`](docs/standard/README.md) | Standard overview |
| [`schemas/tlpx-0.1/`](schemas/tlpx-0.1/) | JSON Schema for records |
| `npm run conformance` | Automated conformance suite |

### Minimum contract (one screen)

```text
Agent/Machine
  → Switchboard (whitelist + credibility 0–0.99 + approval route)
  → Evaluation Request
  → Decision Record   ALLOW | REQUIRE_APPROVAL | DENY (switchboard hard gate)
  → Operator Action   APPROVE | REJECT   (when escalated, via approval_route)
  → Execution Record  EXECUTED | BLOCKED | FAILED
  → Audit chain + optional Accountability Report

Parties: declarer | router (switchboard) | evaluator | authorizer | executor
```

See [`docs/switchboard.md`](docs/switchboard.md) and `config/switchboard.json`.

**Hard rule for conforming executors:** never emit `EXECUTED` unless authorization is `AUTHORIZED`.

**Air-gapped mode (default):** audit log required; execution auth is derived only from the audit chain; single operator outcome; use `executeAuthorized` so side effects never run without AUTHORIZED. See [`docs/airgap.md`](docs/airgap.md).

---

## For real-world adopters

Integrate as an **admission controller** for agent tools / sensitive actions:

1. Build an Evaluation Request from the pending action.  
2. Call `evaluate` (or your TL-PX-conforming service).  
3. If `PENDING_HUMAN_APPROVAL`, route to your ops queue.  
4. Only perform the side effect when `AUTHORIZED`.  
5. Append an Execution Record (success, block, or failure).  
6. On incidents, run accountability over the receipt chain.

Alternate languages should: implement the **spec**, emit the **record shapes**, and port the **conformance fixtures** — not necessarily this JS runtime.

---

## Repository layout

```text
docs/standard/     TL-PX specification
schemas/tlpx-0.1/  JSON schemas
src/               Reference implementation
bin/glass.mjs      CLI
config/policy.yaml Demo policy pack
examples/          Fixtures for humans + machines
scripts/
  conformance.mjs  Standard pass/fail
  test.mjs         Extra tests
  demo.mjs         Narrative demo
```

---

## Related

- Site: https://trust-layer-ai.github.io/Trust-Layer-AI/
- Prism: https://github.com/Trust-Layer-AI/prism-protocol
- APEX-Lite / Trust-Engine: https://github.com/Trust-Layer-AI/Trust-Engine
- Local early reference: `~/APEX-Lite`

---

## Status

- [x] Minimum Profile draft (v0.1)
- [x] Schemas + validators
- [x] Conformance suite
- [x] Reference implementation (local)
- [ ] Public repo release (when you choose)
- [ ] Language SDKs / HTTP profile
- [ ] Signed authorization extension profile
