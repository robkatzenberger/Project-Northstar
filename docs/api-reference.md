# API Reference

JavaScript ES module API exported from `src/index.mjs`.  
**Runtime:** Node 18+. Zero npm dependencies.

```js
import { /* symbols below */ } from "./src/index.mjs";
```

Unless noted, durable operations require:

```js
{ auditPath: "/absolute/or/relative/path/to/audit.jsonl" }
```

Use `{ allowEphemeral: true }` **only** in pure unit tests (skips audit persistence requirement).

---

## Constants

| Export | Value / meaning |
| --- | --- |
| `STANDARD_ID` | `"TL-PX"` |
| `STANDARD_VERSION` | `"0.1.0"` (runtime records; 0.2 hashes are a separate oracle) |
| `CONTROL_MODE` | `"ALLOW_OR_ESCALATE"` |
| `HASH_PATTERN` | `/^sha256:[0-9a-f]{64}$/` |
| `GLASS_VERSION` | Reference impl version string |
| `PRISM_VERSION` | `"prism_v0.1"` |
| `CRED_MIN` / `CRED_MAX` | `0` / `0.99` |

---

## Prism

### `createPrismSignal(input) → signal`

Mint a Prism-compatible signal plus Glass extension fields.

**Required input**

| Field | Type |
| --- | --- |
| `agent` | string |
| `intent_summary` | string |

**Optional**

| Field | Type | Notes |
| --- | --- | --- |
| `actor_type` | `"human"` \| `"machine"` | default `machine` |
| `action` | string \| null | |
| `target` | string \| null | |
| `risk` | `"low"` \| `"medium"` \| `"high"` | |
| `data_classes` | string[] | |
| `context` | object | non-sensitive only |

**Returns** object with `prism_id`, `timestamp`, `agent`, `intent_summary`, `prism_version`, and `glass: { ... }`.

### `toEvaluationIntent(signal) → intent`

Flattens Prism + `glass` into an evaluation request used by the gate.

---

## Policy

### `readPolicyFile(path) → policy`

Load and **compile** the restricted YAML pack. Invalid packs throw.

### `parsePolicyText(text) → policy`

Parse policy text. `compilePolicy` / `readPolicyFile` reject unknown structure.

### `compilePolicy(policy) → policy`

Validate the entire pack. Compile identity is a module-private `WeakSet`; caller markers cannot skip validation.

### `evaluateRules(intent, policy) → { decision, reason, policy_id }`

Compiles first, then evaluates. Invalid packs throw and do not `ALLOW`.  
`decision`: `ALLOW` | `REQUIRE_APPROVAL`.

Expression subset: `==`, `!=`, `and` / `or`, `"X" in fieldName`, closed field list.

---

## Switchboard

### `loadSwitchboard(path) → switchboard`

Load and validate `config/switchboard.json` style config.

### `normalizeConfig(raw) → switchboard`

Validate in-memory config.

### `lookupPrincipal(switchboard, actorId) → { status, principal }`

`status`: `"known"` | `"unknown"`.

### `routeThroughSwitchboard(switchboard, intent) → context`

Produces flags, credibility, approval_route, and optional hard `gate` (`DENY` / escalate).

### `enrichIntentWithSwitchboard(intent, context) → intent`

Adds `whitelisted`, `credibility`, `low_credibility`, `high_trust`, etc. for policy expressions.

### `suggestCredibilityDelta(current, event) → { previous, delta, next, event }`

Suggests score change after `success` | `failure` | `escalation_honest` | `rejected` | `bypass_suspected`.  
Does **not** persist; caller writes config if desired.

---

## Gate (Glass)

### `evaluateIntent(intent, policy, opts) → decision`

Full evaluate path: optional Switchboard → policy → decision record.

**opts**

| Option | Required | Description |
| --- | --- | --- |
| `auditPath` | yes* | Append decision to JSONL |
| `switchboard` | no | Switchboard object from `loadSwitchboard` |
| `allowEphemeral` | no | Skip audit requirement (tests only) |

\* Required unless `allowEphemeral: true`.

**Returns** `tlpx.decision` record including `receipt_id`, `decision`, `authorization_status`, `parties`, `approval_route`, `switchboard` summary, `original_intent`.

