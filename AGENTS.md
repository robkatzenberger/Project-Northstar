# AGENTS.md — Northstar monorepo

## Layout

- **Docs & schemas** — repo root (`docs/`, `schemas/`). Language-agnostic.
- **Implementations** — `implementations/<lang>/`.
- **Current reference code** — `implementations/javascript/` (Node ES modules).
- **Authoritative core (MVP)** — `implementations/rust` (`tlpx`): accepted 0.2 types/hashes plus durable local evaluate, issuance, and atomic claim; named commit `c9bdd0f` adds builder-verified bounded schema evidence and a sealed outbox. Local commits `a87f822`, `1addb5c`, and `e835c4e` add the unaccepted 2.4 contract, 3.1 policy activation, and 3.2 typed-action/hash candidates. The uncommitted 3.3 candidate adds kernel-derived Unix peer identity, authenticated role facades, policy-bound approval routes, and atomic pending cancellation with sealed operator evidence. Full Section 3 verification is deferred. Not a PEP yet.

Before substantive work, read `NORTHSTAR-SESSION-START.md`, `docs/BUILD-SPEC-SHEET.md`, and the newest applicable builder report under `tests/reports/`. The 2.3 evaluation/authorization schema core is accepted; execution-receipt, post-claim cancellation/reconciliation, and revocation-evidence schema closure is deferred. The 2.3d/Rust local-authority implementation at `aed80e2` is independently accepted for its bounded evaluate/issue/claim scope. The evidence/outbox increment at `c9bdd0f` is builder-verified and not independently accepted. Slice 2.4 is locally committed at `a87f822`; slice 3.1 at `1addb5c`; slice 3.2 is bounded exact-commit verified at `e835c4e`; slice 3.3 is an uncommitted builder-verified working-tree candidate. Full Section 3 verification and independent review are deferred, so all remain unaccepted. Raw principal-string methods remain trusted-embedding inputs; untrusted callers must use the authenticated Unix peer facade. There is still no approval resolution, execution receipt, or PEP. The JS gate remains a TL-PX 0.1 reference plus Phase 1 compile and a 0.2 schema/policy/typed-action/JCS/hash oracle. Do not expand JS into the planned production authority. The August 7 `AUTHORIZED` snapshot token is abandoned.

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
