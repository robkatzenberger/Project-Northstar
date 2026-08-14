# AGENTS.md — Northstar monorepo

## Layout

- **Docs & schemas** — repo root (`docs/`, `schemas/`). Language-agnostic.
- **Implementations** — `implementations/<lang>/`.
- **Current reference code** — `implementations/javascript/` (Node ES modules).
- **Authoritative core (MVP)** — `implementations/rust` (`tlpx`): accepted 0.2 types/hashes plus durable local evaluate, issuance, and atomic claim; the current working tree adds builder-verified bounded schema evidence and a sealed outbox. Not a PEP yet.

Before substantive work, read `NORTHSTAR-SESSION-START.md`, `docs/BUILD-SPEC-SHEET.md`, and `tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md`. The 2.3 evaluation/authorization schema core is accepted; execution-receipt, cancellation/reconciliation, and revocation-evidence schema closure is deferred. The 2.3d/Rust local-authority implementation at `aed80e2` is independently accepted for its bounded evaluate/issue/claim scope. The newer working-tree evidence/outbox delta is builder-verified but uncommitted and not independently accepted. Caller-supplied authenticated-context strings remain trusted-embedding inputs, not an authentication mechanism. There is still no execution receipt or PEP. The JS gate remains a TL-PX 0.1 reference plus Phase 1 compile and a 0.2 schema/JCS/hash oracle. Do not expand JS into the planned production authority. The August 7 `AUTHORIZED` snapshot token is abandoned.

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
