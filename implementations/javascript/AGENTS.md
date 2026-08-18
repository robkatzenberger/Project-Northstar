# AGENTS.md — Northstar / TL-PX

Instructions for AI coding agents and collaborators working in this repo.

## What this project is

**Northstar** hosts the **Trust Layer Pre-Execution Minimum Standard (TL-PX)** plus a conforming reference implementation.

Open-source intent: a **minimum standard** people can implement, test (conformance), and run in real apps — not only a concept demo (that was APEX-Lite).

- **Prism** = open intent signal (upstream; do not redefine casually)
- **TL-PX** = minimum standard (spec + schemas + conformance) — primary artifact here
- **APEX-Lite / Trust-Engine** = early playable references
- **Glass** = enterprise product profile that **extends** TL-PX

Public story: https://trust-layer-ai.github.io/Trust-Layer-AI/

Normative 0.1: `docs/standard/SPEC-v0.1.md` (frozen)  
Draft 0.2: `docs/standard/SPEC-v0.2.md` (accepted evaluation/authorization schema core through 2.3; accepted Rust 2.3d at `aed80e2`; builder-verified evidence/outbox at `c9bdd0f`, not independently accepted; local 2.4/3.1 candidates at `a87f822`/`1addb5c`, with exact-commit/full Section 3 verification deferred and no acceptance claim)
Full documentation hub: `docs/README.md`

## Non-negotiables

1. **Pre-execution only for decisions that matter.** Glass authorizes *before* action hits real systems.
2. **Deterministic policy outcomes.** Do not route core ALLOW / REQUIRE_APPROVAL / DENY through LLM judgment.
3. **Prism stays metadata-only.** No prompts, chain-of-thought, or private reasoning in the signal.
4. **Receipts and audit are first-class.** Every evaluation should be inspectable and replayable.
5. **Transparency over gaming.** Honest declaration is rewarded; the boundary is not a cat-and-mouse filter on hidden intent.
6. **Separation of concerns.** Signal (Prism) ≠ judgment (Glass) ≠ execution (the agent/runtime).

## How to work in this repo

- Prefer **plan → approve → implement** for architecture or API shape changes.
- Match existing Trust Layer vocabulary: Prism signal, Glass/APEX decision, receipt, audit, escalation, operator.
- When unsure whether a feature belongs in Glass vs Prism vs APEX-Lite, **stop and ask** — do not merge layers.
- Reuse concepts from APEX-Lite (`ALLOW`, `REQUIRE_APPROVAL`, receipts, JSONL audit) unless we explicitly version a Glass contract.
- Keep secrets out of git. No API keys, SMS tokens, or production policy dumps in the tree.
- Do not force-push, rewrite published history, or push remotes unless the human asks.

## Stack guidance

- The authoritative 0.2 core is being built in **Rust**. Keep this JavaScript tree as the frozen 0.1 reference, Phase 1 compiler evidence, and 0.2 schema/policy/JCS/hash oracle; do not grow it into the production authority.
- Config and examples stay plain: **JSON / YAML**, readable by operators without a build step.
- Tests should cover policy decisions deterministically (fixtures in, receipt out).

## Commands

```bash
cd implementations/javascript   # from monorepo root
# or: cd ~/projects/northstar/implementations/javascript
npm run conformance   # frozen TL-PX 0.1 pass/fail
npm run conformance:0.2 # draft 0.2 schemas + contract oracles only
npm test              # unit + compile + 0.2 JCS/schema oracle
npm run test:jcs      # 0.2 hash oracle only
npm run test:policy:0.2 # 0.2 slice 2.4 policy/ordering oracle only
npm run demo          # 3 scenarios + accountability report
node bin/glass.mjs help
```

When changing decision enums, party model, or record fields: **update SPEC + schemas + conformance together**.

Local APEX-Lite reference (outside this repo):

```bash
cd ~/APEX-Lite && npm test
```

## Related code (do not treat as this repo)

| Location | Role |
| --- | --- |
| https://github.com/Trust-Layer-AI/prism-protocol | Prism open protocol |
| https://github.com/Trust-Layer-AI/Trust-Engine | APEX-Lite public reference |
| ~/APEX-Lite | Local APEX-Lite work |
| https://github.com/Trust-Layer-AI/Trust-Layer-AI | Marketing / GitHub Pages site |

## Definition of done (for future slices)

A slice is done when:

1. Behavior is specified (inputs, decisions, receipt fields).
2. Deterministic tests or fixtures pass.
3. Docs/examples updated if the public contract moved.
4. No secrets committed.

## Collaboration style

The human collaborates closely. Default:

- Propose approach for ambiguous product/architecture choices.
- Implement agreed slices fully (code + tests + short notes).
- Ask before remote git operations, dependency major bumps, or renames of public protocol fields.
