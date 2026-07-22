# Northstar — Glass product workspace

**Codename:** `northstar`  
**Product surface:** **Glass** (enterprise verification & enforcement)  
**Ecosystem:** [Trust Layer AI](https://trust-layer-ai.github.io/Trust-Layer-AI/)

Northstar is the working home for **Glass**: the checkpoint that receives Prism intent signals, evaluates policy / identity / risk, applies human-in-the-loop controls, issues authorization, and writes tamper-evident audit before execution.

This repo is intentionally a **workspace + planning shell**. Runtime product code lands after we lock scope against Prism and APEX-Lite.

---

## Trust Layer map

| Layer | Name | Role | Public home |
| --- | --- | --- | --- |
| Signal | **Prism** | Open, metadata-only pre-execution intent signal. Describes intent; does not judge. | [prism-protocol](https://github.com/Trust-Layer-AI/prism-protocol) |
| Gate (OSS ref) | **APEX-Lite / Trust-Engine** | Minimal deterministic policy boundary: `ALLOW` / `REQUIRE_APPROVAL`, receipts, local console. | [Trust-Engine](https://github.com/Trust-Layer-AI/Trust-Engine) |
| Gate (product) | **Glass** ← *this workspace* | Enterprise verification: policy, risk, HITL, auth tokens, revocation, audit, ops. | *Northstar* |
| Narrative | **Trust Layer AI** | Product story + architecture site. | [site](https://trust-layer-ai.github.io/Trust-Layer-AI/) · [repo](https://github.com/Trust-Layer-AI/Trust-Layer-AI) |

Local reference on this machine (earlier APEX work):

- `~/APEX-Lite` — local APEX-Lite checkout / experiments

---

## Canonical flow

```text
Agent proposes action
  → Prism emits intent metadata
  → Glass receives signal
  → Policy engine: identity, risk, scope, authorization
  → Human approval if required
  → Signed authorization token (product goal)
  → Execution proceeds
  → Tamper-evident audit log
```

**Principle:** verification *before* execution. Agent-agnostic, model-agnostic, vendor-neutral. Deterministic rules — not in-model guardrails.

---

## Repo layout

```text
northstar/
  AGENTS.md          # conventions for humans + coding agents
  README.md          # this file
  docs/
    vision.md        # product north star and non-goals
    architecture.md  # target Glass shape vs APEX-Lite
  config/            # future policy / gate config samples
  examples/          # future Prism signals + decisions
  src/               # empty until first implementation slice
```

---

## Related local paths

| Path | Notes |
| --- | --- |
| `~/APEX-Lite` | Prior APEX-Lite reference work |
| `~/projects/northstar` | This Glass product workspace |

---

## Status

- [x] Workspace created
- [x] Git initialized
- [x] Vision + architecture notes
- [x] Agent conventions (`AGENTS.md`)
- [ ] Product MVP scope locked
- [ ] First implementation slice (TBD)

---

## License

TBD — ecosystem public refs are Apache-2.0; product licensing for Glass is a separate decision.
