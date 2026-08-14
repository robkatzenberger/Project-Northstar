# Northstar / TL-PX — Documentation Suite

**Project codename:** Northstar  
**Standard:** Trust Layer Pre-Execution Minimum Standard (**TL-PX**) v0.1 frozen; v0.2 draft contract in progress  
**Reference implementation:** Glass / `tlpx-reference`  
**License:** Apache-2.0  
**Status:** Local development (not published to a remote by default)

This is the full documentation hub for the Trust Layer air-gapped pre-execution checkpoint: intent before action, Switchboard identity routing, deterministic policy, human escalation, fail-closed execution, and dual human/machine accountability.

---

## Start here

| Audience | Read first |
| --- | --- |
| **Share with anyone (plain language)** | **[SHARE.md](./SHARE.md)** — human-readable overview + diagrams |
| Resume project work / frontier model | [Fresh-context instructions](../NORTHSTAR-SESSION-START.md) · [Hardened build specification](./BUILD-SPEC-SHEET.md) · [2026-08-13 disposition](./reviews/build-plan-review-disposition-2026-08-13.md) |
| New to the project | [Getting Started](./getting-started.md) |
| Want the big idea | [Vision](./vision.md) · [Concepts](./concepts.md) |
| Implementers / integrators | [Architecture](./architecture.md) · [API Reference](./api-reference.md) · [Integration Guide](./integration.md) |
| Operators | [CLI Reference](./cli-reference.md) · [Configuration](./configuration.md) · [Logging](./logging.md) |
| Security / review | [Security Model](./security.md) · [Air-Gapped Operation](./airgap.md) |
| Standards / interop | [TL-PX Spec v0.1 (frozen)](./standard/SPEC-v0.1.md) · [TL-PX Spec v0.2 (draft)](./standard/SPEC-v0.2.md) · [Standard overview](./standard/README.md) |
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
| [standard/SPEC-v0.1.md](./standard/SPEC-v0.1.md) | Frozen TL-PX 0.1 Minimum Profile |
| [standard/SPEC-v0.2.md](./standard/SPEC-v0.2.md) | Draft 0.2 decision/error/state/compatibility contract (slice 2.1) |
| [../schemas/tlpx-0.1/](../schemas/tlpx-0.1/) | Frozen 0.1 JSON Schemas |
| [../schemas/tlpx-0.2/](../schemas/tlpx-0.2/) | Reserved for slice 2.3 |

### Current build planning and reviews

| Document | Description |
| --- | --- |
| [BUILD-SPEC-SHEET.md](./BUILD-SPEC-SHEET.md) | Current hardened build baseline; live scope is slice 2.1 |
| [reviews/build-spec-review-2026-08-11-model-2.md](./reviews/build-spec-review-2026-08-11-model-2.md) | Independent architecture/spec review and accepted dispositions |
| [reviews/build-plan-review-disposition-2026-08-13.md](./reviews/build-plan-review-disposition-2026-08-13.md) | Implementation-boundary disposition: Phase 1 only, abandoned snapshot token, OS-enforced 3.9 bar |
| [../tests/reports/northstar-two-agent-test-proof.md](../tests/reports/northstar-two-agent-test-proof.md) | Executed two-agent baseline evidence and confirmed defects |

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
| [http-api.md](./http-api.md) | Go `tlpxd` localhost control plane |
| [testing.md](./testing.md) | Unit, conformance, red-team, technical test #1 |
| [MVP.md](./MVP.md) | Historical MVP handoff brief |
| [changelog.md](./changelog.md) | Project changelog |
| [**roadmap/**](./roadmap/README.md) | **Ideas & next phases (H-M/M-M, PEP, tokens, handoff)** |

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
cd ~/projects/northstar/implementations/javascript

npm test                    # unit + switchboard + air-gap
npm run conformance         # TL-PX 0.1 suite
node scripts/tech-test.mjs  # formal technical test #1
npm run demo
npm run demo:switchboard

node bin/glass.mjs help
```

Canonical technical-test audit log (monorepo root):

```text
../../var/tech-test-audit.jsonl
# absolute: ~/projects/northstar/var/tech-test-audit.jsonl
```

Multi-language layout: [`../implementations/README.md`](../implementations/README.md)

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
