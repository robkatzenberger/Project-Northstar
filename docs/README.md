# Northstar / TL-PX — Documentation Suite

**Project codename:** Northstar  
**Standard:** Trust Layer Pre-Execution Minimum Standard (**TL-PX**) v0.1  
**Reference implementation:** Glass / `tlpx-reference`  
**License:** Apache-2.0  
**Status:** Local development (not published to a remote by default)

This is the full documentation hub for the Trust Layer air-gapped pre-execution checkpoint: intent before action, Switchboard identity routing, deterministic policy, human escalation, fail-closed execution, and dual human/machine accountability.

---

## Start here

| Audience | Read first |
| --- | --- |
| New to the project | [Getting Started](./getting-started.md) |
| Want the big idea | [Vision](./vision.md) · [Concepts](./concepts.md) |
| Implementers / integrators | [Architecture](./architecture.md) · [API Reference](./api-reference.md) · [Integration Guide](./integration.md) |
| Operators | [CLI Reference](./cli-reference.md) · [Configuration](./configuration.md) · [Logging](./logging.md) |
| Security / review | [Security Model](./security.md) · [Air-Gapped Operation](./airgap.md) |
| Standards / interop | [TL-PX Spec v0.1](./standard/SPEC-v0.1.md) · [Standard overview](./standard/README.md) |
| QA | [Testing](./testing.md) |
| Vocabulary | [Glossary](./glossary.md) |

---

## Document map

### Product & concepts

| Document | Description |
| --- | --- |
| [vision.md](./vision.md) | Problem, product thesis, non-goals, success metrics |
| [concepts.md](./concepts.md) | Core mental model: Prism, Switchboard, Glass, parties, chain |
| [architecture.md](./architecture.md) | Layers, data flow, module map, inheritance from APEX-Lite |
| [glossary.md](./glossary.md) | Terms and enums |

### Standards

| Document | Description |
| --- | --- |
| [standard/README.md](./standard/README.md) | Why a minimum standard; OSS posture |
| [standard/SPEC-v0.1.md](./standard/SPEC-v0.1.md) | Normative MUST/SHOULD Minimum Profile |
| [../schemas/tlpx-0.1/](../schemas/tlpx-0.1/) | JSON Schemas for records |

### Runtime subsystems

| Document | Description |
| --- | --- |
| [switchboard.md](./switchboard.md) | Identity router: whitelist, credibility 0–0.99, approval routes |
| [airgap.md](./airgap.md) | Air-gapped mode: mandatory audit, chain auth, fail-closed executor |
| [logging.md](./logging.md) | Where audit JSONL lives; tech-test path |
| [security.md](./security.md) | Threat model, what is enforced, residual risks |

### How-to & reference

| Document | Description |
| --- | --- |
| [getting-started.md](./getting-started.md) | Install, first evaluate, first demo |
| [api-reference.md](./api-reference.md) | JavaScript library API |
| [cli-reference.md](./cli-reference.md) | `bin/glass.mjs` / `tlpx` commands |
| [configuration.md](./configuration.md) | Policy YAML, Switchboard JSON, examples |
| [integration.md](./integration.md) | Wiring real runtimes (air-gapped first; optional harness adapters) |
| [testing.md](./testing.md) | Unit, conformance, red-team, technical test #1 |
| [MVP.md](./MVP.md) | Historical MVP handoff brief |
| [changelog.md](./changelog.md) | Project changelog |

---

## One-screen architecture

```text
Agent / human / script
        │
        │  declared intent (Prism-compatible + Glass fields)
        ▼
┌───────────────────┐
│   SWITCHBOARD     │  first line: identity, whitelist, action scope, credibility
│   (router)        │  hard DENY if unknown / not whitelisted / action denied
└─────────┬─────────┘
          │ clears Switchboard
          ▼
┌───────────────────┐
│   POLICY GATE     │  deterministic rules → ALLOW | REQUIRE_APPROVAL
│   (Glass / TL-PX) │
└─────────┬─────────┘
          │
          ├─ ALLOW ──────────────────────────────┐
          ├─ REQUIRE_APPROVAL → human operator ──┤
          └─ DENY (Switchboard) ─────────────────┤
                                                 ▼
                              authorization from AUDIT CHAIN only
                                                 │
                                                 ▼
                              executeAuthorized (fail-closed)
                                                 │
                                                 ▼
                              side effect  OR  BLOCKED
                                                 │
                                                 ▼
                              append-only JSONL audit + accountability
```

---

## Quick commands

```bash
cd ~/projects/northstar

npm test                    # unit + switchboard + air-gap
npm run conformance         # TL-PX 0.1 suite
node scripts/tech-test.mjs  # formal technical test #1
npm run demo
npm run demo:switchboard

node bin/glass.mjs help
```

Canonical technical-test audit log:

```text
var/tech-test-audit.jsonl
```

---

## Related ecosystem

| Asset | Role |
| --- | --- |
| [Trust Layer AI site](https://trust-layer-ai.github.io/Trust-Layer-AI/) | Public product narrative |
| [Prism Protocol](https://github.com/Trust-Layer-AI/prism-protocol) | Open intent signal |
| [Trust-Engine / APEX-Lite](https://github.com/Trust-Layer-AI/Trust-Engine) | Early OSS reference gate |
| Local `~/APEX-Lite` | Early local reference checkout |

---

## Contributing to docs

- Prefer clear MUST/SHOULD language only in the normative SPEC.
- Keep security claims honest: distinguish **enforced** vs **deployment-dependent**.
- Update [changelog.md](./changelog.md) when behavior changes.
- Keep examples runnable from the repo root with Node 18+.
