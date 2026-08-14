# Implementations

Language-specific reference implementations of **TL-PX** (Trust Layer Pre-Execution) and related product pieces (Switchboard, Glass gate, executor).

**Language-agnostic (repo root):**

| Path | Role |
| --- | --- |
| [`../docs/`](../docs/) | Documentation suite |
| [`../schemas/tlpx-0.1/`](../schemas/tlpx-0.1/) | Frozen 0.1 JSON schemas |
| [`../tests/fixtures/tlpx-0.2/jcs/`](../tests/fixtures/tlpx-0.2/jcs/) | Accepted 0.2 JCS / hash golden fixtures |
| [`../docs/BUILD-SPEC-SHEET.md`](../docs/BUILD-SPEC-SHEET.md) | Proposed hardened baseline for a separately versioned TL-PX 0.2 |
| [`../LICENSE`](../LICENSE) | Apache-2.0 |

**Rule:** current v0.1 implementations follow the frozen [SPEC v0.1](../docs/standard/SPEC-v0.1.md). Hardened breaking changes must target a separate v0.2 spec/schema/conformance line; do not silently backport them into v0.1 or redefine Prism core fields casually.

---

## Languages

| Folder | Status | Notes |
| --- | --- | --- |
| [`javascript/`](./javascript/) | **Active 0.1 reference + 0.2 hash oracle** | Node 18+ ES modules; CLI, sealed audit, Phase 1 compile, JCS fixtures |
| [`go/`](./go/) | **Active secondary** | Switchboard + sealed audit + ops + HTTP `tlpxd` |
| [`java/`](./java/) | **Skeleton+** | Policy + Switchboard evaluate (Maven) |
| [`rust/`](./rust/) | **Planned authority** | Small TL-PX 0.2 authorization core + hardened PEP; not started |
| [`python/`](./python/) | Placeholder | Prototyping / data-platform adapters |

---

## Adding a language

1. Create `implementations/<lang>/` with a `README.md` stating status and how to build/test.
2. Implement evaluate → (operator) → chain-verified execute against shared schemas.
3. Prefer a conformance harness that mirrors `javascript/scripts/conformance.mjs` fixtures in `javascript/examples/`.
4. Link from this file and the root `README.md`.

Do **not** put language-specific code under `docs/` or `schemas/`.

For TL-PX 0.2, JavaScript/TypeScript remains the readable reference and conformance oracle. Rust is the planned authoritative security core. Java, Go, Python, and TypeScript integrations should be SDKs/adapters rather than independently drifting authorization authorities.
