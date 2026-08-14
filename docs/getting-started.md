# Getting Started

This guide gets you from a clean checkout to a verified evaluate → (optional approve) → execute path on your machine.

## Requirements

- **Node.js 18+** (ES modules)
- No npm install required for the reference runtime (zero production dependencies)
- macOS, Linux, or compatible environment

## Project location

```text
~/projects/northstar                 # monorepo root (docs, schemas)
~/projects/northstar/implementations/javascript   # active Node code
```

## 1. Enter the JavaScript implementation

```bash
cd ~/projects/northstar/implementations/javascript
node --version   # >= 18
```

Docs and shared schemas stay at the monorepo root (`../../docs`, `../../schemas`).

## 2. Run the test suite (sanity)

```bash
npm test                 # unit + switchboard + air-gap + policy compile + 0.2 JCS/schema
npm run conformance      # TL-PX 0.1 Minimum Profile
```

Expected: all suites pass / CONFORMING.

## 3. Run the narrative demos

```bash
npm run demo               # → ../../var/demo-audit.jsonl
npm run demo:switchboard   # → ../../var/demo-switchboard-audit.jsonl
```

## 4. First CLI evaluate (with audit log)

Air-gapped mode **requires** an audit file (monorepo `var/`):

```bash
mkdir -p ../../var
: > ../../var/tech-test-audit.jsonl
chmod 600 ../../var/tech-test-audit.jsonl

node bin/glass.mjs evaluate \
  examples/intent-safe.json \
  config/policy.yaml \
  --log ../../var/tech-test-audit.jsonl
```

With Switchboard (default if `config/switchboard.json` exists):

```bash
node bin/glass.mjs evaluate \
  examples/intent-pii-email.json \
  config/policy.yaml \
  --log ../../var/tech-test-audit.jsonl
```

Copy the `receipt_id` from the JSON output.

### If escalated (`REQUIRE_APPROVAL`)

```bash
node bin/glass.mjs approve <receipt_id> \
  --operator human.ops.alex \
  --log ../../var/tech-test-audit.jsonl \
  --note "Approved for test"
```

### Check authorization from the audit chain

```bash
node bin/glass.mjs auth <receipt_id> --log ../../var/tech-test-audit.jsonl
```

### Record execution (chain-verified)

```bash
node bin/glass.mjs execute <receipt_id> \
  --executor runtime.demo \
  --status EXECUTED \
  --log ../../var/tech-test-audit.jsonl
```

### Inspect the chain

```bash
node bin/glass.mjs chain <receipt_id> --log ../../var/tech-test-audit.jsonl
```

## 5. Formal technical test #1

```bash
node scripts/tech-test.mjs
```

This resets and writes:

```text
../../var/tech-test-audit.jsonl   # monorepo root var/
```

Covers: safe allow, Switchboard DENY (unknown / not whitelisted), escalate→approve→execute, reject→block, audit chain.

## 6. Library usage (minimal)

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
// monorepo root audit path (from implementations/javascript)
const auditPath = "../../var/tech-test-audit.jsonl";

const intent = toEvaluationIntent(
  createPrismSignal({
    agent: "agent.docs.summarizer",
    actor_type: "machine",
    intent_summary: "Summarize quarterly report",
    action: "summarize_report",
    target: "internal_workspace",
    risk: "low",
    data_classes: []
  })
);

const decision = evaluateIntent(intent, policy, { auditPath, switchboard });

if (decision.decision === "REQUIRE_APPROVAL") {
  resolveEscalation(
    decision,
    { operator_id: "human.ops.alex", outcome: "APPROVE" },
    { auditPath }
  );
}

const result = await executeAuthorized({
  auditPath,
  receipt_id: decision.receipt_id,
  executor_id: "my-runtime",
  sideEffect: () => {
    // Only runs if audit chain says AUTHORIZED
    return doSensitiveWork();
  }
});

console.log(result.ok ? result.result : result.reason);
```

## What “good” looks like

| Scenario | Expected |
| --- | --- |
| Whitelisted low-risk action | `ALLOW` → side effect runs |
| Unknown principal | `DENY` (`switchboard.unknown_deny`) → side effect never runs |
| PII email | `REQUIRE_APPROVAL` → pending cannot execute → after APPROVE can execute |
| After REJECT | `DENIED` → side effect never runs |

## Next reading

1. [Concepts](./concepts.md)  
2. [Switchboard](./switchboard.md)  
3. [Air-Gapped Operation](./airgap.md)  
4. [Security Model](./security.md)  
5. [API Reference](./api-reference.md)  

## Troubleshooting

| Symptom | Cause / fix |
| --- | --- |
| `auditPath required` | Pass `--log` / `auditPath` — air-gap default |
| Everything DENY | Principal missing or `whitelisted: false` in Switchboard |
| Constant REQUIRE_APPROVAL | Low credibility or matching policy rule — check `policy_id` |
| Side effect still runs outside tests | Your app must call `executeAuthorized` (or equivalent); the library cannot stop unaudited processes |
