# Integration Guide

How to connect real systems to the Trust Layer **without** abandoning the air-gapped design.

## Principle

```text
Your runtime proposes an action
  → builds declared intent
  → calls evaluate (+ Switchboard)
  → obtains receipt
  → only then performs the side effect via executeAuthorized (or equivalent)
```

The library does **not** magically wrap every process on the machine. **You** provide mediation.

---

## Pattern A — Library mediation (recommended default)

Best for: services, workers, custom agent runtimes, scripts.

```js
import {
  createPrismSignal,
  toEvaluationIntent,
  evaluateIntent,
  resolveEscalation,
  executeAuthorized,
  readPolicyFile,
  loadSwitchboard
} from "./src/index.mjs";

const policy = readPolicyFile("./config/policy.yaml");
const switchboard = loadSwitchboard("./config/switchboard.json");
const auditPath = process.env.TLPX_AUDIT || "./var/tech-test-audit.jsonl";

export async function gatedAction({ agent, summary, action, target, risk, data_classes, run }) {
  const intent = toEvaluationIntent(
    createPrismSignal({
      agent,
      actor_type: "machine",
      intent_summary: summary,
      action,
      target,
      risk,
      data_classes
    })
  );

  const decision = evaluateIntent(intent, policy, { auditPath, switchboard });

  if (decision.decision === "DENY") {
    return { allowed: false, decision };
  }

  if (decision.decision === "REQUIRE_APPROVAL") {
    // hand off to your ops queue using decision.approval_route
    // later:
    // resolveEscalation(decision, { operator_id, outcome: "APPROVE" }, { auditPath });
    return { allowed: false, pending: true, decision };
  }

  const result = await executeAuthorized({
    auditPath,
    receipt_id: decision.receipt_id,
    executor_id: agent,
    sideEffect: () => run()
  });

  return { allowed: result.ok, result, decision };
}
```

Register each agent id in Switchboard with credibility and `allowed_actions`.

---

## Pattern B — CLI / subprocess gate

Best for: shell automation, polyglot services.

```bash
node bin/glass.mjs evaluate intent.json config/policy.yaml --log "$AUDIT"
# parse decision JSON
# if REQUIRE_APPROVAL → wait for human CLI/API
node bin/glass.mjs execute "$RECEIPT" --executor my-job --status EXECUTED --log "$AUDIT"
```

Your job runner must **not** run the payload until `auth` is AUTHORIZED.

---

## Pattern C — Optional coding-harness adapter (e.g. Grok Build)

Best for: developer agents in a TUI/CLI harness.

Grok Build (and similar tools) can call the gate from a **`PreToolUse` hook**, but:

- That is an **adapter**, not the product core  
- Harness hooks may **fail open** on script errors — harden carefully  
- **Always-approve** modes skip human prompts; hooks/gate remain the control  

Recommended order:

1. Prove Pattern A/B (air-gap technical test) — **done for this repo**  
2. Map tool names → actions (`run_terminal_command` → `shell`, edits → `modify_file`)  
3. Hook calls evaluate; deny on DENY/PENDING (or async approve workflow)  
4. PostToolUse records execution outcome  

Do not claim “Grok is secured” until the hook is fail-closed for the tools you care about and Switchboard principals exist for main/subagents.

---

## Pattern D — Future local service

HTTP/Unix socket profile (not shipped yet):

```text
POST /v1/evaluate
POST /v1/operator-action
GET  /v1/auth/:receipt_id
POST /v1/execution
```

Keep the same records and audit semantics for TL-PX conformance.

---

## Mapping host actions → intent

| Host action | Suggested `action` | Typical risk |
| --- | --- | --- |
| Read-only search | `read` / omit | low |
| Edit files | `modify_file` | medium |
| Shell | `shell` | medium–high |
| Network egress | `access_api` | medium–high |
| Deploy | `deploy` | high |
| Payments | `transfer_funds` | high |
| Spawn subagent | `spawn_agent` | medium |

Keep Switchboard `allowed_actions` tight per principal.

---

## Human-in-the-loop integration

When `REQUIRE_APPROVAL`:

1. Read `approval_route` from the decision  
2. Notify operators (email, Slack, ops UI — your system)  
3. Call `resolveEscalation` with authenticated operator id  
4. Only then `executeAuthorized`  

Until operator authN exists in-repo, treat operator id as an integration responsibility.

---

## Checklist for production-shaped deploys

- [ ] Every sensitive side effect goes through `executeAuthorized` (or equivalent)  
- [ ] Switchboard lists all production principals  
- [ ] Audit path not world-writable  
- [ ] Policy/Switchboard not writable by agents  
- [ ] `allowEphemeral` never set  
- [ ] Separate principals for interactive vs CI  
- [ ] Incident runbook uses `chain` + accountability  
- [ ] Conformance + tech-test green in CI  

---

## Related

- [Air-Gapped Operation](./airgap.md)  
- [Security Model](./security.md)  
- [API Reference](./api-reference.md)  
- [Switchboard](./switchboard.md)  
