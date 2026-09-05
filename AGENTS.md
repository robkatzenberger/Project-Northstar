# AGENTS.md — Northstar monorepo

## Layout

- **Docs & schemas** — repo root (`docs/`, `schemas/`). Language-agnostic.
- **Implementations** — `implementations/<lang>/`.
- **Current reference code** — `implementations/javascript/` (Node ES modules).
- **Authoritative core (MVP)** — `implementations/rust` (`tlpx`): accepted 0.2 types/hashes plus durable local evaluate, issuance, and atomic claim; named commit `c9bdd0f` adds builder-verified bounded schema evidence and a sealed outbox. Later candidates remain unaccepted. Tested source candidate `f025332` and direct-child evidence commit `32c049e` record the bounded macOS 3.9 gate in report `064717`. A subsequent hard review found a pre-transaction production-time sampling defect; runtime commit `77d77b8` contains remediation, but a new exact-commit run and independent re-review are still required.

Before substantive work, read `NORTHSTAR-SESSION-START.md`, `docs/BUILD-SPEC-SHEET.md`, and the newest applicable report under `tests/reports/`. The 2.3 evaluation/authorization schema core and bounded 2.3d/Rust commit `aed80e2` are accepted; later slices are not. Report `064717` is builder evidence for source candidate `f025332`, not independent acceptance or evidence for later working-tree changes. Current remediation is committed at `77d77b8` and must be fully rerun (including the administrator gate), and independently re-reviewed. Do not imply Section 3/Phase 4 acceptance, production readiness, portable handoff, universal forced mediation, or an HTTP/network-egress PEP. JavaScript and Go live permission paths are historical/cooperative 0.1-era references, not 0.2 authorities or PEPs. The August 7 `AUTHORIZED` snapshot token is abandoned.

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
