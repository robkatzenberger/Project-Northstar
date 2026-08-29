# JavaScript / Node reference implementation

**Status:** Historical/cooperative TL-PX 0.1 reference; not a TL-PX 0.2 authority or PEP
**Runtime:** Node.js 18+ (ES modules, `.mjs`)  
**Package name:** `tlpx-reference`

This is the frozen **TL-PX 0.1** reference for Switchboard plus a cooperative executor, together with the **0.2 schema/policy/typed-action/JCS/hash oracle**. Runtime records still use `standard_version: "0.1.0"`. `executeAuthorized` only protects calls routed through that function; it has no atomic 0.2 claim and cannot mediate a caller that retains a direct side-effect route. Do not connect 0.2 adapters to it or deploy it as the Rust authority.

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
