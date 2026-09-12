# APEX-Lite

APEX-Lite is a minimal external pre-execution policy boundary for AI systems.

APEX-Lite evaluates declared intent before execution. It compares that intent to explicit operator-defined policy, returns a deterministic decision, emits a receipt, and can append that receipt to an audit log.

This repository is the minimal public reference implementation of APEX-Lite. It is meant to be understandable, inspectable, and broadly reusable on its own. It does not claim to include every private, enterprise, or patent-backed implementation detail that may exist around the broader APEX approach.

The open-source reference implementation now also includes a local gate configuration under `config/`. That file defines the required request fields and simple keyword-driven gates such as email and folders.
It also includes an optional local notification configuration under `config/` for SMS escalation notices.

## What APEX-Lite Does

APEX-Lite is:

- a narrow pre-execution policy evaluator
- a deterministic decision engine
- an external pre-execution policy boundary
- a local reference implementation built for inspection and replay

The current reference flow is:

1. Read an intent JSON file.
2. Read a local policy file in the repository's simple YAML-like format.
3. Evaluate the intent against policy rules in order.
4. Return `ALLOW` or `REQUIRE_APPROVAL`.
5. Emit a structured JSON receipt.
6. Optionally append the receipt to a JSONL audit log.

## What APEX-Lite Does Not Do

APEX-Lite is not:

- an AI agent
- an orchestration framework
- a monitoring platform
- an in-model safety layer

It does not rely on LLM reasoning for policy outcomes.

## Decision Model

The decision model is intentionally small:

- `ALLOW`
- `REQUIRE_APPROVAL`

Normal uncertainty routes to approval rather than hard denial. The current implementation keeps the existing `REQUIRE_APPROVAL` label so it can later map cleanly to `ESCALATE`.

Escalation is treated as a positive transparency signal, not a failure state. The reference implementation does not expose a normal `DENY` or block path. It operates in `ALLOW_OR_ESCALATE` mode and records escalations with a reward signal rather than punishing honest declaration.
If both a gate and a YAML rule apply, `REQUIRE_APPROVAL` wins over `ALLOW`.

## Policy Format

APEX-Lite reads the repository's existing rule format.

```yaml
rules:
  - id: rule_01
    description: Require approval for high-risk actions
    if: risk == "high"
    require: human_approval

  - id: rule_02
    description: Require approval when PII is involved
    if: action == "send_email" and "PII" in data_classes
    require: human_approval
```

The evaluator intentionally supports only a small, explicit expression set used by this repo:

- equality checks like `risk == "high"`
- inequality checks like `risk != "low"`
- array membership like `"PII" in data_classes`
- `and` / `or` combinations

## CLI

Run an evaluation:

```bash
node bin/apex-lite.js evaluate examples/intent.json examples/policy.yaml
```

Append the receipt to an audit log:

```bash
node bin/apex-lite.js evaluate examples/intent.json examples/policy.yaml --log var/audit.jsonl
```

The CLI prints the receipt JSON to stdout and exits nonzero on invalid usage or parse errors.

## Operator Console

APEX-Lite also includes a small local execution console built as a thin UI over the real engine.

Start it with:

```bash
npm start
```

Then open [http://localhost:3000](http://localhost:3000).

The console:

- accepts manual intent input
- accepts a local operator identifier for escalation outcomes
- accepts `declared_intent`, request `version`, and `log_id`
- posts to `POST /api/evaluate`
- posts queue approvals and escalations to `POST /api/operator-action`
- displays the real receipt returned by the engine
- appends real evaluation results to the feed
- routes `REQUIRE_APPROVAL` decisions into a local escalation queue
- fetches recent receipts from `GET /api/audit`

The local server uses:

- the example policy under `examples/`
- the append-only audit log under `var/`
- the optional notification configuration under `config/`

The console also makes the trust model explicit to the submitting party: the declared intent is entering a policy gate, and gaming that declaration is not beneficial for either side because it breaks trust in the boundary itself.

## Receipt

A receipt is the deterministic record produced for one policy evaluation. It includes:

- `receipt_type`
- `apex_version`
- `evaluated_at`
- `receipt_id`
- `control_mode`
- `blocking`
- `decision`
- `reason`
- `policy_id`
- `reward_signal`
- `original_intent`

Example:

```json
{
  "receipt_type": "apex-lite.receipt",
  "apex_version": "0.1.0",
  "evaluated_at": "2026-01-01T00:00:00.000Z",
  "receipt_id": "rcpt_INT_001_20260101T000000000Z",
  "control_mode": "ALLOW_OR_ESCALATE",
  "blocking": false,
  "decision": "REQUIRE_APPROVAL",
  "reason": "Email gate matched escalation keywords",
  "policy_id": "email_gate",
  "reward_signal": "TRANSPARENCY_REWARDED",
  "original_intent": {
    "intent_id": "INT-001",
    "actor": "agent_42",
    "declared_intent": "send external customer email with pii attachment",
    "action": "send_email",
    "target": "external_user",
    "risk": "medium",
    "intent_version": "1",
    "log_id": "LOG-001",
    "data_classes": ["PII"],
    "timestamp": 1730000000
  }
}
```

`receipt_id` is a plain readable identifier for the local reference build. It gives operators and audit entries a simple way to refer to one evaluation. 

## Gate Config

The local gate configuration lives under `config/`.

It gives the open-source build one plain place to define:

- required request fields such as user or agent name, date and time, declared intent, version number, and log id
- gate categories such as email and folders
- keywords that should escalate
- keywords that are safe to allow

Gate matching is intentionally simple and deterministic. It uses normalized whole-keyword matching rather than arbitrary substring matching, and YAML policy rules still run after gate evaluation so approval requirements cannot be bypassed by a gate allow match.

## Audit Log

The optional audit log is append-only JSONL. Each evaluation appends one receipt as one line.
Escalation resolutions also append operator action entries with an explicit `outcome`, so the audit trail records not just that escalation occurred, but how it ended.
Those action entries also record the resolving operator identifier.
Escalation notifications can also append `apex-lite.notification` entries so operators can see whether an SMS notice was skipped, simulated, sent, or failed.
Only one operator outcome is accepted for a given escalated receipt.
Operator-action verification reads the audit log once per request and checks the referenced `receipt_id` and prior operator outcome against that recorded history.

That keeps the first implementation:

- simple
- local
- inspectable
- easy to replay with standard tooling

## Tests

Run the test suite with:

```bash
npm test
```

The tests cover:

- high-risk escalation
- PII escalation
- safe allow behavior
- YAML approval rules overriding gate allow matches
- whole-keyword gate matching instead of arbitrary substring matches
- plain receipt id presence
- one-line-per-evaluation audit logging
- real `POST /api/evaluate` responses
- real `POST /api/operator-action` responses
- real `GET /api/audit` responses
- rejection of forged or non-escalated operator actions

## Repository Shape

The current implementation is intentionally small:

- `bin/apex-lite.js`
- `src/engine`
- `src/policy`
- `src/receipt`
- `src/audit`
- `src/index`
- `src/server`
- `src/operator-action`
- `src/gates`
- `src/notifications`
- `config/`
- `public/`

This is meant to feel closer to a SCADA-style interlock or admission controller than to an autonomous system.

## License

This repository is licensed under Apache-2.0. See `LICENSE`.

That permissive license applies to this public reference implementation. It should not be read as a claim that every enterprise deployment pattern, private implementation detail, or broader patent-backed system around APEX-Lite is included in this repository.
Copyright 2025 Robert Katzenberger
