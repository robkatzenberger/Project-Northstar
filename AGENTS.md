# AGENTS.md — Northstar monorepo

## Layout

- **Docs & schemas** — repo root (`docs/`, `schemas/`). Language-agnostic.
- **Implementations** — `implementations/<lang>/`.
- **Current reference code** — `implementations/javascript/` (Node ES modules).
- **Authoritative core (MVP)** — `implementations/rust` (`tlpx`): accepted 0.2 types/hashes plus durable local evaluate, issuance, and atomic claim; named commit `c9bdd0f` adds builder-verified bounded schema evidence and a sealed outbox. Later implementation slices remain unaccepted except for the separately scoped macOS marker profile named below. Frozen source `82f5cd6cc505cc64bdea73eebeb535c2a1b4cd11` includes the production-time, replay, socket-budget, and evidence remediation. Its full non-administrator matrix and bounded macOS administrator gate passed; report `172840` records cleanup status 0. The reviewer issued scoped acceptance for that bounded marker profile at source `82f5cd6`, evidence child `6bb0a07`, and report `172840`; `docs/reviews/CURRENT-REVIEW.md` identifies the exact hashes and disposition.

Before substantive work, read `NORTHSTAR-SESSION-START.md`, `docs/BUILD-SPEC-SHEET.md`, and the newest applicable report under `tests/reports/`. The 2.3 evaluation/authorization schema core and bounded 2.3d/Rust commit `aed80e2` remain the accepted implementation baseline. The separate marker-profile acceptance does not accept every later implementation slice. Historical report `064717` is builder evidence only for source `f025332`; the new report `172840` and JSONL cover source `82f5cd6`. Keep the source commit frozen and preserve evidence/status updates in a separate child. The reviewer's supplied scoped disposition is recorded in `docs/reviews/restricted-marker-scoped-acceptance-2026-09-05.md`; Phase 4 and Section 3 remain CHANGES REQUESTED / open. Do not imply Section 3/Phase 4 acceptance, production readiness, portable handoff, universal forced mediation, or an HTTP/network-egress PEP. JavaScript and Go live permission paths are historical/cooperative 0.1-era references, not 0.2 authorities or PEPs. The August 7 `AUTHORIZED` snapshot token is abandoned.

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
