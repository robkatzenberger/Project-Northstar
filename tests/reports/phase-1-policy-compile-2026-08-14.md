# Phase 1 Policy Compile Evidence — 2026-08-14

**Scope:** slices 1.1–1.2 only (strict policy parse → validate → compile → evaluate, plus negative suite).  
**Repository:** `robkatzenberger/Project-Northstar`  
**Branch:** `main`  
**Baseline commit:** `7a0b371e1307739e465f8c5bd313ef9372adc9be`  
**Implementation:** local commit on `main` (not pushed)  
**Runtime:** Node from this machine; commands run in `implementations/javascript/`

This is implementation evidence for the Phase 1 fail-closed compiler. It does not supersede `northstar-two-agent-test-proof.md`, which remains the independent pre-fix baseline.

## Commands

| Command | Exit | Result |
|---|---:|---|
| `npm test` | 0 | 151 assertions: 25 core, 24 Switchboard, 19 air-gap, 11 policy/operator, 72 compile/negative |
| `npm run conformance` | 0 | 47 passed, 0 failed (frozen TL-PX 0.1 fixtures untouched) |
| `npm run redteam` | 0 | 16 PASS, 0 FAIL, 4 documented WARN |
| `npm run tech-test` | 0 | 29 passed, 0 failed |

## Defect closed

The two-agent probe `if: risk ==` no longer authorizes. `compileExpression`, `compilePolicy`, `evaluateCondition`, `evaluateRules`, `evaluateIntent`, and `readPolicyFile` all throw. `evaluateIntent` writes no audit row.

Red-team A7 now treats a malformed inject pack as a compile failure with no `AUTHORIZED` record, not as a silent non-match that falls through to `ALLOW`.

Valid packs keep the frozen v0.1 evaluate rule: no matching escalation rule implies `ALLOW`.

## Out of scope (unchanged)

- Switchboard runtime/schema `DENY` mismatch
- TL-PX 0.2 records, hashes, or conformance
- Rust authority, claim tickets, PEP / `tlpx-run`
- Forced mediation (A16 WARN remains)

No commit or push was made.
