# Trust Layer Pre-Execution Standard (TL-PX)

**Minimum Profile** — the open, testable contract for real-world adopters.

TL-PX **0.1** is frozen historical evidence. **[SPEC-v0.2.md](./SPEC-v0.2.md)** is the draft 0.2 contract with an accepted evaluation/authorization schema core through 2.3 and unaccepted local implementation candidates through exact-commit builder-verified cooperative slice 3.8 `7c41450`. Execution-receipt, cancellation/reconciliation, and revocation-evidence schema closure remains deferred. Implementation-driven 2.3d is independently accepted at exact commit `aed80e2`. The JavaScript gate still implements 0.1 and hosts the 0.2 schema/policy/typed-action/JCS/hash oracle.

## Why a standard (not only a demo)

APEX-Lite showed the **concept**: declare intent, evaluate policy, escalate when needed.

What production teams need next is a **minimum standard**:

| Deliverable | Purpose |
| --- | --- |
| Normative spec | Shared MUST/SHOULD language |
| JSON schemas | Stable record shapes |
| Conformance suite | Pass/fail for any implementation |
| Reference implementation | One clear, working baseline |

Enterprise product features (Glass full suite: signed tokens, multi-tenant ops, quorum, etc.) can **extend** this profile without forking the minimum semantics.

## Start here

1. Project docs hub: **[../README.md](../README.md)**  
2. Read **[SPEC-v0.1.md](./SPEC-v0.1.md)** (frozen 0.1) and **[SPEC-v0.2.md](./SPEC-v0.2.md)** (draft 0.2 with accepted 2.3 core, accepted 2.3d scope, and unaccepted local 2.4/3.x candidates)
3. Inspect schemas in `../../schemas/tlpx-0.1/` (frozen) and `../../schemas/tlpx-0.2/` (draft).
4. JCS/hash golden fixtures: `../../tests/fixtures/tlpx-0.2/jcs/`  
5. Run the **0.1** suite and the **0.2 schema/JCS** checks:

```bash
cd ../../implementations/javascript
npm run conformance
npm test
npm run test:jcs
npm run demo
```

## Open-source posture

- **Intended license:** Apache-2.0 (see `LICENSE` in repo root) — same family as Prism / APEX-Lite.
- **Publish deliberately:** a baseline remote exists, while post-baseline work remains local until explicitly pushed.
- **“Open to an extent”:** Minimum Profile + reference + conformance = open core. Broader Glass enterprise layers can remain product-differentiated.

## Stack relative to Prism / APEX

```text
Prism          → optional intent signal dialect
TL-PX 0.1      → frozen historical minimum (47 fixtures)  
TL-PX 0.2      → draft contract; accepted 2.3/2.3d scope plus unaccepted local 2.4/3.x candidates
APEX-Lite      → early playable reference (concept)
This reference → conforming TL-PX 0.1 implementation + 0.2 contract oracles
Glass product  → enterprise extensions on top of TL-PX
```
