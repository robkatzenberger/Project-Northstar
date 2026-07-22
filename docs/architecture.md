# Glass architecture notes

Status: **planning**. No runtime code in this repo yet.

## Layers

```text
┌─────────────────────────────────────────────────────────┐
│  Agent / app / orchestrator                             │
│  - forms action                                         │
│  - emits Prism signal                                   │
│  - waits for authorization                              │
└───────────────────────────┬─────────────────────────────┘
                            │ Prism intent (metadata only)
                            ▼
┌─────────────────────────────────────────────────────────┐
│  Glass (Northstar product)                              │
│  1. Ingest + validate signal                            │
│  2. Identity / scope binding                            │
│  3. Gates + policy evaluation (deterministic)           │
│  4. Risk / approval routing                             │
│  5. HITL queue (if required)                            │
│  6. Issue auth decision (+ future signed token)         │
│  7. Append audit / receipts                             │
└───────────────────────────┬─────────────────────────────┘
                            │ ALLOW | REQUIRE_APPROVAL | (future DENY?)
                            ▼
┌─────────────────────────────────────────────────────────┐
│  Execution runtime                                      │
│  - proceeds only with valid authorization               │
│  - may re-check token / revocation                      │
└─────────────────────────────────────────────────────────┘
```

## Inheritance from APEX-Lite

Carry forward unless we deliberately version a break:

| Concept | APEX-Lite today | Glass direction |
| --- | --- | --- |
| Decision model | `ALLOW`, `REQUIRE_APPROVAL` | Same core; map escalation cleanly; DENY is an explicit product choice |
| Control mode | `ALLOW_OR_ESCALATE` | Keep transparency-rewarded escalation as default philosophy |
| Policy | Simple YAML rules + keyword gates | Policy packs, env scoping, stronger expression language later |
| Receipt | `apex-lite.receipt` JSON | Stable, versioned receipt; possible `glass.receipt` with backward mapping |
| Audit | Append-only JSONL | Same semantics; durable store + export for enterprise |
| Console | Local operator UI | Product ops console / APIs |
| Notifications | Optional SMS config | Pluggable notifiers |

Local reference: `~/APEX-Lite`  
Upstream OSS: https://github.com/Trust-Layer-AI/Trust-Engine

## Prism contract (do not invent casually)

v0.1 core fields (from Prism docs):

- `prism_id`
- `timestamp`
- `agent`
- `intent_summary`
- `prism_version` (e.g. `prism_v0.1`)

Glass may accept **enriched evaluation intents** (risk, action, data_classes, etc.) as APEX-Lite does today, while remaining able to bind a pure Prism envelope. Document any required extension fields as **Glass request schema**, not as Prism core.

Upstream: https://github.com/Trust-Layer-AI/prism-protocol

## Suggested first implementation slices (later)

1. **Contract pack** — JSON schemas for Glass evaluate request, decision, receipt.
2. **Core evaluator port** — TypeScript (or shared) port of APEX-Lite evaluate path + fixtures.
3. **HTTP service** — `POST /v1/evaluate`, `POST /v1/operator-action`, `GET /v1/audit`.
4. **Prism adapter** — map pure Prism v0.1 → Glass evaluation intent.
5. **Ops MVP** — queue + single-operator resolution + audit export.

Do not start slice 2+ until slice 1 (or an agreed subset) is written down.

## Explicit out of scope for v0 code

- Cryptographic token issuance (design first)
- Multi-region HA
- Full RBAC admin product
- Model-based risk scoring as primary gate
