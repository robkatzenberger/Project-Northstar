# AGENTS.md — Northstar monorepo

## Layout

- **Docs & schemas** — repo root (`docs/`, `schemas/`). Language-agnostic.
- **Implementations** — `implementations/<lang>/`.
- **Active code** — `implementations/javascript/` (Node ES modules).

When editing runtime code, work under `implementations/javascript/` unless the task is explicitly another language.

## Commands (JS)

```bash
cd implementations/javascript
npm test
npm run conformance
npm run tech-test
```

## Non-negotiables

Same as the JS tree: pre-execution, deterministic policy, Prism metadata-only, Switchboard-first DENY, chain-verified execute, no secrets in git.

Full product rules: `implementations/javascript/AGENTS.md`  
Docs hub: `docs/README.md`
