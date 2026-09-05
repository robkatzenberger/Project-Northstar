# Restricted marker candidate — reviewer disposition

**Received from the owner:** 2026-09-05

**Source:** Reviewer response supplied by Robert in the project conversation; reviewer model/name not specified

**Disposition:** **CHANGES REQUESTED** for Phase 4, slice 3.9 acceptance, and the uncommitted candidate

**Accepted implementation boundary:** `aed80e2527f05a3730b1057f2d90c55a6c3eb646`

This records the supplied review. It is not a new builder acceptance decision or a claim that the builder independently reproduced the reviewer’s work. Earlier dated reports remain unchanged.

## Reviewer findings and verification scope

The reviewer recognizes `f025332` / administrator report `064717` as real builder evidence for the bounded macOS marker profile. The reviewer does not accept that source commit because production transition time was sampled before SQLite write-transaction acquisition. The reviewer reports reproducing a P1 deadline crossing under lock wait.

The reviewer considers the current remediation direction appropriate: production time sampled after `BEGIN IMMEDIATE` and clamped to the durable floor; consumed replay denied after PEP-owned marker deletion; authentication before work-budget consumption; retained canonical JSONL linked to the tested binary digest; and corrected scope claims.

The reviewer independently reran the PEP-oracle suite and reports that it passed. The reviewer did not independently rerun Cargo, Clippy, or the administrator gate. A preview of uncommitted source and a builder log do not satisfy the reviewer’s exact-commit requirement.

## Required next packet

1. The owner authorizes committing the intended tree; the reviewer will not author that commit.
2. Supply the full source commit hash.
3. Review that exact commit for time-after-lock, consumed replay, evidence binding, socket work-budget ordering, and current documentation. `4ca86d6` must not be presented as live status.
4. Run the full non-administrator matrix and `sudo ./run-northstar-3.9.sh` from a clean checkout of that hash. The reviewer may run the non-administrator part if asked; administrator evidence must come from an actual administrator run.
5. Preserve the new exact-candidate reports, canonical JSONL, binary digest, and cleanup result. Obtain the reviewer’s disposition against the named source commit.

The requested next step is a frozen candidate and evidence, not another reviewer or another feature slice. See [CURRENT-REVIEW.md](./CURRENT-REVIEW.md) for the packet’s current preparation state.

## Maximum acceptance scope if the packet passes

The reviewer’s proposed future acceptance is limited to a bounded macOS separate-identity marker PEP at a named commit with named local administrator evidence. That conditional statement is not present acceptance.

Phase 4, Section 3, network/egress mediation, hostile same-UID containment, and production readiness remain open even if this bounded profile is subsequently accepted. The accepted `aed80e2` baseline has not moved in this review.
