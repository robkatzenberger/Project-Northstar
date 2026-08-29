# Phase 4 independent review disposition

**Date received:** 2026-08-28
**Reviewed security-code target:** `64d08203c2f30964b176be8872b390ba3c28e5e6`
**Review-package commit:** `d51e46ce4cf87e4ab339fd1d70aac5e078c3e7e0`
**Independent disposition:** **CHANGES REQUESTED**
**Remediation status:** builder tests pass on the pre-commit working tree; a named remediation commit and independent re-review are still required

This records the external review supplied to the project owner. It is a disposition and remediation map, not a replacement for the reviewer's findings and not acceptance. Historical Phase 4 reports remain point-in-time builder evidence for the original commits.

## Finding disposition

| Finding | Disposition after remediation | Evidence or remaining boundary |
| --- | --- | --- |
| H1 — 3.9 is unproven | **OPEN SEPARATE GATE** | The administrator-backed dedicated-identity run remains unperformed. The macOS harness now requires three explicit, distinct, inactive test accounts and refuses shared/system identities. Ordinary Rust tests are not 3.9 evidence. |
| H2 — JS/Go are live 0.1 paths | **CLAIM CORRECTED** | Root and implementation docs plus source headers label both paths historical/cooperative 0.1-era behavior, not 0.2 authority/PEP endpoints. Their 0.1 behavior was not silently rewritten. |
| H3 — exporter capacity did not stop authority | **REMEDIATED, RE-REVIEW REQUIRED** | Every evidence enqueue computes the exact complete canonical export size inside the state transaction and returns `AUDIT_CAPACITY_EXCEEDED` before state commit. Reconciliation streams rows rather than collecting the complete outbox. |
| M1 — “authorization signing” was symmetric HMAC | **REMEDIATED** | Runtime types, columns, key purpose, revocation scope/reason, messages, and docs now say authorization MAC/HMAC. No asymmetric or portable-ticket claim is made. |
| M2 — three of five required key roles were unused | **REMEDIATED** | The local authority profile requires only the two exercised roles: authorization MAC and audit sealing. Service identity, operator authentication, and tenant trust are reserved future purposes. |
| M3/M10 — revocation after claim did not stop start/spawn | **REMEDIATED THROUGH START BOUNDARY** | Durable execution start transactionally rechecks authorization, both principals, policy, tenant, environment, capability, and authorization-MAC-key revocations. Sequential and two-connection race tests cover the boundary. Active cancellation after a successful start remains deferred. |
| M4 — path hash-to-spawn and wrong marker scope | **REMEDIATED WITHIN DISTINCT-IDENTITY PROFILE** | Marker scope is bound in the restricted profile. The cooperative runner requires root/PEP ownership, non-group/world-writable file and directory provenance, hashes an `O_NOFOLLOW` descriptor, retains it, and rechecks pathname-to-inode identity immediately before spawn. Hostile same-UID containment and proof of the marker inode creator remain non-claims; 3.9 is still unrun. |
| M5 — handoff wrapper is optional | **CLAIM CORRECTED** | The wrapper is documented as optional co-presentation preflight. Ordinary evaluation may name B without this wrapper; B is authenticated later at claim. No universal handoff mediation or portable credential is claimed. |
| M6 — reconciliation checked source IDs only | **REMEDIATED, RE-REVIEW REQUIRED** | Reconciliation compares security-relevant operational evaluation, operator-action, authorization, claim, and terminal-execution fields to sealed record content and fails on source-row tampering. |
| M7 — database acknowledgement was not sink proof | **REMEDIATED** | Export acknowledgement is an audit-key HMAC over outbox id, chain hash, and acknowledgement time. Combined readiness opens and verifies the actual sink and rejects acknowledged rows whose bytes are absent. |
| M8 — `create_new` sibling lock was weak | **REMEDIATED TO AN HONEST COOPERATIVE CLAIM** | A persistent file is held with nonblocking advisory `flock`, and its descriptor/path inode identity is checked. It coordinates cooperating exporters; hostile same-UID isolation is explicitly not claimed. |
| M9 — policy selection preceded Switchboard | **REMEDIATED** | Switchboard refusal now commits before tenant policy selection and binds a canonical Switchboard configuration digest. Docs also state that validating the six-stage precedence array does not make Rust a generic six-stage interpreter. |
| M11 — frozen 0.1 DENY schema defect | **OPEN, RECORDED** | No 0.1 history was rewritten. The live 0.1 defect remains explicit and must be resolved on the separately versioned line. |
| M12 — Go looked like a parallel authority | **CLAIM CORRECTED** | Go is labeled a historical cooperative 0.1-era secondary that trusts caller-supplied actor identifiers; it is not a 0.2 authority, adapter endpoint, or PEP. |
| L1 — large `authority.rs` | **DEFERRED** | No security-sensitive drive-by split was attempted in this remediation. Reviewability debt remains. |
| L2 — `IMPLEMENTED` looked like acceptance | **CLARIFIED** | The build sheet visibly states that `IMPLEMENTED` is only named builder evidence and that the acceptance column is authoritative. Older pre-commit 2.4/3.1 rows remain `PLANNED`. |
| L3 — review package was not blob verification | **CLAIM CORRECTED** | The package is consistently described as an index and reproduction aid, not security or blob-level verification. |
| L4 — rerun multipliers existed only in prose | **REMEDIATED** | `tests/redteam-phase-4-remediation.sh` executes five focused reviewer-reproducer rounds and twenty handoff rounds by default. |
| L5 — macOS harness reused shared accounts | **REMEDIATED IN HARNESS; RUN OPEN** | Shared/system accounts are rejected and dedicated explicit identities are mandatory. The administrator run is still not performed. |
| L6 — schema startup did not re-fingerprint indexes/checks | **REMEDIATED** | Startup verifies critical unique indexes, the exact idempotency partial-index predicate, and key table constraints including acknowledgement invariants. Dropped and weakened-index tests fail closed. |
| L7 — adapter tests use one eUID | **NON-CLAIM RESTATED** | These fixtures test mapping and contract behavior, not distinct-peer OS isolation. The separate-identity gate remains 3.9. |

## Builder verification completed before commit

- Rust formatting and strict Clippy pass.
- Rust `cargo test --all-targets --offline` passes 125 tests, including the ordinary restricted-service socket test. The latter required desktop sandbox permission to bind a temporary local socket; it was not the administrator-backed script.
- Phase 4 remediation red team passes five focused rounds and twenty handoff rounds.
- JavaScript combined suite passes 935 checks; 0.1 conformance passes 47/47; technical test passes 29/29; red team reports 16 PASS, 0 FAIL, 4 documented WARN; Rust evidence crosscheck passes 10/10.
- Operations and historical review-package checks pass.

The password/sudo-backed `scripts/restricted-agent-acceptance.sh` was deliberately not run at the product owner's direction. Therefore this document does not close 3.9, Section 3, Phase 4, forced mediation, or independent acceptance.

## Next gate

Commit this remediation locally, rerun the complete matrix against the exact commit, record the commit and results in a new builder report, then send that named commit for independent re-review. Keep the dedicated-identity 3.9 acceptance run as a separately reported administrator gate.
