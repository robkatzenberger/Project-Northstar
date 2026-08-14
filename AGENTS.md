# AGENTS.md — Northstar monorepo

## Layout

- **Docs & schemas** — repo root (`docs/`, `schemas/`). Language-agnostic.
- **Implementations** — `implementations/<lang>/`.
- **Current reference code** — `implementations/javascript/` (Node ES modules).
- **Planned authoritative core** — Rust, after the TL-PX 0.2 contract and fixtures are defined.

Before substantive work, read `NORTHSTAR-SESSION-START.md`, `docs/BUILD-SPEC-SHEET.md`, and `docs/reviews/build-plan-review-disposition-2026-08-13.md`. Live implementation scope is slices 1.1–1.2 only. Correct current v0.1 reference defects under `implementations/javascript/`; do not expand JS into the planned production authority. Build new Rust authority code only from accepted TL-PX 0.2 contracts and conformance fixtures. The August 7 `AUTHORIZED` snapshot token is abandoned.

## Commands (JS)

```bash
cd implementations/javascript
npm test
npm run conformance
npm run tech-test
```

## Non-negotiables

Pre-execution, deterministic fail-closed policy, Prism metadata-only, Switchboard-first DENY, exact-action binding, authenticated single-use atomic authorization, distinct authorization/execution evidence, and no secrets in git.

Preserve TL-PX 0.1 as historical evidence. Breaking hardened changes target a separately versioned TL-PX 0.2 spec/schema/conformance line.

Full product rules: `implementations/javascript/AGENTS.md`  
Fresh-context instructions: `NORTHSTAR-SESSION-START.md`

Hardened build baseline: `docs/BUILD-SPEC-SHEET.md`

Docs hub: `docs/README.md`
