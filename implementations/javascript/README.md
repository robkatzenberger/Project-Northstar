# JavaScript / Node reference implementation

**Status:** Active  
**Runtime:** Node.js 18+ (ES modules, `.mjs`)  
**Package name:** `tlpx-reference`

This is the current full reference for TL-PX + Switchboard + air-gapped executor.

## Layout

```text
javascript/
  src/           Library (glass, switchboard, chain, executor, …)
  bin/glass.mjs  CLI
  config/        policy.yaml, switchboard.json
  examples/      Intent fixtures
  scripts/       tests, demos, tech-test, red-team
  package.json
  AGENTS.md      Conventions for coding agents in this tree
```

Shared schemas live at repo root: `../../schemas/tlpx-0.1/`.  
Docs live at repo root: `../../docs/`.

## Commands

```bash
cd implementations/javascript

npm test
npm run conformance
npm run tech-test
npm run demo
npm run demo:switchboard
npm run redteam

node bin/glass.mjs help
node bin/glass.mjs evaluate examples/intent-safe.json config/policy.yaml \
  --log ../../var/tech-test-audit.jsonl
```

Canonical local audit directory (monorepo root):

```text
../../var/tech-test-audit.jsonl
```

## Next language ports

See [../README.md](../README.md). TypeScript can live here later (`typescript/` sibling) or as a typed evolution of this package.
