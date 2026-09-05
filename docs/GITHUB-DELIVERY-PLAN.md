# GitHub delivery package

**Final sweep:** 2026-09-05

**Owner decision:** Preserve evidence history; curate current code and documentation.

**Recommendation:** Publish one review branch containing the existing history plus the final documentation/ignore cleanup, then open one draft PR. The code, tests, schemas, fixtures, lockfile, and named evidence belong in this package. Generated files and private drafts stay local. This sweep does not commit, push, open a PR, or merge.

## Exact publication boundary

| Item | Verified value |
| --- | --- |
| Destination | `robkatzenberger/Project-Northstar`, private, active repository; visibility unchanged |
| Default branch | `main` |
| GitHub `main` | `7a0b371e1307739e465f8c5bd313ef9372adc9be`, read from GitHub during this sweep |
| Audited local tip | `b6d597eaff9f1193b155c2246f623791cde4d613` |
| Outgoing history at that tip | 61 commits ahead, zero behind; 168 paths differ from GitHub `main` |
| Current tracked tree | 228 files, approximately 1.9 MB; no binary blobs, Git submodules, or symlinks |
| Final cleanup | 17 documentation/ignore files: current status, reproduction notes, delivery plan, and two narrow ignore rules; no runtime, test, schema JSON, fixture, manifest, lockfile, harness, or script changes |
| Proposed review branch | `review/northstar-authority-remediation`, not yet created |
| Proposed PR base | `main` |

After the final cleanup commit, the outgoing range will contain 62 commits if the remote remains unchanged. Resolve and verify that final tip before publication. Do not push all branches or tags, force-push, rebase, or squash this evidence lineage.

A branch push includes its reachable history. The historical reports and superseded documents in those ancestors remain part of the package. The agreed curation keeps them labeled and indexed instead of rewriting the commit IDs used by the evidence.

## What goes to GitHub

| Material | Tracked files | Purpose |
| --- | ---: | --- |
| `implementations/rust/` | 32 | Authority and marker PEP source, binaries' source, tests, example, administrator harness, Cargo manifest/lockfile, and README |
| `implementations/javascript/` | 54 | Cooperative 0.1 reference plus 0.2 schema/hash/policy/evidence oracles, tests, examples, and configuration |
| `implementations/go/`, `java/`, `python/` | 21 | Existing historical secondary, skeleton, and placeholder; retain their explicit scope labels |
| `schemas/` | 18 | Frozen 0.1 and separately versioned 0.2 schema lines |
| `tests/` | 46 | Fixtures, reproduction/package checks, historical reports, and exact-candidate evidence |
| `docs/` | 49 | Standards, architecture, operations, indexed history, current review, and this delivery package |
| Root files and `implementations/README.md` | 8 | README, license, contributor instructions, ignore rules, administrator entry point, historical forwarding shim, and language map |

No npm lockfile is missing: the JavaScript package has no external dependencies. Rust's `Cargo.lock` is included. Keep historical `064717.evidence.jsonl`: the PEP oracle's positive/negative regression suite reads that fixture directly. The newer `172840` report/JSONL establishes the administrator result for the reviewed source.

## What stays local

- `skills.md`, `skills-archive.md`, `.grok/`, and local reference-repository checkouts.
- `Claude outputs/`, including the raw reviewer export; the attributed disposition is already recorded in the curated review documents.
- `docs/roadmap/restricted-egress-profile.md`, the deferred future design.
- `target/`, `node_modules/`, runtime `var/`, SQLite files/sidecars, audit logs, seal files, environment files, keys, caches, temporary command logs, and backups.

The two deferred draft paths now have explicit root-scoped ignore rules. Their files are preserved locally. Reviewed report JSONL and fingerprint files remain tracked and are not hidden by broad ignore rules.

## Source, evidence, and acceptance

| Boundary | Exact identity |
| --- | --- |
| Independently accepted implementation baseline | `aed80e2527f05a3730b1057f2d90c55a6c3eb646` |
| Tested marker-profile source | `82f5cd6cc505cc64bdea73eebeb535c2a1b4cd11` |
| Evidence-only child | `6bb0a071ceb21df3b26558b9dc64a980d1a3f09c` |
| Scoped-disposition documentation child | `b6d597eaff9f1193b155c2246f623791cde4d613` |
| Administrator evidence | [Report 172840](../tests/reports/slice-3.9-administrator-gate-2026-09-05-172840.md) and [canonical JSONL](../tests/reports/slice-3.9-administrator-gate-2026-09-05-172840.evidence.jsonl); PASS, cleanup status 0 |
| Reviewer disposition | [Scoped accept](reviews/restricted-marker-scoped-acceptance-2026-09-05.md) for the bounded macOS separate-identity marker PEP only |

