# Architecture

## Overview

Northstar is a **local reference implementation** of the **TL-PX 0.1** pre-execution trust checkpoint, plus product extensions:

- **Switchboard** — identity / whitelist / credibility / approval routing  
- **Air-gapped audit chain** — authorization derived only from append-only JSONL  
- **Fail-closed executor** — side effects only when AUTHORIZED  

It inherits decision philosophy from **APEX-Lite** (`ALLOW` / `REQUIRE_APPROVAL`) and pairs with **Prism** as an optional intent signal dialect.

## Logical layers

```text
┌──────────────────────────────────────────────────────────────┐
│  Clients                                                     │
│  CLI · library callers · future HTTP · optional harness hooks│
└────────────────────────────┬─────────────────────────────────┘
                             │ Evaluation Request
┌────────────────────────────▼─────────────────────────────────┐
│  Switchboard (src/switchboard.mjs)                           │
│  lookup → whitelist → action scope → credibility → routes    │
│  optional HARD DENY                                          │
└────────────────────────────┬─────────────────────────────────┘
                             │ enriched intent (or DENY outcome)
┌────────────────────────────▼─────────────────────────────────┐
│  Policy engine (src/policy.mjs)                              │
│  first-match deterministic rules                             │
└────────────────────────────┬─────────────────────────────────┘
                             │ ALLOW | REQUIRE_APPROVAL
┌────────────────────────────▼─────────────────────────────────┐
│  Glass core (src/glass.mjs)                                  │
│  decision / operator_action / execution records              │
│  mandatory auditPath (air-gap)                               │
└────────────────────────────┬─────────────────────────────────┘
                             │
┌────────────────────────────▼─────────────────────────────────┐
│  Chain auth (src/chain.mjs) + Executor (src/executor.mjs)    │
│  resolveAuthorizationFromAudit · executeAuthorized           │
└────────────────────────────┬─────────────────────────────────┘
                             │
┌────────────────────────────▼─────────────────────────────────┐
│  Audit JSONL (src/audit.mjs) + Accountability                │
│  (src/accountability.mjs)                                    │
└──────────────────────────────────────────────────────────────┘
```

## Module map (`implementations/javascript/src/`)

| Module | Responsibility |
| --- | --- |
| `prism.mjs` | Create Prism-compatible signals; map to evaluation intent |
| `switchboard.mjs` | Principal registry, routing, hard gates, credibility helpers |
| `policy.mjs` | Load/parse policy; evaluate conditions/rules |
| `glass.mjs` | evaluateIntent, resolveEscalation, recordExecution |
| `chain.mjs` | Audit-derived authorization; single-outcome guards |
| `executor.mjs` | Fail-closed `executeAuthorized` |
| `audit.mjs` | Sealed JSONL append/read/verify; chain filter |
| `accountability.mjs` | Findings / dual human-machine report |
| `validate.mjs` | Structural validators for TL-PX records |
| `standard.mjs` | Standard id/version/record type constants |
| `ids.mjs` | UUID, receipt id (time + entropy) |
| `index.mjs` | Public exports |

## CLI

`implementations/javascript/bin/glass.mjs` — operator and automation entrypoint. See [CLI Reference](./cli-reference.md).

## Configuration surface

| Artifact | Role |
| --- | --- |
| `implementations/javascript/config/policy.yaml` | Deterministic rules (JS ref) |
| `implementations/javascript/config/switchboard.json` | Identity router |
| `implementations/javascript/examples/*` | Fixtures |
| `implementations/{go,java,…}/` | Language ports |
| `schemas/tlpx-0.1/*` | JSON Schema documents (shared) |
| `var/*.jsonl` | Runtime audits at monorepo root (gitignored) |

## Data flow (sequence)

```text
1. Client builds intent (Prism or flat)
2. evaluateIntent:
   a. require auditPath
   b. routeThroughSwitchboard (if enabled)
   c. if switchboard.gate → DENY/escalate outcome
   d. else evaluateRules(policy)
   e. append tlpx.decision
3. If REQUIRE_APPROVAL:
   a. human resolveEscalation (once)
   b. append tlpx.operator_action
4. executeAuthorized:
   a. resolveAuthorizationFromAudit
   b. if not AUTHORIZED → record BLOCKED, return (no sideEffect)
   c. else run sideEffect; record EXECUTED or FAILED
5. Optional: incident / accountability report
```

## Decision ownership

| Stage | Owner of outcome |
| --- | --- |
| Access / identity | Switchboard |
| Intent vs policy rules | Policy engine |
| Human judgment on escalate | Operator action |
| Whether side effect runs | Executor + audit chain |

## Relationship to public Trust Layer story

| Public name | This repo |
| --- | --- |
| Prism | `prism.mjs` + examples |
| Glass | product framing + gate implementation |
| APEX-Lite | policy semantics + early lineage |
| Separation of powers (standards vs ops) | Spec in `docs/standard/`; commercial Glass may extend |

## Non-goals in architecture

- Multi-region HA  
- Built-in operator SSO  
- Cryptographic token profile (roadmap extension)  
- In-process sandbox for untrusted policy authors  
- Automatic wrapping of third-party agent binaries  

## Extension points

1. **New principals** — Switchboard config only  
2. **New rules** — policy YAML  
3. **New clients** — call the same JS API or CLI  
4. **HTTP profile** — wrap glass/chain/executor without changing record types  
5. **Signed receipts** — add fields; keep TL-PX core intact for conformance  

## See also

- [Concepts](./concepts.md)  
- [SPEC-v0.1](./standard/SPEC-v0.1.md)  
- [Security](./security.md)  
- [Integration](./integration.md)  
