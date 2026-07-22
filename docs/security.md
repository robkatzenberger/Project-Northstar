# Security Model

This document is **engineering guidance**, not a formal certification or legal opinion.

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

## What is enforced in code (when used correctly)

| Control | Enforcement |
| --- | --- |
| Switchboard first-line DENY | Yes — short-circuits policy |
| Mandatory audit path | Yes — default air-gap |
| Single operator outcome | Yes — state machine |
| No EXECUTED unless audit AUTHORIZED | Yes — chain-verified |
| `executeAuthorized` skips sideEffect if not AUTHORIZED | Yes |
| In-memory forged AUTHORIZED | Rejected when audit is source of truth |

## Residual risks (not fully solved)

| Risk | Severity | Mitigation |
| --- | --- | --- |
| **Process never calls the gate** | Critical if assumed universal | OS/runtime mediation; admission controller pattern |
| **Anyone who can write the audit can forge ALLOW** | High | File ACLs; dedicated gate user; future hash/HMAC chain |
| **Operator id is free string (no authN)** | High in multi-user | Bind operators to SSO/mTLS/local allowlist + secrets |
| **Declared intent can lie** | Medium–High | By design metadata trust; pair with scope limits + monitoring |
| **Policy expression engine** | Medium | Trusted policy authors only; future pure DSL |
| **`allowEphemeral`** | Medium if misused | Never enable in production adapters |
| **Secrets in audit JSONL** | High if leaked | Redact; restrict file perms (`chmod 600`); no commit |

## Red team summary (post-hardening)

In-scope API/audit-chain attacks (forged memory status, double-resolve, pending execute, stub operator) **held**.

Residual **WARN/FAIL** class items: audit file write (A9), bypass outside library (A16), operator spoof (A18), semantic under-declaration (A12).

See `scripts/adversarial-redteam.mjs`.

## Air-gap recommendations

1. Run gate + executor as a **dedicated local process** or library call from a single mediation layer.  
2. Audit file writable **only** by that process user.  
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
2. `node bin/glass.mjs chain <receipt_id> --log <audit>`  
3. `node bin/glass.mjs incident ...` or `analyzeAccountability`  
4. Review Switchboard principal and credibility; adjust policy if needed.  
5. Rotate secrets if audit may contain them.  

## Disclosure

Public marketing should describe this as a **pre-execution checkpoint and evidence standard**, not as “AI safety solved” or universal agent containment.
