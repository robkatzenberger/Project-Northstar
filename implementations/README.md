# Implementations

Language-specific reference implementations of **TL-PX** (Trust Layer Pre-Execution) and related product pieces (Switchboard, Glass gate, executor).

**Language-agnostic (repo root):**

| Path | Role |
| --- | --- |
| [`../docs/`](../docs/) | Documentation suite |
| [`../schemas/tlpx-0.1/`](../schemas/tlpx-0.1/) | JSON schemas (shared contract) |
| [`../LICENSE`](../LICENSE) | Apache-2.0 |

**Rule:** new languages implement the same records and semantics from the [SPEC](../docs/standard/SPEC-v0.1.md). They do not redefine Prism core fields casually.

---

## Languages

| Folder | Status | Notes |
| --- | --- | --- |
| [`javascript/`](./javascript/) | **Active reference** | Node 18+ ES modules; CLI, sealed audit, tests, tech test #1 |
| [`go/`](./go/) | **Active secondary** | Switchboard + sealed audit + ops + HTTP `tlpxd` |
| [`java/`](./java/) | **Skeleton+** | Policy + Switchboard evaluate (Maven) |
| [`rust/`](./rust/) | Placeholder | Hardened PEP / high-assurance runtime |
| [`python/`](./python/) | Placeholder | Prototyping / data-platform adapters |

---

## Adding a language

1. Create `implementations/<lang>/` with a `README.md` stating status and how to build/test.
2. Implement evaluate → (operator) → chain-verified execute against shared schemas.
3. Prefer a conformance harness that mirrors `javascript/scripts/conformance.mjs` fixtures in `javascript/examples/`.
4. Link from this file and the root `README.md`.

Do **not** put language-specific code under `docs/` or `schemas/`.
