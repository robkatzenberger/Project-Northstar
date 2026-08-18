# AGENTS.md — Northstar monorepo

## Layout

- **Docs & schemas** — repo root (`docs/`, `schemas/`). Language-agnostic.
- **Implementations** — `implementations/<lang>/`.
- **Current reference code** — `implementations/javascript/` (Node ES modules).
- **Authoritative core (MVP)** — `implementations/rust` (`tlpx`): accepted 0.2 types/hashes plus durable local evaluate, issuance, and atomic claim; named commit `c9bdd0f` adds builder-verified bounded schema evidence and a sealed outbox. Local commits through 3.5 `ad95653` add the unaccepted 2.4 and Section 3.1–3.5 candidates, including Unix peer roles, approval lifecycle, and authenticated transactional revocation. Full Section 3 verification is deferred. Not a PEP yet.

Before substantive work, read `NORTHSTAR-SESSION-START.md`, `docs/BUILD-SPEC-SHEET.md`, and the newest applicable builder report under `tests/reports/`. The 2.3 evaluation/authorization schema core is accepted; execution-receipt, post-claim cancellation/reconciliation, and portable revocation-evidence schema closure is deferred. The 2.3d/Rust local-authority implementation at `aed80e2` is independently accepted for its bounded evaluate/issue/claim scope. The evidence/outbox increment at `c9bdd0f` is builder-verified and not independently accepted. Slices 2.4 and 3.1–3.5 are local commits; 3.5 is exact-commit builder-verified at `ad95653`. Full Section 3 verification and independent review are deferred, so all remain unaccepted. Raw principal-string methods remain trusted-embedding inputs; untrusted callers must use authenticated role facades. Approval expiry and revocation have durable authority-local state but no portable records because those accepted schemas do not exist. There is still no execution receipt or PEP. The JS gate remains a TL-PX 0.1 reference plus 0.2 contract oracles. Do not expand JS into the planned production authority. The August 7 `AUTHORIZED` snapshot token is abandoned.

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
