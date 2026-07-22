# Air-gapped operation (hardened)

The Trust Layer is designed as an **external checkpoint**, not a harness hook.

```text
Client declares intent
        │  (local CLI / API / future socket — no cloud required)
        ▼
  Switchboard + policy evaluate  →  append decision to audit
        │
        ▼
  Optional human operator action  →  append (once) to audit
        │
        ▼
  executeAuthorized / recordExecution
        │  authorization_status derived ONLY from audit chain
        ▼
  Side effect runs only if AUTHORIZED
```

## Hardening rules (enforced in code)

| Rule | Behavior |
| --- | --- |
| **Mandatory audit** | `evaluateIntent`, `resolveEscalation`, `recordExecution` require `auditPath` (unless `allowEphemeral: true` for pure unit tests) |
| **Single operator outcome** | Second approve/reject on same receipt throws |
| **Chain-verified execute** | Ignores caller-supplied `authorization_status` / stub operator objects |
| **Fail-closed executor** | `executeAuthorized` never runs `sideEffect` unless audit says AUTHORIZED |
| **Unique receipts** | Receipt ids include entropy (no same-second collisions) |

## API sketch

```js
import {
  evaluateIntent,
  resolveEscalation,
  executeAuthorized,
  resolveAuthorizationFromAudit
} from "./src/index.mjs";

const auditPath = "var/audit.jsonl";

const decision = evaluateIntent(intent, policy, { auditPath, switchboard });

if (decision.decision === "REQUIRE_APPROVAL") {
  resolveEscalation(decision, { operator_id: "human.ops", outcome: "APPROVE" }, { auditPath });
}

const result = await executeAuthorized({
  auditPath,
  receipt_id: decision.receipt_id,
  executor_id: "runtime.worker",
  sideEffect: (auth) => {
    // only runs if AUTHORIZED
    return doSensitiveThing();
  }
});
```

## What this does *not* solve

- A process that never calls the gate can still act (OS/runtime must wrap tools).
- Audit file integrity without OS ACLs / signatures (local trust of the log file).
- Policy sandbox for untrusted multi-tenant policy authors.

Those remain deployment properties. This hardening makes the **gate itself** honest under air-gapped use.
