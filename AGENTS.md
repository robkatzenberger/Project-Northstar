# AGENTS.md — Northstar monorepo

## Layout

- **Docs & schemas** — repo root (`docs/`, `schemas/`). Language-agnostic.
- **Implementations** — `implementations/<lang>/`.
- **Current reference code** — `implementations/javascript/` (Node ES modules).
- **Authoritative core (MVP)** — `implementations/rust` (`tlpx`): accepted 0.2 types/hashes plus durable local evaluate, issuance, and atomic claim; named commit `c9bdd0f` adds builder-verified bounded schema evidence and a sealed outbox. Later candidates remain unaccepted. Independent Phase 4 review of target `64d0820` returned changes requested; exact aggregate candidate `4ca86d6` passes the Rust builder matrix and bounded macOS 3.9 gate but still requires independent re-review.

Before substantive work, read `NORTHSTAR-SESSION-START.md`, `docs/BUILD-SPEC-SHEET.md`, and the newest applicable report under `tests/reports/`. The 2.3 evaluation/authorization schema core and bounded 2.3d/Rust commit `aed80e2` are accepted; later slices are not. Slice 3.9's dedicated-identity administrator gate passed locally on macOS for exact candidate `4ca86d6`; this is builder evidence, not independent acceptance. Phase 4 builder evidence remains point-in-time evidence, and independent review returned changes requested. Do not imply Section 3/Phase 4 acceptance, production readiness, portable handoff, or universal forced mediation. JavaScript and Go live permission paths are historical/cooperative 0.1-era references, not 0.2 authorities or PEPs. The August 7 `AUTHORIZED` snapshot token is abandoned.

## Commands (JS)

```bash
cd implementations/javascript
npm test
npm run test:jcs
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
