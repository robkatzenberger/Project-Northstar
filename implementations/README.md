# Implementations

Language-specific reference implementations of **TL-PX** (Trust Layer Pre-Execution) and related product pieces (Switchboard, Glass gate, executor).

**Language-agnostic (repo root):**

| Path | Role |
| --- | --- |
| [`../docs/`](../docs/) | Documentation suite |
| [`../schemas/tlpx-0.1/`](../schemas/tlpx-0.1/) | Frozen 0.1 JSON schemas |
| [`../tests/fixtures/tlpx-0.2/jcs/`](../tests/fixtures/tlpx-0.2/jcs/) | Accepted 0.2 JCS / hash golden fixtures |
| [`../tests/fixtures/tlpx-0.2/actions/`](../tests/fixtures/tlpx-0.2/actions/) | Slice 3.2 candidate typed-action/hash and exact Action Binding fixtures |
| [`../docs/BUILD-SPEC-SHEET.md`](../docs/BUILD-SPEC-SHEET.md) | Current hardened baseline for a separately versioned TL-PX 0.2 |
| [`../tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md`](../tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md) | Named-commit Rust authority evidence and explicit limits |
| [`../tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md`](../tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md) | Builder evidence for named commit `c9bdd0f` (not independently accepted) |
| [`../tests/reports/slice-2.4-policy-ordering-builder-verification-2026-08-17.md`](../tests/reports/slice-2.4-policy-ordering-builder-verification-2026-08-17.md) | Pre-commit builder evidence and limits for local slice 2.4 commit `a87f822` |
| [`../tests/reports/slice-3.1-rust-policy-activation-builder-verification-2026-08-17.md`](../tests/reports/slice-3.1-rust-policy-activation-builder-verification-2026-08-17.md) | Pre-commit builder evidence and limits for local slice 3.1 commit `1addb5c` |
| [`../tests/reports/slice-3.2-typed-action-hash-builder-verification-2026-08-17.md`](../tests/reports/slice-3.2-typed-action-hash-builder-verification-2026-08-17.md) | Bounded exact-commit evidence and limits for local slice 3.2 commit `e835c4e` |
| [`../tests/reports/slice-3.3-local-auth-cancellation-builder-verification-2026-08-18.md`](../tests/reports/slice-3.3-local-auth-cancellation-builder-verification-2026-08-18.md) | Builder matrix and limits for the uncommitted slice 3.3 Unix peer-authentication and pending-cancellation candidate |
| [`../LICENSE`](../LICENSE) | Apache-2.0 |

**Rule:** current v0.1 implementations follow the frozen [SPEC v0.1](../docs/standard/SPEC-v0.1.md). Hardened breaking changes must target a separate v0.2 spec/schema/conformance line; do not silently backport them into v0.1 or redefine Prism core fields casually.

---

## Languages

| Folder | Status | Notes |
| --- | --- | --- |
| [`javascript/`](./javascript/) | **Active 0.1 reference + 0.2 contract oracle** | Node 18+ ES modules; CLI, sealed audit, Phase 1 compile, schemas, policy/ordering/typed-action helpers, JCS fixtures |
| [`go/`](./go/) | **Active secondary** | Switchboard + sealed audit + ops + HTTP `tlpxd` |
| [`java/`](./java/) | **Skeleton+** | Policy + Switchboard evaluate (Maven) |
| [`rust/`](./rust/) | **Local authority MVP** | Commit `aed80e2` independently accepted; later unaccepted commits add sealed evidence, policy activation, and typed-hash parity. The 3.3 working tree adds kernel-derived Unix peer roles and atomic pending cancellation with sealed operator evidence. Full Section 3 verification is deferred. No approval resolution, execution receipt, hardened service, or PEP yet. |
| [`python/`](./python/) | Placeholder | Prototyping / data-platform adapters |

---

## Adding a language

1. Create `implementations/<lang>/` with a `README.md` stating status and how to build/test.
2. Implement evaluate → (operator) → chain-verified execute against shared schemas.
3. Prefer a conformance harness that mirrors `javascript/scripts/conformance.mjs` fixtures in `javascript/examples/`.
4. Link from this file and the root `README.md`.

Do **not** put language-specific code under `docs/` or `schemas/`.

For TL-PX 0.2, JavaScript/TypeScript remains the readable reference and conformance oracle. Rust is the planned authoritative security core. Java, Go, Python, and TypeScript integrations should be SDKs/adapters rather than independently drifting authorization authorities.
