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
| [`../tests/reports/slice-3.3-local-auth-cancellation-builder-verification-2026-08-18.md`](../tests/reports/slice-3.3-local-auth-cancellation-builder-verification-2026-08-18.md) | Full exact-commit builder matrix and limits for local slice 3.3 commit `c19b1d2` |
| [`../tests/reports/slice-3.4-approval-expiry-builder-verification-2026-08-18.md`](../tests/reports/slice-3.4-approval-expiry-builder-verification-2026-08-18.md) | Full exact-commit builder matrix and limits for local slice 3.4 commit `133cd94` |
| [`../tests/reports/slice-3.8-cooperative-shell-runner-builder-verification-2026-08-20.md`](../tests/reports/slice-3.8-cooperative-shell-runner-builder-verification-2026-08-20.md) | Full exact-commit builder matrix and limits for local cooperative slice 3.8 commit `7c41450` |
| [`../tests/reports/slice-3.9-restricted-pep-candidate-unverified-2026-08-27.md`](../tests/reports/slice-3.9-restricted-pep-candidate-unverified-2026-08-27.md) | Historical pre-gate report for candidate `e6f2bb0`; superseded for current status |
| [`../tests/reports/slice-3.9-administrator-gate-2026-09-04-064717.md`](../tests/reports/slice-3.9-administrator-gate-2026-09-04-064717.md) | Source `f025332`; bounded macOS administrator gate preserved by evidence commit `32c049e`, with a later deadline finding still blocking acceptance |
| [`../tests/reports/slice-4.1-separated-key-roles-builder-verification-2026-08-27.md`](../tests/reports/slice-4.1-separated-key-roles-builder-verification-2026-08-27.md) | Exact-commit builder/red-team evidence for separated role keys through `efa7f0f` |
| [`../tests/reports/slice-4.2-durable-audit-export-builder-verification-2026-08-27.md`](../tests/reports/slice-4.2-durable-audit-export-builder-verification-2026-08-27.md) | Exact-commit builder/red-team evidence for bounded durable local audit export at `833d8d4` |
| [`../tests/reports/slice-4.3-race-time-crash-assurance-builder-verification-2026-08-27.md`](../tests/reports/slice-4.3-race-time-crash-assurance-builder-verification-2026-08-27.md) | Exact-commit builder/red-team evidence for bounded time/race/restart assurance at `eb0e624` |
| [`../tests/reports/slice-4.4-operational-readiness-builder-verification-2026-08-27.md`](../tests/reports/slice-4.4-operational-readiness-builder-verification-2026-08-27.md) | Exact-commit builder/red-team evidence for bounded local operations/incident readiness at `bdd8a00` |
| [`../tests/reports/slice-4.5-multi-agent-handoff-builder-verification-2026-08-27.md`](../tests/reports/slice-4.5-multi-agent-handoff-builder-verification-2026-08-27.md) | Exact-commit builder/red-team evidence for authenticated non-transitive handoff at `536111a` |
| [`../tests/reports/slice-4.6-external-security-review-readiness-builder-verification-2026-08-27.md`](../tests/reports/slice-4.6-external-security-review-readiness-builder-verification-2026-08-27.md) | Point-in-time builder evidence for package `d51e46c`; independent review returned changes requested |
| [`../tests/reports/phase-4-remediation-builder-verification-2026-08-28.md`](../tests/reports/phase-4-remediation-builder-verification-2026-08-28.md) | Historical exact-commit builder evidence for `ee720d4`; later source/evidence candidates and findings supersede it for current status |
| [`../LICENSE`](../LICENSE) | Apache-2.0 |

**Rule:** current v0.1 implementations follow the frozen [SPEC v0.1](../docs/standard/SPEC-v0.1.md). Hardened breaking changes must target a separate v0.2 spec/schema/conformance line; do not silently backport them into v0.1 or redefine Prism core fields casually.

---

## Languages

| Folder | Status | Notes |
| --- | --- | --- |
| [`javascript/`](./javascript/) | **Active 0.1 reference + 0.2 contract oracle** | Node 18+ ES modules; CLI, sealed audit, Phase 1 compile, schemas, policy/ordering/typed-action helpers, JCS fixtures |
| [`go/`](./go/) | **Historical cooperative secondary** | 0.1-era Switchboard/sealed-audit/HTTP reference; caller-supplied actor IDs; not a 0.2 authority, adapter endpoint, or PEP |
| [`java/`](./java/) | **Skeleton+** | Policy + Switchboard evaluate (Maven) |
| [`rust/`](./rust/) | **Accepted bounded MVP + unaccepted remediation** | Commit `aed80e2` is the accepted boundary. Source `f025332` and evidence commit `32c049e` preserve bounded marker-gate report `064717`; a later pre-lock deadline defect is remediated in runtime commit `77d77b8`. A clean exact-candidate full rerun, administrator evidence, and independent review remain required. Full Section 3, Phase 4 acceptance, universal forced mediation, and network-egress mediation remain unverified. |
| [`python/`](./python/) | Placeholder | Prototyping / data-platform adapters |

---

## Adding a language

1. Create `implementations/<lang>/` with a `README.md` stating status and how to build/test.
2. Implement evaluate → (operator) → chain-verified execute against shared schemas.
3. Prefer a conformance harness that mirrors `javascript/scripts/conformance.mjs` fixtures in `javascript/examples/`.
4. Link from this file and the root `README.md`.

Do **not** put language-specific code under `docs/` or `schemas/`.

For TL-PX 0.2, JavaScript/TypeScript remains the readable reference and conformance oracle. Rust is the planned authoritative security core. Java, Go, Python, and TypeScript integrations should be SDKs/adapters rather than independently drifting authorization authorities.
