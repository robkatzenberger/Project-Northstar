# Testing

## Quick commands

```bash
cd ~/projects/northstar/implementations/javascript

npm test                      # unit + switchboard + air-gap + policy + compile
npm run test:unit
npm run test:switchboard
npm run test:airgap
npm run conformance           # TL-PX 0.1 Minimum Profile
node scripts/tech-test.mjs    # formal technical test #1
node scripts/adversarial-redteam.mjs
npm run demo
npm run demo:switchboard
```

All tests use local Node; no network required.

---

## Suite map

| Script | Purpose | Audit |
| --- | --- | --- |
| `scripts/test.mjs` | Core Prism/gate/accountability | temp files |
| `scripts/test-switchboard.mjs` | Whitelist, credibility, DENY paths | temp files |
| `scripts/test-airgap.mjs` | Chain auth, state machine, anti-forgery, executor | temp files |
| `scripts/test-policy-ops.mjs` | Safe expressions + operator allowlist | temp files |
| `scripts/test-policy-compile.mjs` | Phase 1 negative compile / fail-closed load | temp files |
| `scripts/conformance.mjs` | Spec conformance (deterministic decisions, parties, chain) | temp files |
| `scripts/tech-test.mjs` | **Formal E2E technical test #1** | monorepo `var/tech-test-audit.jsonl` |
| `scripts/adversarial-redteam.mjs` | Red team / residual risk | temp files |
| `scripts/no-bs.mjs` | Earlier theory scoreboard | temp files |
| `scripts/demo.mjs` | Narrative demo | `var/demo-audit.jsonl` |
| `scripts/demo-switchboard.mjs` | Switchboard demo | `var/demo-switchboard-audit.jsonl` |

---

## Technical test #1 (passed)

**Command:** `node scripts/tech-test.mjs`  
**Log:** monorepo `var/tech-test-audit.jsonl` (from package: `../../var/…`)

| ID | Scenario | Expectation |
| --- | --- | --- |
| T1 | Safe allow | ALLOW → sideEffect runs → EXECUTED |
| T2 | Unknown principal | Switchboard DENY → sideEffect never runs → BLOCKED |
| T3 | Not whitelisted | DENY → no sideEffect |
| T4 | PII email | REQUIRE_APPROVAL → pending blocks → APPROVE → EXECUTED |
| T5 | Funds / low cred | REQUIRE_APPROVAL → REJECT → BLOCKED |
| T6 | Audit | Multi-record chain present |

**Status:** PASS (29/29) as of last formal run.

---

## Conformance

Claims **TL-PX 0.1 Minimum Profile CONFORMING** when:

```bash
npm run conformance
# → Conformance: N passed, 0 failed
```

Categories: decision determinism, authorization mapping, execution guard, party model, chain integrity, accountability, request shape.

---

## Air-gap hardening checks

`npm run test:airgap` covers:

- Forged in-memory AUTHORIZED rejected  
- Stub operator ignored  
- Double-resolve blocked  
- REJECT terminal  
- Forged execution line does not create AUTHORIZED chain state  
- `executeAuthorized` fail-closed  
- Receipt id uniqueness  

---

## Red team

```bash
node scripts/adversarial-redteam.mjs
```

Interprets **PASS / FAIL / WARN**.  
WARN items (audit file write, bypass outside library, operator spoof) may remain by design until deployment controls exist.

Do not treat red team PASS as “no residual risk.”

---

## Writing new tests

1. Always pass `auditPath` (use `os.tmpdir()` files).  
2. Prefer `executeAuthorized` when testing side-effect gating.  
3. Assert on `policy_id` and Switchboard `flags` for routing.  
4. Clean up temp logs in the test.  
5. Update this doc and [changelog.md](./changelog.md) for behavioral tests.  

---

## CI notes

No CI pipeline is configured in-repo by default (local project). Suggested job:

```bash
cd implementations/javascript
node --version
npm test
npm run conformance
node scripts/tech-test.mjs
```