### `resolveEscalation(input, action, opts) → operator_action`

Resolve a `REQUIRE_APPROVAL` receipt.

**input:** decision object or `{ receipt_id }`  
**action:** `{ operator_id, outcome: "APPROVE"|"REJECT", note? }`  
**opts:** `{ auditPath }` required

Loads decision **from audit**. Enforces **single terminal** operator outcome.

### `recordExecution(input, opts) → execution`

Record `EXECUTED` | `BLOCKED` | `FAILED`.

**input**

| Field | Required |
| --- | --- |
| `receipt_id` (or `decision.receipt_id`) | yes |
| `executor_id` | yes |
| `status` | yes |
| `executor_type` | no (default `machine`) |
| `result_summary` / `error` | no |

Authorization is **always re-derived from audit**. Forged in-memory status is ignored.  
`EXECUTED` throws unless audit says `AUTHORIZED`.

### `resolveAuthorizationFromAudit(auditPath, receiptId) → auth`

```js
{
  authorization_status, // AUTHORIZED | PENDING_HUMAN_APPROVAL | DENIED
  decision,
  operator_action,      // or null
  source,               // policy_allow | human_approve | human_reject | gate_deny | awaiting_operator
  terminal              // boolean
}
```

---

## Fail-closed executor

### `async executeAuthorized(opts) → result`

```js
{
  auditPath,      // required
  receipt_id,     // required
  executor_id,    // required
  executor_type?, // default machine
  sideEffect,     // async/sync function(auth) → any
  result_summary?
}
```

**Returns**

```js
// success
{ ok: true, result, execution, auth }

// not authorized or sideEffect threw
{ ok: false, reason, execution, auth, failed? }
```

`sideEffect` is **never** invoked unless `authorization_status === "AUTHORIZED"`.

---

## 0.2 JCS / hash oracle

Not used by 0.1 evaluate/audit. Other languages match `tests/fixtures/tlpx-0.2/jcs/golden.json`.

| Export | Role |
| --- | --- |
| `canonicalize(value)` / `canonicalizeJsonText(text)` | Northstar RFC 8785 JCS (UTF-16 key sort; reject floats and lone surrogates) |
| `intentHash` / `authorizedActionHash` / `executedActionHash` / `approvalContextHash` | Domain-separated `sha256:` strings |
| `assertHashString(value)` | Require `sha256:` + 64 lowercase hex |

---

## Audit

### `appendAudit(logPath, record) → record`

Append one JSON line.

### `readAudit(logPath) → record[]`

Read all records.

### `chainForReceipt(records, receiptId) → record[]`

Filter records linked to a receipt.

### `buildChain(auditRecords, receiptId) → record[]`

Ordered chain for display / analysis.

---

## Accountability

### `analyzeAccountability(chain, incident?) → report`

Build accountability report (findings + parties_involved).

Optional `incident`: `{ what_went_wrong, observed_action?, severity? }`.

### `reportFromAuditFile(auditPath, receiptId, incident?, opts?) → { chain, report }`

Load chain and analyze. `{ persist: true }` appends incident + report to audit.

---

## Validation

Structural validators (no Ajv dependency):

- `validateEvaluationRequest(req)`
- `validateDecisionRecord(rec)`
- `validateOperatorAction(rec)`
- `validateExecutionRecord(rec)`
- `validateAccountabilityReport(rec)`

Each returns `{ ok: boolean, errors: string[] }`.

---

## Chain helpers

- `assertNotAlreadyResolved(auditPath, receiptId)` — throws if operator already acted  
- `findDecisionInRecords(records, receiptId)`  

---

## Typical control flow

```js
const decision = evaluateIntent(intent, policy, { auditPath, switchboard });

if (decision.decision === "DENY") {
  // refuse work; optional executeAuthorized will BLOCK
}

if (decision.decision === "REQUIRE_APPROVAL") {
  // wait for human…
  resolveEscalation(decision, { operator_id, outcome: "APPROVE" }, { auditPath });
}

const { ok, result, reason } = await executeAuthorized({
  auditPath,
  receipt_id: decision.receipt_id,
  executor_id: "runtime",
  sideEffect: () => performAction()
});
```

See also [CLI Reference](./cli-reference.md) and [Integration Guide](./integration.md).
