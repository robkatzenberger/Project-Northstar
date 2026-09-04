# Slice 3.9 administrator gate — 2026-09-04T05:49:05Z

**Disposition:** PASS
**Base HEAD:** `4ca86d61ab61e8350bfce9ece8771bcb0fea870e`
**Candidate state:** clean exact commit 4ca86d61ab61e8350bfce9ece8771bcb0fea870e
**Temporary-account cleanup status:** 0

This is a local administrator-backed acceptance run for the bounded macOS separate-identity profile. It is not independent review, Section 3 acceptance, production-readiness evidence, or a claim of universal forced mediation.

## Exact output

```text
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
PASS  authorization reuse cannot repeat the side effect
PASS  new request cannot reopen the one-shot target
PASS  caller cannot inject an alternate path or argv
PASS  restricted agent cannot bind a replacement endpoint
Restricted PEP evidence crosscheck

  PASS  tlpx.decision
  PASS  tlpx.decision
  PASS  tlpx.authorization
  PASS  tlpx.authorization_claim
  PASS  tlpx.execution

Restricted PEP evidence: 5 rows, 0 failures
PASS  sealed execution and denial evidence independently reconciles and validates
PASS  restart does not reopen completed target
PASS  expired authorization cannot start the side effect
PASS  slice 3.9 separate-identity restricted-agent acceptance
```
