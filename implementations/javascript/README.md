# JavaScript / Node reference implementation

**Status:** Active  
**Runtime:** Node.js 18+ (ES modules, `.mjs`)  
**Package name:** `tlpx-reference`

This is the current full **TL-PX 0.1** reference for Switchboard + air-gapped executor, plus the **0.2 schema/policy/typed-action/JCS/hash oracle**. Runtime records still use `standard_version: "0.1.0"`.

## Layout

```text
javascript/
  src/           Library (glass, switchboard, chain, executor, jcs, hash, …)
  bin/glass.mjs  CLI
  config/        policy.yaml, switchboard.json
  examples/      Intent fixtures
  scripts/       tests, demos, tech-test, red-team
  package.json
  AGENTS.md      Conventions for coding agents in this tree
```

Shared 0.1 schemas live at repo root: `../../schemas/tlpx-0.1/`.  
0.2 JCS fixtures: `../../tests/fixtures/tlpx-0.2/jcs/`.  
0.2 policy fixture: `../../tests/fixtures/tlpx-0.2/policy/`.
0.2 typed-action fixture: `../../tests/fixtures/tlpx-0.2/actions/`.
Docs live at repo root: `../../docs/`.

## Commands

```bash
cd implementations/javascript

npm test
npm run test:jcs
npm run test:policy:0.2
npm run test:actions:0.2
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
