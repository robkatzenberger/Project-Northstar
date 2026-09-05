# Current review packet

**Disposition:** **CHANGES REQUESTED**

**Scope:** Bounded macOS separate-identity marker PEP and its deadline/evidence remediation

**Review candidate:** C3, the commit containing this documentation packet; resolve its full hash with `git rev-parse HEAD` in the frozen C3 checkout

**Frozen runtime parent (C2):** `77d77b8745b342d8f61b326a468c53523fe945a1`

**Preparation base:** `32c049e03d2c14729a8dc71205aef80e221e30ac`

**Accepted implementation:** `aed80e2527f05a3730b1057f2d90c55a6c3eb646`

Robert authorized the three local commits and a clean non-administrator rerun. The administrator gate and publication remain separate. The source candidate cannot contain its own Git hash; the exact C3 hash and run results will be supplied after commit and retained in a separate evidence packet.

The [latest reviewer disposition](./restricted-marker-review-disposition-2026-09-05.md) requires one exact source commit, a complete rerun, and new administrator evidence. No additional reviewer or architecture expansion is needed to prepare that packet.

## Candidate contents

| Packet | Contents |
| --- | --- |
| C1 — Rust remediation | Production clock sampling after the write transaction; durable time floor; completion/reconciliation retry semantics; claim, approval, and lease lock-wait regressions |
| C2 — PEP evidence and harness | Exactly five canonical records, full linkage and tested-binary binding, BOM rejection, negative oracle tests, protected retained-copy/cleanup checks |
| C3 — Current documentation | Accurate accepted/candidate boundaries, concise navigation, reviewer disposition, working-tree builder evidence, and delivery plan; future egress design remains excluded |

The [delivery plan](../GITHUB-DELIVERY-PLAN.md) supplies the exact path groups. Code and test bytes are unchanged from the [2026-09-05 working-tree builder report](../../tests/reports/review-followup-builder-verification-2026-09-05.md); its [fingerprints](../../tests/reports/review-followup-builder-verification-2026-09-05.sha256) identify those bytes. That report is not evidence for a future exact commit.

## Verification required after commit

- Provide the full hash of C3 after freezing C1–C3; this hash, including the curated documentation, is the review/test target. C2 names the exact runtime parent and is not a substitute for C3 in the final review.
- Use a clean isolated checkout of that hash with its history and test fixtures available.
- Run both Rust modes, formatting, both strict Clippy modes, all JS/oracle/conformance/technical/red-team/evidence checks, repeated remediation/handoff checks, and operations/package checks documented in [testing](../testing.md).
- Build the production-feature `tlpx-run`, then run the separately authorized macOS administrator gate. Retain the dated report, canonical JSONL, binary/evidence digests, and cleanup result.
- Record the source hash and new evidence paths in this index in an evidence-only descendant. A source commit cannot contain its own full Git hash; the descendant records the source it tested without changing that source.
- Ask the existing reviewer to assess only that exact source candidate and its evidence packet. Keep **CHANGES REQUESTED** until the reviewer provides a new scoped disposition.

## Historical evidence

| Artifact | What it establishes |
| --- | --- |
| [Accepted 2.3d review](../../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md) | Independent acceptance at `aed80e2` for the bounded local evaluate/issue/claim MVP |
| [Administrator report 064717](../../tests/reports/slice-3.9-administrator-gate-2026-09-04-064717.md) and [JSONL](../../tests/reports/slice-3.9-administrator-gate-2026-09-04-064717.evidence.jsonl) | Bounded marker run at source `f025332`, preserved at `32c049e`; does not close the later production-time finding |
| [Phase 4 disposition](./phase-4-independent-review-disposition-2026-08-28.md) | Prior independent findings and remediation map; Phase 4 remains unaccepted |
| [Evidence index](../../tests/README.md) | Historical reports and reproduction inputs, including superseded candidates |

`4ca86d6` is a superseded historical candidate, not current status. This packet does not establish Section 3/Phase 4 acceptance, production readiness, network/egress mediation, hostile same-UID containment, or universal forced mediation. Even a future reviewer acceptance is limited to the specifically named macOS marker profile.
