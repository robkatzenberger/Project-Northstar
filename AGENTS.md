# AGENTS.md — Northstar monorepo

## Layout

- **Docs & schemas** — repo root (`docs/`, `schemas/`). Language-agnostic.
- **Implementations** — `implementations/<lang>/`.
- **Current reference code** — `implementations/javascript/` (Node ES modules).
- **Planned authoritative core** — Rust, after the TL-PX 0.2 contract and fixtures are defined.

Before substantive work, read `NORTHSTAR-SESSION-START.md`, `docs/BUILD-SPEC-SHEET.md`, and `docs/reviews/build-plan-review-disposition-2026-08-13.md`. Live scope is slice 2.1: freeze TL-PX 0.1 and draft the 0.2 decision/error/state contract. Do not start 2.2 hashing fixtures, 2.3 schemas/validators, or Rust. Correct remaining v0.1 reference defects under `implementations/javascript/` without expanding JS into the planned production authority. The August 7 `AUTHORIZED` snapshot token is abandoned.

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
