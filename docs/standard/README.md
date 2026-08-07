# Trust Layer Pre-Execution Standard (TL-PX)

**Minimum Profile** — the open, testable contract for real-world adopters.

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
2. Read **[SPEC-v0.1.md](./SPEC-v0.1.md)** (normative)  
3. Inspect schemas in `../../schemas/tlpx-0.1/`  
4. Run:

```bash
cd ../../implementations/javascript
npm run conformance
npm test
npm run demo
```

## Open-source posture

- **Intended license:** Apache-2.0 (see `LICENSE` in repo root) — same family as Prism / APEX-Lite.
- **Publish when ready:** repo is local-only until you choose a public remote.
- **“Open to an extent”:** Minimum Profile + reference + conformance = open core. Broader Glass enterprise layers can remain product-differentiated.

## Stack relative to Prism / APEX

```text
Prism          → optional intent signal dialect
TL-PX 0.1      → minimum gate + records + parties + audit contract  ← you are here
APEX-Lite      → early playable reference (concept)
This reference → conforming TL-PX implementation
Glass product  → enterprise extensions on top of TL-PX
```
