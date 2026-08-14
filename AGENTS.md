# AGENTS.md — Northstar monorepo

## Layout

- **Docs & schemas** — repo root (`docs/`, `schemas/`). Language-agnostic.
- **Implementations** — `implementations/<lang>/`.
- **Current reference code** — `implementations/javascript/` (Node ES modules).
- **Authoritative core (MVP)** — `implementations/rust` (`tlpx`): 0.2 types/hashes plus durable local evaluate, issuance, and atomic claim. Not a PEP yet.

Before substantive work, read `NORTHSTAR-SESSION-START.md`, `docs/BUILD-SPEC-SHEET.md`, and `docs/reviews/build-plan-review-disposition-2026-08-13.md`. Slices 1.1–2.3 are the accepted contract line. Rust `tlpx` has a local authority MVP; its caller-supplied authenticated-context strings are trusted-embedding inputs, not an authentication mechanism. It has no execution receipt or PEP. The JS gate remains a TL-PX 0.1 reference plus Phase 1 compile and a 0.2 schema/JCS/hash oracle. Do not expand JS into the planned production authority. The August 7 `AUTHORIZED` snapshot token is abandoned.

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
