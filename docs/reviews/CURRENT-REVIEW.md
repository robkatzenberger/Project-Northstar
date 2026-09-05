# Current review packet

**Disposition:** **CHANGES REQUESTED**

**Scope:** Bounded macOS separate-identity marker PEP and its deadline/evidence remediation

**Tested source candidate (C3):** `82f5cd6cc505cc64bdea73eebeb535c2a1b4cd11`

**Tested Git tree:** `903ab8661d4916ba26f5168213480d9c664235c9`

**Frozen runtime parent (C2):** `77d77b8745b342d8f61b326a468c53523fe945a1`

**Preparation base:** `32c049e03d2c14729a8dc71205aef80e221e30ac`

**Accepted implementation:** `aed80e2527f05a3730b1057f2d90c55a6c3eb646`

The full non-administrator matrix passed in a clean isolated checkout of C3. Robert then ran `sudo ./run-northstar-3.9.sh` in that checkout on 2026-09-05; report `172840` records PASS at the same exact source hash with temporary-account cleanup status 0. This documentation and the reports form the evidence-only child packet; the tested source commit remains unchanged. Publication and independent acceptance remain pending.

The [earlier reviewer disposition](./restricted-marker-review-disposition-2026-09-05.md) requested the frozen source and exact-candidate evidence. In the owner's subsequent supplied response, the reviewer independently checked C3's source/tree, deadline sampling, replay behavior, socket budget order, historical status labels, and excluded egress draft. The reviewer reports running the PEP oracle (one positive and 15 negative cases) and all seven production-time tests successfully, but did not rerun the full matrix or administrator gate. The reviewer kept **CHANGES REQUESTED** because C3 then lacked administrator evidence. Report `172840` was produced afterward and has not yet received the reviewer's disposition.

## Candidate contents

| Packet | Contents |
| --- | --- |
| C1 — Rust remediation | Production clock sampling after the write transaction; durable time floor; completion/reconciliation retry semantics; claim, approval, and lease lock-wait regressions |
| C2 — PEP evidence and harness | Exactly five canonical records, full linkage and tested-binary binding, BOM rejection, negative oracle tests, protected retained-copy/cleanup checks |
| C3 — Current documentation | Accurate accepted/candidate boundaries, concise navigation, reviewer disposition, working-tree builder evidence, and delivery plan; future egress design remains excluded |

The [delivery plan](../GITHUB-DELIVERY-PLAN.md) supplies the exact path groups. The [working-tree builder report](../../tests/reports/review-followup-builder-verification-2026-09-05.md) remains a historical record. The new reports below name C3 directly.

## Exact-candidate evidence

| Artifact | Result and attribution |
| --- | --- |
| [Full non-administrator builder matrix](../../tests/reports/exact-candidate-82f5cd6-builder-verification-2026-09-05.md) | Codex ran both Rust modes (126/39 passed; modes overlap), both strict Clippy modes, JS/oracles, conformance, technical, evidence, repeated remediation/handoff, and package/operations checks at clean C3. One dedicated volume probe remained ignored; four historical JS warnings remain. |
| [Administrator report 172840](../../tests/reports/slice-3.9-administrator-gate-2026-09-05-172840.md) | Robert ran the macOS gate at clean C3; PASS, cleanup status 0. |
| [Retained canonical JSONL](../../tests/reports/slice-3.9-administrator-gate-2026-09-05-172840.evidence.jsonl) | Five ordered, linked records: DENY, ALLOW, authorization, claim, and completed execution. The tested-source oracle passed again when Codex verified the retained file. |

**Tested binary SHA-256:** `sha256:7c6332d3066b4b0bab5dd12fa0354d9d882a84899a71255494fab5e4673fad85`

**Retained JSONL SHA-256:** `sha256:343d292c4fb3131b8e563678892ce181646f6d9df8618bc63ac5a06926dacc19`

Codex checked these digests against the saved report and binary, verified all 224 tracked source-file fingerprints were unchanged, and confirmed the three temporary accounts/UIDs were absent and no `/tmp/northstar-3.9.*` harness scratch directory remained. The gate generated only its new report and JSONL in the tested checkout. Copies retained here are byte-identical to those outputs. No runtime, test, schema, dependency, configuration, or harness change is part of this evidence packet.

The non-administrator report predates the administrator run; its then-pending administrator statements are preserved as point-in-time evidence. This index records the subsequent result. The source candidate's stale Rust README statements about an unfinished documentation commit and working-tree-only evidence are corrected only in this descendant's documentation.

## Remaining review step

The existing reviewer must assess source `82f5cd6cc505cc64bdea73eebeb535c2a1b4cd11` with the named non-administrator and administrator artifacts. Keep **CHANGES REQUESTED** until the reviewer supplies an actual scoped acceptance. C2 identifies the runtime parent; the evidence child preserves results; neither replaces C3 as the tested source. Do not rerun or alter the source merely to put its own hash inside itself.

## Historical evidence

| Artifact | What it establishes |
| --- | --- |
| [Accepted 2.3d review](../../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md) | Independent acceptance at `aed80e2` for the bounded local evaluate/issue/claim MVP |
| [Administrator report 064717](../../tests/reports/slice-3.9-administrator-gate-2026-09-04-064717.md) and [JSONL](../../tests/reports/slice-3.9-administrator-gate-2026-09-04-064717.evidence.jsonl) | Bounded marker run at source `f025332`, preserved at `32c049e`; does not close the later production-time finding |
| [Phase 4 disposition](./phase-4-independent-review-disposition-2026-08-28.md) | Prior independent findings and remediation map; Phase 4 remains unaccepted |
| [Evidence index](../../tests/README.md) | Historical reports and reproduction inputs, including superseded candidates |

`4ca86d6` is a superseded historical candidate, not current status. This packet does not establish Section 3/Phase 4 acceptance, production readiness, network/egress mediation, hostile same-UID containment, or universal forced mediation. Even a future reviewer acceptance is limited to the specifically named macOS marker profile.
