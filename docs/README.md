# Northstar / TL-PX — Documentation Suite

**Project codename:** Northstar  
**Standard:** TL-PX v0.1 frozen; v0.2 accepted evaluation/authorization schema core through 2.3 plus accepted 2.3d local-authority scope, unaccepted local 2.4/3.1 commits, and an uncommitted 3.2 typed-action/hash candidate. Execution-side evidence schema closure remains deferred. Gate still emits 0.1.
**Reference implementation:** Glass / `tlpx-reference`  
**License:** Apache-2.0  
**Status:** Local development (not published to a remote by default)

This is the full documentation hub for the Trust Layer air-gapped pre-execution checkpoint: intent before action, Switchboard identity routing, deterministic policy, human escalation, fail-closed execution, and dual human/machine accountability.

---

## Start here

| Audience | Route |
| --- | --- |
| **Human newcomer** | [../README.md](../README.md) → **[SHARE.md](./SHARE.md)** |
| **Implementer** | [../README.md](../README.md) → **this hub** → the relevant guide below |
| **AI collaborator** | [../NORTHSTAR-SESSION-START.md](../NORTHSTAR-SESSION-START.md) |
| **Private continuity** | local `skills.md` (gitignored; not a specification) |

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
| [standard/SPEC-v0.2.md](./standard/SPEC-v0.2.md) | Draft 0.2 contract (decisions, errors, states, policy provenance/precedence, JCS hashes, schemas) |
| [../schemas/tlpx-0.1/](../schemas/tlpx-0.1/) | Frozen 0.1 JSON Schemas |
| [../schemas/tlpx-0.2/](../schemas/tlpx-0.2/) | Slice 2.3 record/object schemas + reason codes |

### Current build planning and reviews

| Document | Description |
| --- | --- |
| [BUILD-SPEC-SHEET.md](./BUILD-SPEC-SHEET.md) | Current hardened build baseline; 2.3d/Rust authority at `aed80e2` independently accepted |
| [../tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md](../tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md) | Named-commit builder evidence, environment, passing matrix, and explicit limits |
| [../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md](../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md) | Independent exact-commit review, rerun, scope, and acceptance disposition |
| [../tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md](../tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md) | Builder verification and limits for named commit `c9bdd0f` (not independently accepted) |
| [../tests/reports/slice-2.4-policy-ordering-builder-verification-2026-08-17.md](../tests/reports/slice-2.4-policy-ordering-builder-verification-2026-08-17.md) | Pre-commit builder matrix, negative cases, and limits for local slice 2.4 commit `a87f822` |
| [../tests/reports/slice-3.1-rust-policy-activation-builder-verification-2026-08-17.md](../tests/reports/slice-3.1-rust-policy-activation-builder-verification-2026-08-17.md) | Pre-commit Rust activation matrix, provenance negatives, and limits for local slice 3.1 commit `1addb5c` |
| [../tests/reports/slice-3.2-typed-action-hash-builder-verification-2026-08-17.md](../tests/reports/slice-3.2-typed-action-hash-builder-verification-2026-08-17.md) | Bounded typed-object/hash parity checks and limits for the uncommitted slice 3.2 candidate |
| [../tests/fixtures/tlpx-0.2/jcs/](../tests/fixtures/tlpx-0.2/jcs/) | Slice 2.2 JCS / `sha256:` golden fixtures |
| [../tests/fixtures/tlpx-0.2/policy/](../tests/fixtures/tlpx-0.2/policy/) | Slice 2.4 candidate policy-manifest/hash fixture |
| [../tests/fixtures/tlpx-0.2/actions/](../tests/fixtures/tlpx-0.2/actions/) | Slice 3.2 candidate typed actions, Action Binding, canonical bytes, and distinct hashes |
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

npm test                    # unit + switchboard + air-gap + policy compile + 0.2 JCS/schema
npm run conformance         # frozen TL-PX 0.1 suite (47 fixtures)
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
