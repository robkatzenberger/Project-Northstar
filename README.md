# Northstar — Trust Layer Pre-Execution (TL-PX)

**Codename:** Northstar  
**What this is:** An **open minimum standard** + **air-gapped reference implementation** for human + machine trust checkpoints *before* execution.

> Intent is declared before action.  
> Switchboard identifies who may act.  
> Glass decides. Humans escalate when needed.  
> Side effects run only when the audit chain says AUTHORIZED.

**Local only for now** — no GitHub remote until you choose.  
**License:** Apache-2.0  

---

## Documentation suite

**Start here → [`docs/README.md`](docs/README.md)**

| Guide | Description |
| --- | --- |
| [Getting Started](docs/getting-started.md) | Install, first evaluate, demos |
| [Concepts](docs/concepts.md) | Mental model |
| [Architecture](docs/architecture.md) | Layers and modules |
| [API Reference](docs/api-reference.md) | JavaScript library |
| [CLI Reference](docs/cli-reference.md) | `bin/glass.mjs` |
| [Configuration](docs/configuration.md) | Policy + Switchboard |
| [Switchboard](docs/switchboard.md) | Identity, whitelist, credibility |
| [Air-Gapped Operation](docs/airgap.md) | Chain auth, fail-closed executor |
| [Security Model](docs/security.md) | Threats and residual risks |
| [Testing](docs/testing.md) | Conformance, tech test, red team |
| [Integration](docs/integration.md) | Wire real runtimes |
| [TL-PX Spec v0.1](docs/standard/SPEC-v0.1.md) | Normative minimum standard |
| [Glossary](docs/glossary.md) | Terms |
| [Changelog](docs/changelog.md) | History |

---

## Quick start

```bash
cd ~/projects/northstar

npm test                    # unit + switchboard + air-gap
npm run conformance         # TL-PX 0.1
node scripts/tech-test.mjs  # formal technical test #1 (PASS)
npm run demo
npm run demo:switchboard
```

```bash
# Air-gapped CLI (audit required)
node bin/glass.mjs evaluate examples/intent-safe.json config/policy.yaml \
  --log var/tech-test-audit.jsonl

node bin/glass.mjs auth  <receipt_id> --log var/tech-test-audit.jsonl
node bin/glass.mjs chain <receipt_id> --log var/tech-test-audit.jsonl
```

Zero runtime npm dependencies. **Node 18+**.

Canonical audit path: **`var/tech-test-audit.jsonl`**

---

## One-screen flow

```text
Agent/Machine
  → Switchboard (whitelist + credibility 0–0.99 + approval route)
  → Policy gate (ALLOW | REQUIRE_APPROVAL)
  → Human APPROVE/REJECT if escalated
  → executeAuthorized (fail-closed)
  → Append-only JSONL audit + accountability
```

Switchboard **DENY** is automatic and happens **before** policy rules.

---

## Open-source layers

| Layer | Open? | Role |
| --- | --- | --- |
| **Prism** | Yes | Metadata-only intent signal |
| **TL-PX Minimum** | Yes (this) | Spec + schemas + conformance + reference |
| **APEX-Lite** | Yes | Early concept playground |
| **Glass enterprise** | Your call | Tokens, multi-tenant, ops UI — *extends* TL-PX |

---

## Repository layout

```text
docs/                 Full documentation suite
docs/standard/        TL-PX normative spec
schemas/tlpx-0.1/     JSON schemas
src/                  Reference implementation
bin/glass.mjs         CLI
config/               policy.yaml + switchboard.json
examples/             Intent fixtures
scripts/              tests, demos, tech-test, red-team
var/                  Local audit logs (gitignored)
```

---

## Status

- [x] TL-PX 0.1 draft + schemas  
- [x] Switchboard + air-gap hardening  
- [x] Conformance suite  
- [x] Technical test #1 **PASS** (29/29)  
- [x] Documentation suite  
- [ ] Public remote release (when you choose)  
- [ ] HTTP profile / signed receipts / operator authN  

---

## Related

- [Trust Layer AI site](https://trust-layer-ai.github.io/Trust-Layer-AI/)  
- [Prism](https://github.com/Trust-Layer-AI/prism-protocol)  
- [Trust-Engine](https://github.com/Trust-Layer-AI/Trust-Engine)  
- Local early ref: `~/APEX-Lite`  

For agents working in this repo, see [`AGENTS.md`](AGENTS.md).
