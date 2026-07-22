# AGENTS.md — Northstar (Glass)

Instructions for AI coding agents and collaborators working in this repo.

## What this project is

**Northstar** is the product workspace for **Glass**: the Trust Layer enterprise gate.

- **Prism** = open intent signal (upstream protocol; do not redefine casually)
- **APEX-Lite / Trust-Engine** = open-source minimal reference boundary
- **Glass** = productized verification & enforcement layer we build here

Public story: https://trust-layer-ai.github.io/Trust-Layer-AI/

## Non-negotiables

1. **Pre-execution only for decisions that matter.** Glass authorizes *before* action hits real systems.
2. **Deterministic policy outcomes.** Do not route core ALLOW / REQUIRE_APPROVAL (or future DENY) through LLM judgment.
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

## Stack guidance (until locked)

- Default implementation language for new Glass code: **TypeScript on Node** (modern product path), unless we choose to evolve the JS APEX-Lite tree in place.
- Config and examples stay plain: **JSON / YAML**, readable by operators without a build step.
- Tests should cover policy decisions deterministically (fixtures in, receipt out).

## Commands

None yet — scaffold only. When `package.json` lands, document:

```bash
# expected later
npm test
npm start
```

Until then:

```bash
# local APEX-Lite reference (outside this repo)
cd ~/APEX-Lite && npm test   # if package scripts exist
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
