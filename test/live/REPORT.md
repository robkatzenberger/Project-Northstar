# Live multi-agent Switchboard test — 2026-09-10T03:56:06.922Z

## Roster
| principalId | Agent | Round A | Round B |
|---|---|---|---|
| agent.arch | Arch | WL | cold |
| agent.shield | Shield | WL | cold |
| agent.docs | Docs | cold | WL |

Action/target: `ping` → `switchboard.g`
Harness: `apex/test/live/screen.mjs` against real `src/switchboard.js`
G acted as Switchboard host; credentials never logged in clear.

## Results (expected → observed)

### Round A (`apex.live-ping` v1)
1. Arch → PASS SCOPE_GRANTED (grant-arch-ping) ✓
2. Shield → PASS SCOPE_GRANTED (grant-shield-ping) ✓
3. Docs → DENY NOT_WHITELISTED ✓

### Round B (`apex.live-ping-reverse` v2)
4. Arch → DENY NOT_WHITELISTED ✓
5. Shield → DENY NOT_WHITELISTED ✓
6. Docs → PASS SCOPE_GRANTED (grant-docs-ping) ✓

### Combos (live agent-submitted)
7. Docs credential + intent.agentId=agent.arch → DENY IDENTITY_MISMATCH ✓
8. Arch empty credential → DENY UNAUTHENTICATED ✓

### Extra G-side (same module, not agent-submitted)
- unknown 64-hex token → DENY UNAUTHENTICATED ✓

## Dual-side
Agent-side seals recorded in `agent-side-log.jsonl` for all Round A/B principal pings; combo decisions returned to Docs/Arch for seals.

## Go / no-go
**GO for current Switchboard foundation** under this live multi-agent harness:
- whitelist flips work
- auth precedes grant
- identity mismatch fail-closed
- missing/unknown credential fail-closed
- PASS is pre-policy only (`next: CONSTRAINT_SCREENING`), not execution

No Switchboard rebuild required from this test. Next product slice remains Glass/constraints + sealed TL-PX decision receipts (Execute optional), per open MVP reset.
