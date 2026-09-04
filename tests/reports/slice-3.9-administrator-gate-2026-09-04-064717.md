# Slice 3.9 administrator gate — 2026-09-04T06:47:17Z

**Disposition:** PASS
**Base HEAD:** `f0253328e00fd1868e4de811298b8ad2e91d230d`
**Candidate state:** clean exact commit f0253328e00fd1868e4de811298b8ad2e91d230d
**Tested binary SHA-256:** `sha256:7c483a6eb95e4defd1ad4b2999cd23b520c90324450bbc7222565145768f5470`
**Temporary-account cleanup status:** 0
**Retained canonical evidence:** [slice-3.9-administrator-gate-2026-09-04-064717.evidence.jsonl](./slice-3.9-administrator-gate-2026-09-04-064717.evidence.jsonl)
**Retained evidence SHA-256:** `sha256:0144d98e9f5f3b4a2504d9fe4e4760eefd78985eaa05d04a3a440ee53a5eb20e`

This is a local administrator-backed acceptance run for the bounded macOS separate-identity profile. It is not independent review, Section 3 acceptance, production-readiness evidence, or a claim of universal forced mediation.

## Exact output

```text
TESTED_BINARY_SHA256 sha256:7c483a6eb95e4defd1ad4b2999cd23b520c90324450bbc7222565145768f5470
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
