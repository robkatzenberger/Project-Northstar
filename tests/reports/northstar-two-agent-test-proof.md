# Northstar Two-Agent Execution-Gate Test Proof

Date: 2026-08-11  
Repository: `robkatzenberger/Project-Northstar`  
Tested commit: `ca05f6996534471e817d11f3c668e38411797fb8`  
Method: Agent A acted as requester/executor/adversary; Agent B independently acted as execution gate/auditor. Both agents were stopped after this single run.

## Published JavaScript suites

| Command | Exit | Result |
|---|---:|---|
| `npm test` | 0 | 79 assertions passed: 25 core, 24 Switchboard, 19 air-gap, 11 policy/operator |
| `npm run conformance` | 0 | 47 passed, 0 failed |
| `npm run tech-test` | 0 | 29 passed, 0 failed |
| `npm run redteam` | 0 | 16 PASS, 0 FAIL, 4 documented WARN |

Runtime: Node v25.5.0; npm 11.8.0. Go and Java tests were not run because usable Go, Java, and Maven toolchains were unavailable.

## Two-agent execution scenarios

| Scenario | Decision / state | Observable result | Verdict |
|---|---|---|---|
| Safe action | `ALLOW` / `AUTHORIZED` via `policy_allow` | `safe.marker` created with `SAFE_EXECUTED` | Gate allowed authorized action |
| Approval-required, before approval | `REQUIRE_APPROVAL` / pending | `pending-before.marker` absent; `BLOCKED` recorded | Gate blocked action |
| Forged in-memory `AUTHORIZED` | Audit remained pending | `forged.marker` absent; second `BLOCKED` recorded | Forgery rejected |
| After valid approval | `AUTHORIZED` via `human_approve` | `approved-after.marker` created with `APPROVED_EXECUTED` | Gate allowed approved action |
| Unknown principal | `DENY` / `DENIED` via `gate_deny` | `denied.marker` absent; `BLOCKED` recorded | Gate denied action |
| Direct action outside gate | No evaluation or receipt | `direct-bypass-control.marker` created | Confirms forced mediation is not implemented |

## Independent audit verification

Agent B independently inspected the scenario source, actual marker files, all nine audit rows, receipt-derived authorization, hash links, and HMAC seals rather than trusting Agent A's summary.

`verifyAudit` result:

```json
{"ok":true,"lines":9,"errors":[]}
```

Audit order:

1. Safe decision
2. Safe execution
3. Approval-required decision
4. Pre-approval blocked execution
5. Forged-state blocked execution
6. Operator approval by `human.ops.alex`
7. Approved execution
8. Unknown-principal denial
9. Denied execution recorded as blocked

## Targeted negative probes

Two concerns from the static review were reproduced:

1. A malformed policy condition, `if: risk ==`, parsed without rejecting the policy. It failed to match and the engine returned `ALLOW` with `No approval rules matched`.
2. A real runtime Switchboard `DENY` failed `validateDecisionRecord` with `decision must be ALLOW|REQUIRE_APPROVAL`. The published decision schema enum likewise excludes `DENY`.

These probes exited 0 because the temporary harness was designed to assert that the suspected defects were observable.

## Evidence hashes

| Artifact | SHA-256 |
|---|---|
| `audit.jsonl` | `fe9187f70f3521b45ee87b89d1b9d6efc91cd5198d25a3804cf8f7981ccb7bc8` |
| `.audit.jsonl.seal` | `71e37a610d9930559f60e02be3ac4e6bde08cf99acff065393988af607b2803f` |
| `evidence.json` | `c3787b3092930cb163d425f712ea1d28b9fb030aca6ec617ed74586d6d3a9ee5` |

## Conclusion

The implemented cooperative execution gate works for mediated calls: it allowed safe and approved actions, blocked pending and denied actions, rejected forged in-memory authorization, and produced a valid sealed audit chain.

The test also proves that Northstar is not yet an unavoidable enforcement boundary: a process retaining direct authority can bypass the gate. Policy parsing currently has a fail-open path, and the runtime/schema contract disagrees about `DENY`.

No repository files were changed during the test run. No commits, pushes, or pull requests were created by the test agents.
