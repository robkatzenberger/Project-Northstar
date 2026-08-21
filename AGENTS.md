# AGENTS.md — Northstar monorepo

## Layout

- **Docs & schemas** — repo root (`docs/`, `schemas/`). Language-agnostic.
- **Implementations** — `implementations/<lang>/`.
- **Current reference code** — `implementations/javascript/` (Node ES modules).
- **Authoritative core (MVP)** — `implementations/rust` (`tlpx`): accepted 0.2 types/hashes plus durable local evaluate, issuance, and atomic claim; named commit `c9bdd0f` adds builder-verified bounded schema evidence and a sealed outbox. Local commits through 3.8 `7c41450` add the unaccepted 2.4 and Section 3.1–3.8 candidates, including Unix peer roles, approval lifecycle, authenticated revocation, durable idempotent terminal execution/reconciliation receipts, an authenticated local adapter contract, and one bounded cooperative direct-argv runner. Full Section 3 verification is deferred. Not a forced-mediation PEP yet.

Before substantive work, read `NORTHSTAR-SESSION-START.md`, `docs/BUILD-SPEC-SHEET.md`, and the newest applicable builder report under `tests/reports/`. The 2.3 evaluation/authorization schema core is accepted; portable revocation and approval-expiry evidence remain deferred. The 2.3d/Rust local-authority implementation at `aed80e2` is independently accepted for its bounded evaluate/issue/claim scope. The evidence/outbox increment at `c9bdd0f` is builder-verified and not independently accepted. Slices 2.4 and 3.1–3.8 are local commits; 3.8 is exact-commit builder-verified at `7c41450`. Full Section 3 verification and independent review are deferred, so all remain unaccepted. Public authority mutation paths now require `AuthenticatedIdentity`, while the process-owned peer map and identity construction remain trusted deployment inputs. Slice 3.8 performs one bounded same-UID cooperative side effect through `tlpx-run-demo`; it is bypassable and is not forced mediation. The JS gate remains a TL-PX 0.1 reference plus 0.2 contract oracles. Do not expand JS into the planned production authority. The August 7 `AUTHORIZED` snapshot token is abandoned.

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