The [current review packet](reviews/CURRENT-REVIEW.md) owns the full digests, command reports, and attribution. The complete non-administrator matrix passed at source `82f5cd6`; Robert ran its administrator gate; the existing reviewer then checked the evidence child and issued the named scoped acceptance. The later documentation/ignore cleanup does not change that tested source or either evidence artifact.

Phase 4, Section 3, network/egress PEP, hostile same-UID containment, production, and universal forced mediation remain **CHANGES REQUESTED / open**. Publishing this repository does not broaden the accepted implementation baseline or the marker-profile acceptance.

## Final sweep findings and checks

- Re-read GitHub's branch tip and repository metadata: the destination remains private and active; no remote divergence.
- Inspected all 61 outgoing commit messages and 853 unique text blobs across the outgoing history and current tree (795 blobs are outgoing). Ten local credential-pattern checks found no matches: private keys, GitHub/AWS/OpenAI/Slack/Stripe/Google credentials, authenticated URLs, JWTs, and quoted secret assignments. No generated/private file paths or binary blobs were found. The 17 cleanup files were also scanned with no matches. This bounded pattern scan is not a guarantee that arbitrary secrets cannot exist.
- Checked 314 relative file/directory links across all 96 tracked Markdown files against tracked targets: no missing targets or dependencies on private drafts. Existing external URLs and every heading anchor were not exhaustively checked.
- Verified the source/evidence/disposition commit chain, retained evidence digest, unchanged runtime inputs, and preservation of historical reports. Package, four-case tamper, and operations checks pass.
- Corrected remaining current-status paragraphs that still called the completed marker gate pending. Historical review reports remain unchanged.
- Added explicit ignore rules for the two local draft paths and replaced machine-specific audit-path examples with checkout-relative examples.
- Documented the toolchain limit: reproduce with Rust/Cargo 1.97.1 and Node 25.5.0. The frozen crate declares Rust 1.70, but locked test dependencies declare 1.71; minimum-version support needs a separately tested source change. See [reproduction notes](testing.md#reproduction-environment).

The full runtime matrix and administrator gate were not repeated for this documentation-only sweep. No CI workflow currently runs this matrix on GitHub; the named local reports are the available verification. CI setup, toolchain-minimum changes, releases, and new runtime features are separate follow-ups.

## Prepared PR

**Title:** Add Rust authority, bounded marker PEP, and exact-commit review evidence

**Description:**

The repository on GitHub predates the local Rust authority and its review work. This PR brings over the preserved development history, including durable evaluate/issue/claim, authenticated execution, the bounded marker PEP, JavaScript contract/evidence oracles, and the fixes for production time after lock acquisition, consumed replay, evidence validation, and socket work budgeting.

The independently accepted implementation baseline remains `aed80e2`. The reviewer separately accepted only the bounded macOS separate-identity marker PEP at source `82f5cd6cc505cc64bdea73eebeb535c2a1b4cd11`, evidence child `6bb0a071ceb21df3b26558b9dc64a980d1a3f09c`, and administrator report `172840`; disposition is recorded at `b6d597e`. The full non-administrator matrix passed at the named source, and its administrator gate passed with cleanup 0. See [the review packet](reviews/CURRENT-REVIEW.md) for exact reports and digests. Phase 4, Section 3, network/egress, hostile same-UID, production, and universal forced mediation remain open.

The final packaging changes are documentation and ignore rules only. Private reviewer exports, future egress design, runtime state, and generated outputs are excluded. Preserve merge history: squash/rebase merging would discard the named ancestry used by the evidence.

## Publication sequence after owner approval

1. Commit only the reviewed final documentation/ignore cleanup; identify the resulting tip. Confirm runtime inputs and retained reports still match the frozen source/evidence.
2. Re-read GitHub `main`. If it has changed, assess that divergence before publishing.
3. Push the chosen review branch at the exact final tip to the existing private repository. Open the prepared draft PR against `main` using repository-root links for its GitHub description.
4. Verify the remote branch matches the local tip and the PR contains the intended history. Keep merge, release tags, and any change of visibility as separate owner decisions.

The final sweep prepares a reviewable publication package. It does not authorize or perform those publication actions.
