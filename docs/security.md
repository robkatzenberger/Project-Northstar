# Security Model

This document is **engineering guidance**, not a formal certification or legal opinion.

**Current scope:** the executor, audit-chain, operator, and `allowEphemeral` controls below describe the cooperative JavaScript TL-PX 0.1 reference. The Go HTTP service is likewise historical/cooperative and trusts caller-supplied actor identifiers. Neither is a 0.2 authority or PEP. The Rust authority accepted at `aed80e2` separately provides durable local state; later unaccepted candidates add execution lifecycle and Phase 4 hardening, but the independent Phase 4 review returned changes requested. The remediation working tree fixes Switchboard ordering, post-claim revocation at execution start, bounded audit admission, source-content and at-rest sink reconciliation, authenticated export acknowledgements, and executable provenance within the claimed distinct-identity profile. Rust still trusts its peer map, identity construction, renderer/adapter configuration, same-UID filesystem boundary, process isolation, manifest issuer assertions, and clock. The separate-identity 3.9 test remains unrun, so forced mediation is not established.

## Security goals

1. **Pre-execution decision** for declared intent  
2. **Deterministic policy** (no generative model in the decision path)  
3. **Identity-first access** via Switchboard  
4. **Fail-closed execution** when using the provided executor  
5. **Inspectable accountability** for human and machine parties  
6. **Air-gapped operation** possible (local audit, no cloud required)

## Trust boundaries

```text
┌─────────────────────────────────────────────┐
│ Untrusted: agent models, plugins, user code │
└────────────────────┬────────────────────────┘
                     │ must call gate + executor
┌────────────────────▼────────────────────────┐
│ Semi-trusted: adapter / runtime mediation   │
└────────────────────┬────────────────────────┘
                     │
┌────────────────────▼────────────────────────┐
│ Trusted computing base (TCB) for this ref:  │
│  · Gate process                             │
│  · Policy + Switchboard files (admin-owned) │
│  · Audit log integrity (who can write)      │
│  · Operator identity out-of-band authN      │
└─────────────────────────────────────────────┘
```

## What the JavaScript 0.1 reference enforces when used correctly

| Control | Enforcement |
| --- | --- |
| Switchboard first-line DENY | Yes — short-circuits policy |
| Mandatory audit path | Yes — default air-gap |
| Single operator outcome | Yes — state machine |
| No EXECUTED unless audit AUTHORIZED | Yes — chain-verified |
| `executeAuthorized` skips sideEffect if not AUTHORIZED | Yes |
| In-memory forged AUTHORIZED | Rejected when audit is source of truth |

## What the Rust local authority currently enforces

Within a trusted embedding, the accepted `aed80e2` Rust scope provides deterministic exact-match policy, immutable authenticated idempotency slots, authority-generated identifiers/nonces, durable SQLite decision/error/authorization state, exact Action Binding checks, revocation checks, and one atomic claim winner. The remediation working tree makes Switchboard refusal independent of tenant policy availability and binds that DENY to a canonical Switchboard configuration digest. Current reconciliation streams the bounded outbox and compares security-relevant sealed record content with its operational source row in addition to verifying canonical payloads, envelope bindings, hash/HMAC chain, source coverage, and exact export capacity.

Local 3.3 commit `c19b1d2` authenticates a connected local Unix peer from kernel UID/GID, maps it to opaque roles, and makes requester/executor entry points role-scoped. Pending cancellation is limited to the original requester, an operator on the policy-bound route, or an explicit emergency authority; the transition and one `tlpx.operator_action` evidence row commit atomically. Slice 3.8 removes public raw-principal mutation entry points, requires identity objects for authority mutations, and adds a bounded cooperative runner. Remediation protects executable provenance against the separately identified untrusted account through ownership/mode/directory checks, descriptor hashing, and an immediate inode recheck. The map, identity construction, socket permissions, service configuration, hostile same-UID processes, and direct-capability routes remain trusted deployment concerns.

The local authority uses two independent symmetric HMAC roles: authorization MAC and audit sealing. It does not produce an asymmetric authorization signature, and the reserved service-identity, operator-authentication, and tenant-trust purposes are not wired into authority behavior. Key custody/KMS and an external export service are not implemented. Export acknowledgements are audit-key HMACs bound to outbox id, chain hash, and acknowledgement time; `FileAuditExporter::operational_readiness()` must still be used for deploy/restore health because the database-only snapshot cannot prove sink bytes exist. The 3.8 demo remains cooperative, and a caller that can reach the protected capability directly can still bypass it.

## Residual risks (not fully solved)

| Risk | Severity | Mitigation |
| --- | --- | --- |
| **Process never calls the gate** | Critical if assumed universal | OS/runtime mediation; admission controller pattern |
| **Anyone who can write the audit can forge ALLOW** | Mitigated | Hash-chain + HMAC seal (A9); still protect seal key + OS ACLs |
| **Identity-map or socket misconfiguration** | High | Exact UID/GID mapping and fail-closed unknown peers in 3.3; protect socket, configuration, and service account; no SSO/mTLS |
| **Declared intent can lie** | Medium–High | By design metadata trust; pair with scope limits + monitoring |
| **Policy expression engine** | Low (mitigated) | Safe AST parser (no `new Function`); entire pack compiled before any decision; malformed policy fails load and issues no authorization |
| **0.2 hash canonicalization** | Low (bounded) | RFC 8785 JCS + UTF-16 key sort; lone surrogates fail closed. Implemented as a cross-language contract oracle; not used by the 0.1 audit seal path. |
| **`allowEphemeral`** | Medium if misused | Never enable in production adapters |
| **Secrets in audit JSONL** | High if leaked | Redact; restrict file perms (`chmod 600`); no commit |

## Red team summary (post-hardening)

In-scope API/audit-chain attacks (forged memory status, double-resolve, pending execute, stub operator) **held**.  
Raw unsealed audit forgery (A9) is rejected by integrity verify + HMAC seal.

Residual **WARN** class items: bypass outside library (A16), semantic under-declaration (A12), `allowEphemeral` misuse (A17).

See `implementations/javascript/scripts/adversarial-redteam.mjs`.

## Air-gap recommendations

1. Run gate + executor as a **dedicated local process** or library call from a single mediation layer.  
2. Audit file **and seal key** writable **only** by that process user (protect `var/.*.seal`).  
3. Policy and Switchboard owned by operators; not agent-writable.  
4. Prefer `executeAuthorized` over raw side effects.  
5. Separate interactive vs automation Switchboard principals and credibility.  

## Always-approve coding agents

If a coding harness is in always-approve mode, **harness prompts are not your control**. Either:

- Mediate all tool I/O through this gate, or  
- Accept that the harness can act without the Trust Layer  

Optional later: harness `PreToolUse` hooks as an adapter — not a substitute for air-gap design. See [Integration](./integration.md).

## Incident response

1. Locate `receipt_id` from application logs.  
2. From `implementations/javascript/`: `node bin/glass.mjs verify --log <audit>` then `chain`  
3. `node bin/glass.mjs incident ...` or `analyzeAccountability`  
4. Review Switchboard principal and credibility; adjust policy if needed.  
5. Rotate secrets if audit may contain them.  

## Disclosure

Public marketing should describe this as a **pre-execution checkpoint and evidence standard**, not as “AI safety solved” or universal agent containment.
