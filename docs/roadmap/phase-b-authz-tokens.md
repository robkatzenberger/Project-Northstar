# Phase B — Signed short-lived authorization tokens

## Goal

Make **AUTHORIZED** portable: another process, service, or agent can verify permission **without** sharing the issuer’s audit file.

Audit remains the **issuer’s source of truth**.  
The token is a **verifiable snapshot** of a successful authorization for a bounded action and time.

---

## Why

| Today | Problem |
| --- | --- |
| `authorization_status` on local audit | Only trusted if you have that log + seal key |
| Downstream tool / Agent B | Cannot independently verify |

Tokens turn “trust my gate” into “verify this signature / MAC.”

---

## Draft token shape (`tlpx.authz_token` v0)

Conceptual fields (to implement):

```json
{
  "typ": "tlpx.authz_token",
  "v": "0.1",
  "iss": "tlpx-issuer-id",
  "iat": "2026-08-07T00:00:00Z",
  "exp": "2026-08-07T00:05:00Z",
  "receipt_id": "rcpt_…",
  "intent_hash": "sha256:…",
  "decision": "ALLOW",
  "authorization_status": "AUTHORIZED",
  "actor": "agent.support.mailer",
  "action": "send_email",
  "target": "external_user",
  "scope": {
    "max_risk": "medium",
    "data_classes": ["PII"]
  },
  "sig": "…"
}
```

### Field notes

| Field | Purpose |
| --- | --- |
| `iss` | Which gate/issuer signed |
| `exp` | Short TTL (minutes, not days) |
| `receipt_id` | Link back to sealed audit |
| `intent_hash` | Bind token to declared intent snapshot |
| `action` / `target` / `scope` | Limit what the token authorizes |
| `sig` | Signature or HMAC over canonical body |

**Algorithms (candidates):**

- Ed25519 signature (prefer for multi-party verify)  
- Or HMAC with shared secret (simpler air-gap pair, weaker multi-verifier story)  

---

## Lifecycle

```text
1. evaluateIntent → ALLOW or (REQUIRE_APPROVAL + APPROVE)
2. authorization_status == AUTHORIZED
3. issueAuthzToken(decision) → token string / JSON
4. executor or peer verifies token (iss, exp, sig, action match)
5. perform side effect
6. recordExecution (issuer audit) with token id / hash optional
```

---

## Verification rules (draft MUST)

Verifier MUST reject if:

- `exp` in the past  
- `sig` invalid  
- `iss` not in trusted issuers list  
- requested action not equal (or not subset of) token `action` / `scope`  
- `authorization_status` ≠ `AUTHORIZED`  

Verifier SHOULD:

- Log verify result  
- Prefer one-time use tracking when possible (harder offline)  

---

## Relationship to sealed audit

| Artifact | Role |
| --- | --- |
| Sealed JSONL | Issuer history, chain integrity, incident review |
| Authz token | Portable capability for this action, short-lived |

Compromised seal key ≠ same as signing key if keys are separated (recommend separation in production).

---

## Minimal MVP (implementation sketch)

1. `docs` stay here as the contract draft  
2. JS: `issueAuthzToken` / `verifyAuthzToken` using Ed25519 or first-pass HMAC via env keys  
3. `executeAuthorized` optional mode: require valid token  
4. Go `tlpxd`: `POST /v1/token` issue, `POST /v1/token/verify`  
5. Conformance fixtures for token round-trip  

---

## Non-goals for first token MVP

- Full OAuth/OIDC  
- Refresh tokens  
- Cross-federation PKI at internet scale  
- Binding to hardware attestation (later)  

---

## Open questions

1. Ed25519 vs HMAC for v0?  
2. Is intent_hash over full `original_intent` or a subset?  
3. Should DENY ever issue a token? (**No.**)  
4. Token revocation list vs short TTL only for v0?  
