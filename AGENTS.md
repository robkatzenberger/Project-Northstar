# AGENTS.md — Northstar monorepo

## Layout

- **Docs & schemas** — repo root (`docs/`, `schemas/`). Language-agnostic.
- **Implementations** — `implementations/<lang>/`.
- **Current reference code** — `implementations/javascript/` (Node ES modules).
- **Authoritative core (MVP)** — `implementations/rust` (`tlpx`): accepted 0.2 types/hashes plus durable local evaluate, issuance, and atomic claim; named commit `c9bdd0f` adds builder-verified bounded schema evidence and a sealed outbox. Local commit `e6f2bb0` is an explicitly unverified 3.9 restricted-PEP candidate. Slice 4.1 implementation `980327d`, exact-verified through `efa7f0f`, adds separated role keys, rotation, authorization proofs, and key-revocation enforcement. Independent Phase 4 review is deferred.

Before substantive work, read `NORTHSTAR-SESSION-START.md`, `docs/BUILD-SPEC-SHEET.md`, and the newest applicable report under `tests/reports/`. The 2.3 evaluation/authorization schema core and bounded 2.3d/Rust commit `aed80e2` are accepted; later slices are not. Slice 3.9 candidate `e6f2bb0` still requires `sudo ./scripts/restricted-agent-acceptance.sh`. Slice 4.1 is exact-builder-verified through `efa7f0f`, with independent Phase 4 review deferred. Do not use later work to imply 3.9, Section 3, Phase 4, or forced-mediation acceptance. The JS gate remains a TL-PX 0.1 reference plus 0.2 contract oracles. Do not expand JS into the planned production authority. The August 7 `AUTHORIZED` snapshot token is abandoned.

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
