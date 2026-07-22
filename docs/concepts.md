# Core Concepts

## The problem in one line

AI agents (and humans) can act on real systems faster than oversight can react. Logs after the fact are too late. Sensitive actions need **authorization before execution**, with a trail that names **who declared**, **who decided**, **who approved**, and **who executed**.

## Design center: air-gapped checkpoint

The Trust Layer is an **external** pre-execution boundary — not an in-model guardrail and not (primarily) a plugin inside a coding harness.

```text
Intent is declared  →  independent gate evaluates  →  only then may action run
```

Integrations (CLI, local service, optional harness hooks) are **clients** of this boundary.

## Building blocks

### 1. Prism (intent signal)

A lightweight, **metadata-only** description of what an agent is about to do.

- Does **not** judge, approve, or deny
- Does **not** carry prompts or chain-of-thought
- Upstream open protocol: [prism-protocol](https://github.com/Trust-Layer-AI/prism-protocol)

Core fields (v0.1): `prism_id`, `timestamp`, `agent`, `intent_summary`, `prism_version`.

This repo extends evaluation with a `glass` object (action, risk, data classes, actor type) without claiming those fields are Prism core.

### 2. Switchboard (identity router) — first line

Registry of principals (agents, machines, humans):

| Concern | Behavior |
| --- | --- |
| Identity | Lookup by principal id |
| Whitelist | Not registered / not whitelisted → **DENY** |
| Action scope | Optional per-principal allow-list → **DENY** if violated |
| Credibility | Score **0.00–0.99** (never 1.0) bands low/medium/high |
| Approval route | Who should handle human escalation |

**Order:** Switchboard runs **before** policy. Hard DENY never reaches YAML rules.

See [switchboard.md](./switchboard.md).

### 3. Glass / TL-PX gate (policy decision)

Deterministic evaluation of the evaluation request against operator-defined rules.

| Decision | Meaning |
| --- | --- |
| `ALLOW` | No further human approval required for this request |
| `REQUIRE_APPROVAL` | Human must APPROVE before execution may proceed |
| `DENY` | Hard deny (this reference: primarily Switchboard hard gates) |

Control mode: **`ALLOW_OR_ESCALATE`** for policy (escalate uncertainty rather than opaque auto-block in the APEX lineage). Switchboard remains free to hard-deny access.

No LLM in the allow/escalate path.

### 4. Operator action (human authorizer)

When decision is `REQUIRE_APPROVAL`:

- Human on `approval_route` records `APPROVE` or `REJECT`
- **Exactly one** terminal operator outcome per receipt (state machine)
- REJECT is terminal (cannot later APPROVE the same receipt)

### 5. Execution (fail-closed)

Side effects must run only when the **audit chain** says `AUTHORIZED`.

Use `executeAuthorized`:

- PENDING / DENIED → `sideEffect` **never** called; `BLOCKED` recorded when possible
- AUTHORIZED → run side effect; record `EXECUTED` or `FAILED`

Caller-supplied “AUTHORIZED” strings on in-memory objects are **ignored**.

### 6. Audit log (source of truth)

Append-only JSONL. Authorization for execution is **re-derived from the audit**, not from mutable memory.

Canonical technical-test path: `var/tech-test-audit.jsonl`  
See [logging.md](./logging.md).

### 7. Accountability

Post-incident analysis over a receipt chain:

- Lists human and machine parties involved
- Labels evidence surfaces (declarer, evaluator, authorizer, executor)
- **Not** automatic legal liability assignment

## Party model

Every durable step can name parties with:

```json
{ "id": "string", "type": "human" | "machine" }
```

| Role | Typical type | When |
| --- | --- | --- |
| **declarer** | human or machine | Always on decision |
| **router** | machine (Switchboard) | When Switchboard enabled |
| **evaluator** | machine (gate) | Always on decision |
| **authorizer** | human | After operator resolve |
| **executor** | human or machine | On execution record |

## Trust chain (happy path)

```text
DECLARER  → Prism / evaluation request
ROUTER    → Switchboard (optional hard DENY)
EVALUATOR → Glass decision receipt
AUTHORIZER → human APPROVE/REJECT if escalated
EXECUTOR  → EXECUTED | BLOCKED | FAILED
AUDIT     → JSONL + optional accountability report
```

## What this is not

- Not an agent framework or orchestrator  
- Not in-model “ethics” or prompt safety  
- Not deep packet inspection of undeclared intent  
- Not a guarantee against processes that never call the gate  
- Not cryptographic non-repudiation until an extension profile adds it  

## Related terms

See [Glossary](./glossary.md) for enums and field names.
