# Slice 3.9 administrator gate — 2026-09-05T17:28:40Z

**Disposition:** PASS
**Base HEAD:** `82f5cd6cc505cc64bdea73eebeb535c2a1b4cd11`
**Candidate state:** clean exact commit 82f5cd6cc505cc64bdea73eebeb535c2a1b4cd11
**Tested binary SHA-256:** `sha256:7c6332d3066b4b0bab5dd12fa0354d9d882a84899a71255494fab5e4673fad85`
**Temporary-account cleanup status:** 0
**Retained canonical evidence:** [slice-3.9-administrator-gate-2026-09-05-172840.evidence.jsonl](./slice-3.9-administrator-gate-2026-09-05-172840.evidence.jsonl)
**Retained evidence SHA-256:** `sha256:343d292c4fb3131b8e563678892ce181646f6d9df8618bc63ac5a06926dacc19`

This is a local administrator-backed acceptance run for the bounded macOS separate-identity profile. It is not independent review, Section 3 acceptance, production-readiness evidence, or a claim of universal forced mediation.

## Exact output

```text
TESTED_BINARY_SHA256 sha256:7c6332d3066b4b0bab5dd12fa0354d9d882a84899a71255494fab5e4673fad85
PASS  staged tested binary matches reported SHA-256
PASS  restricted agent cannot write protected target directly
PASS  alternate shell route cannot write protected target
PASS  alternate symlink path cannot write protected target
PASS  restricted agent cannot modify PEP code
PASS  PEP configuration, key, and endpoint directory resist agent access
PASS  second UID cannot use the mapped agent identity
PASS  caller-supplied principal field is rejected
PASS  unsupported action is denied before authorization
PASS  authenticated exact request completes
PASS  PEP identity owns the protected side effect
PASS  existing protected target blocks a repeat request before evaluation
PASS  existing protected target blocks a new request
PASS  consumed authorization cannot be replayed after PEP-owned target deletion
PASS  caller cannot inject an alternate path or argv
PASS  unauthenticated peer does not consume the authenticated-work budget
PASS  restricted agent cannot bind a replacement endpoint
Restricted PEP evidence crosscheck

  PASS  tlpx.decision DENY
  PASS  tlpx.decision ALLOW
  PASS  tlpx.authorization AUTHORIZED_UNCLAIMED
  PASS  tlpx.authorization_claim CLAIMED
  PASS  tlpx.execution COMPLETED

Restricted PEP evidence: 5 rows, 0 failures
Restricted PEP evidence crosscheck

  PASS  tlpx.decision DENY
  PASS  tlpx.decision ALLOW
  PASS  tlpx.authorization AUTHORIZED_UNCLAIMED
  PASS  tlpx.authorization_claim CLAIMED
  PASS  tlpx.execution COMPLETED

Restricted PEP evidence: 5 rows, 0 failures
PASS  sealed execution and denial evidence independently reconciles and validates
PASS  restart does not reopen completed target
PASS  expired authorization cannot start the side effect
PASS  slice 3.9 separate-identity restricted-agent acceptance
```
