# GitHub delivery plan

**Prepared:** 2026-09-05

**Purpose:** Package the existing Northstar implementation and review evidence for GitHub. This is a delivery plan, not a new feature roadmap or acceptance decision.

**Owner decision:** Preserve evidence history; curate current code and docs.

**Execution status:** C1 is `9b68bdb`; C2 is `77d77b8745b342d8f61b326a468c53523fe945a1`; C3 is frozen source `82f5cd6cc505cc64bdea73eebeb535c2a1b4cd11`. C4 is evidence child `6bb0a071ceb21df3b26558b9dc64a980d1a3f09c`, preserving the full non-administrator matrix and administrator report/JSONL `172840`, with PASS and cleanup 0. The reviewer subsequently accepted only the bounded macOS separate-identity marker profile. A separate documentation child records that disposition. Phase 4, Section 3, network/egress, hostile same-UID, production, and universal forced mediation remain open. Nothing has been pushed.

**Latest reviewer disposition:** [scoped acceptance, 2026-09-05](reviews/restricted-marker-scoped-acceptance-2026-09-05.md), against C3 with C4 and report `172840`. [CURRENT-REVIEW.md](reviews/CURRENT-REVIEW.md) remains the exact source/evidence entry point. Historical reports and dispositions stay unchanged. Publication requires owner authorization.

## Recommended delivery

Preserve the 56 local commits that contain the implementation and its evidence. The four focused remediation/documentation/evidence commits are complete; a separate documentation child records the reviewer's subsequent scoped acceptance. Publish one review branch to the existing repository after owner authorization; review and merge that branch separately.

Keep the complete buildable source, schemas, tests, fixtures, dependency lockfile, and evidence needed to reproduce the claims. Make the README short and put detailed evidence behind a single review entry point. Leave the unfinished restricted-egress design, expanded incident essay, private notes, generated files, and local reference repositories out of the new delivery changes.

Preserving history means GitHub receives the existing unpublished ancestors, including their historical documents. A push cannot selectively omit files from those ancestors while preserving their commit IDs. Removing a file from the latest tree would not remove its historical copies. The agreed scope is therefore to audit that history and curate the current tree, without squashing or rewriting the evidence lineage.

## Verified starting point

| Item | Observed state |
| --- | --- |
| Destination | Existing `robkatzenberger/Project-Northstar` repository; keep its visibility unchanged |
| GitHub `main` | `7a0b371e1307739e465f8c5bd313ef9372adc9be`, confirmed with `git ls-remote` on 2026-09-05 |
| Local `main` | `32c049e03d2c14729a8dc71205aef80e221e30ac` |
| Unpublished history | 56 commits; 154 files differ between the recorded GitHub baseline and local HEAD |
| Uncommitted work at inventory | 33 modified tracked files and four nonignored untracked files, before adding this plan |
| Tracked checkout | 218 files, approximately 1.87 MB of current file contents; this is not a measurement of Git history size |
| Rust on GitHub | Only its old README is present at the baseline; the Rust implementation already has local commits |
| Existing languages on GitHub | JavaScript, Go, Java, and the Python placeholder already exist; keep their historical scope labels |
| Current verification | [Working-tree builder report](../tests/reports/review-followup-builder-verification-2026-09-05.md); not exact-commit acceptance |
| Independent disposition | **CHANGES REQUESTED**; accepted implementation boundary remains `aed80e2` |

The old code does not need to be recommitted from scratch. The new remediation does. Before publishing, read the remote tip again and handle any intervening changes explicitly.

## 1. Freeze the reviewer input and inventory the delivery

1. Give the reviewer the current working-tree report and its [source fingerprints](../tests/reports/review-followup-builder-verification-2026-09-05.sha256). Treat this as preliminary review of a dirty candidate, not final exact-commit acceptance.
2. Preserve a local copy of the current patch and nonignored untracked files before curating. Keep that backup outside the repository. Do not use a destructive reset or a blanket stash/clean operation.
3. Inspect both the proposed final tree and the 56 outgoing commits for accidentally tracked credentials, personal runtime data, binary output, and unrelated material. Existing ignore rules do not remove already tracked history. This inventory has not established that every historical blob is free of sensitive content.
4. Resolve any concrete history problem before upload. Do not silently rewrite accepted commit IDs or publish a problematic blob merely because its latest copy was removed.
5. Incorporate the reviewer’s necessary findings in the appropriate source/test group below. Refresh the plan’s scope if the reviewer changes it.

**Done when:** every outgoing change has a purpose, a commit group, and a verification requirement; local-only material is identified and preserved.

## 2. Keep a buildable, reproducible repository

| Material | Delivery treatment |
| --- | --- |
| `implementations/rust/` | Include source, both binaries, example, tests, `Cargo.toml`, `Cargo.lock`, README, and restricted-agent harness |
| `implementations/javascript/` | Include the frozen cooperative reference and the 0.2 schema/hash/policy/evidence oracles with their tests and sample configuration |
| `schemas/tlpx-0.1/`, `schemas/tlpx-0.2/` | Include both versioned lines; preserve frozen 0.1 history |
| `tests/fixtures/` | Include canonical fixtures and their generation instructions; they are required source inputs |
| `tests/reports/`, `docs/reviews/` | Retain evidence referenced by exact commit and package checks. Organize access through indexes; preserve historical filenames and contents |
| `064717.evidence.jsonl` | Include the retained file at its existing path: `test-pep-evidence.mjs` reads it directly |
| `tests/check-*.sh`, `tests/redteam-*.sh` | Include reproduction and package checks |
| `run-northstar-3.9.sh` | Include the documented macOS administrator runner; document its platform, Node path, and privilege prerequisites |
| `.codex-run-northstar-3.9.sh` | Retain the existing five-line forwarding shim because a historical report invokes it; feature only the main runner in current instructions |
| `LICENSE`, `.gitignore`, contributor instructions | Include; retain licensing and the security/acceptance rules |
| Go/Java/Python historical material | Preserve existing tracked material and correct scope labels; do not expand these implementations in this delivery |

Keep local: `skills.md`, `skills-archive.md`, `.grok/`, `reference-repos/`, `var/`, runtime databases and sidecars, private configuration, environment files and keys, `target/`, `node_modules/`, caches, raw temporary logs, and backup archives. Never stage the project folder wholesale. Review whether `.gitignore` needs narrow additions for runtime SQLite databases and sidecars; do not ignore all JSONL or hash files because reviewed fixtures and evidence use those formats.

The future `docs/roadmap/restricted-egress-profile.md` stays local for this delivery. Preserve the expanded incident-comparison draft locally too. In the outgoing copy of the already tracked incident note, retain only the needed historical/non-claim clarification rather than shipping the whole new essay.

## 3. Make navigation clean

Use the existing layout; avoid directory moves that break evidence links or test paths.

| Entry point | Owns |
| --- | --- |
| Root `README.md` | What Northstar is, current scope, quick start, repository map, and links to docs/testing/current review; target about one screen before quick-start details |
| `docs/README.md` | Compact index grouped into current implementation, standards, testing/operations, and historical/future material |
| `docs/BUILD-SPEC-SHEET.md` | Delivery/acceptance baseline; keep its existing authority and maturity semantics |
| `docs/standard/SPEC-v0.2.md` | Normative contract; do not duplicate it in the delivery plan |
| `docs/testing.md` | Complete reproducible commands and prerequisites |
| `docs/reviews/CURRENT-REVIEW.md` — planned | One reviewer entry point containing the exact source commit, evidence commit when known, base range, finding disposition, tests, and open gates |
| `tests/README.md` | Current evidence first, historical evidence grouped below |
| `NORTHSTAR-SESSION-START.md` | Contributor working rules and current status; keep private preferences in the ignored memory file |

Move the long report-by-report list out of the root README’s main path into the existing evidence index. De-emphasize EOD summaries and superseded roadmaps in current navigation. Preserve the documents themselves and the links needed by historical checks. Do not create a second specification or move `authority.rs` merely to make the directory look cleaner.

There are six documents whose current drafts link to the deferred egress profile: `docs/README.md`, `docs/TRUST-LAYER-CONTINUATION-REFERENCE.md`, `docs/integration.md`, `docs/roadmap/README.md`, `docs/roadmap/phase-a-pep.md`, and `docs/roadmap/openai-hf-incident-notes.md`. In the delivery copy, remove those draft-only links and retain a brief statement that network egress remains deferred. Check the selected tree, not just the current workspace: an unstaged file can otherwise conceal a broken link.

**Done when:** newcomers see the current product and quick start; reviewers have one entry point; all included links and test inputs resolve without private files.

## 4. Commit the delivery in focused packets

Keep the 56 existing commits intact. Use explicit paths and selected hunks for the new packets. Confirm each staged diff and run its relevant checks. If a packet depends on another, preserve the order below; do not make an intermediate commit knowingly unbuildable.

| Packet | Proposed commit title | Exact scope and check |
| --- | --- | --- |
| C1 | `fix(rust): sample transition time after transaction acquisition` | `authority.rs`, `shell_runner.rs`, and `trusted_time_production.rs`; includes production completion/reconciliation retry corrections and claim/approval/lease regressions. Run both Rust time modes and formatting/Clippy |
| C2 | `test(pep): harden administrator evidence validation` | JS validator, new validator tests, `package.json`, Rust `restricted_pep.rs` test, and `restricted-agent-acceptance.sh`; keep these coupled because the validator now requires the tested-binary digest. Run JS/oracles, restricted PEP integration, and harness syntax checks |
| C3 | `docs: prepare focused GitHub review package` | Current scope corrections, curated navigation, this plan, the new current-review index, the existing working-tree report/hash file, and any narrow ignore-rule additions. Keep future drafts out. Check links, source fingerprints, historical package checks, and outgoing content |
| C4 | `test: record exact-candidate verification and review evidence` | New dated exact-candidate builder report, administrator report/JSONL, and current-review index updates. Add the independent reviewer’s actual disposition when available, with attribution and exact scope |

The subsequent disposition-only documentation child records the reviewer's scoped acceptance without altering C3, C4, or their reports.

The full hash of C3 is the initial **source candidate**. C4 adds evidence for that candidate. The old working-tree report remains an honest historical record; do not rewrite it as if it had tested C3. If the reviewer changes code, tests, schemas, configuration, dependencies, or harnesses, make a new source candidate and rerun affected gates before claiming that the prior results cover it.

Historical fingerprint files describe their original snapshots. If later reviewer changes alter those bytes, preserve the old fingerprint file and produce a new manifest with the new evidence; do not edit old hashes to make historical verification appear current.

All runtime changes from this session belong in C1/C2; sending only the latest tiny retry/BOM changes would omit their required dependent deadline remediation.

## 5. Verify the selected candidate in a clean checkout

Use an isolated checkout at the committed source candidate with its actual history available. Keep the owner’s deferred local drafts outside it. A ZIP/source archive alone is insufficient for checks that inspect historical commit ancestry.

Install the documented Rust and Node toolchains and fetch the Cargo dependencies using `Cargo.lock` before invoking the offline matrix. The recorded working-tree run used Rust/Cargo 1.97.1 and Node 25.5.0; that is evidence for those versions, not proof that every version advertised in manifests works. Resolve/document the macOS runner’s Node-location prerequisite. Do not claim a tested minimum Rust version or general cross-platform support from this one host.

Run the full matrix listed in the [working-tree report](../tests/reports/review-followup-builder-verification-2026-09-05.md): both Rust modes, both strict Clippy modes, formatting, JS tests, frozen conformance, technical/red-team/evidence checks, repeated remediation/handoff scenarios, operations checks, and historical package checks. Add link/input closure and an outgoing-file inventory against the selected commit.

Then build `tlpx-run` with the default production features, run the prerequisite check, and run the separately authorized macOS administrator gate at the exact candidate. Record the source hash, feature profile, binary digest, canonical evidence digest, command exits, cleanup outcome, and actual limitations. Generated binaries, keys, databases, and full scratch directories stay out of GitHub. Retain only the reviewed report and canonical test evidence.

Verify C4 changes only the intended evidence/index files and leaves all tested source, fixture, dependency, and harness bytes unchanged. Have the reviewer name the exact source candidate and inspect the final delivery diff. A preliminary working-tree review does not automatically approve the later candidate.

**Done when:** the selected checkout reproduces the documented checks; the new administrator report covers the candidate; reviewer findings are disposed with evidence. Until then the package remains a review candidate with **CHANGES REQUESTED**, even if uploaded to a draft branch.

## 6. Publish deliberately

1. Present the final outgoing commit range, file summary, draft PR text, evidence references, and unresolved findings to Robert. Committing and publishing remain separate actions; this planning request does not execute either.
2. After authorization to publish, push only the chosen review branch to the existing remote. Suggested branch name: `review/northstar-authority-remediation`. Do not push all branches/tags, change visibility, or force-push.
3. Open one draft PR against GitHub `main`. Its description should lead with the Rust authority and review remediation, identify the 56-commit backlog, link the short review entry point, and distinguish accepted baseline from unaccepted additions. Use the C1–C4 grouping and existing slice history to guide review rather than manufacturing independent PRs that cannot build separately.
4. If the reviewer needs GitHub access earlier, an explicitly authorized draft-branch upload can precede the final gates, with their pending status visible. That upload is not acceptance or permission to merge.
5. Verify the remote branch matches the intended local tip and that the required history objects and evidence files are available. Merge only after the reviewer/owner’s disposition and explicit merge authorization. Preserve the commit history used by evidence; do not squash or rebase it during merge.

No release tag, package publication, public launch, network PEP, new language implementation, CI redesign, or security-module refactor is part of this delivery. A small automatic CI workflow can be a separate follow-up once this exact matrix is reproduced in its chosen environment; no CI workflow currently exists in the repository.

## Delivery checklist

- [ ] Reviewer’s current findings captured and necessary changes applied.
- [ ] Outgoing history and selected files inspected; private/generated material excluded.
- [ ] C1/C2 contain the complete dependent remediation and regressions.
- [ ] C3 has concise navigation and no links to excluded local drafts.
- [ ] Source candidate committed and verified in an isolated checkout.
- [ ] Exact-candidate administrator evidence and cleanup recorded in C4.
- [ ] Independent disposition names the source commit; acceptance language matches its actual scope.
- [ ] Robert receives the final reviewable publication package.
- [ ] Authorized branch push verified; merge remains a separate decision.

## Working-tree file selection

The following inventory covers the 37 pre-plan working-tree paths. `Selected` means only current-scope corrections and navigation; preserve the rest in the owner’s local draft. It is a plan, not a staging operation.

| Packet | Path | Selection |
| --- | --- | --- |
| C3 | `AGENTS.md` | Current scope corrections; curate navigation as specified |
| C3 | `NORTHSTAR-SESSION-START.md` | Current scope corrections; curate navigation as specified |
| C3 | `README.md` | Current scope corrections; curate navigation as specified |
| C3 | `docs/BUILD-SPEC-SHEET.md` | Current scope corrections; curate navigation as specified |
| C3 | `docs/MULTI-AGENT-HANDOFF.md` | Current scope corrections; curate navigation as specified |
| C3 | `docs/README.md` | Selected; omit egress links / expanded incident essay |
| C3 | `docs/SHARE.md` | Current scope corrections; curate navigation as specified |
| C3 | `docs/TRUST-LAYER-CONTINUATION-REFERENCE.md` | Selected; omit egress links / expanded incident essay |
| C3 | `docs/architecture.md` | Current scope corrections; curate navigation as specified |
| C3 | `docs/changelog.md` | Current scope corrections; curate navigation as specified |
| C3 | `docs/http-api.md` | Current scope corrections; curate navigation as specified |
| C3 | `docs/integration.md` | Selected; omit egress links / expanded incident essay |
| C3 | `docs/operations/README.md` | Current scope corrections; curate navigation as specified |
| C3 | `docs/roadmap/README.md` | Selected; omit egress links / expanded incident essay |
| C3 | `docs/roadmap/deferred.md` | Current scope corrections; curate navigation as specified |
| C3 | `docs/roadmap/openai-hf-incident-notes.md` | Selected; omit egress links / expanded incident essay |
| C3 | `docs/roadmap/phase-a-pep.md` | Selected; omit egress links / expanded incident essay |
| Local | `docs/roadmap/restricted-egress-profile.md` | Defer entire future design |
| C3 | `docs/security.md` | Current scope corrections; curate navigation as specified |
| C3 | `docs/standard/README.md` | Current scope corrections; curate navigation as specified |
| C3 | `docs/standard/SPEC-v0.2.md` | Current scope corrections; curate navigation as specified |
| C3 | `docs/testing.md` | Current scope corrections; curate navigation as specified |
| C3 | `docs/vision.md` | Current scope corrections; curate navigation as specified |
| C3 | `implementations/README.md` | Current scope corrections; curate navigation as specified |
| C3 | `implementations/javascript/AGENTS.md` | Current scope corrections; curate navigation as specified |
| C2 | `implementations/javascript/package.json` | Complete coupled change |
| C2 | `implementations/javascript/scripts/test-pep-evidence.mjs` | Complete coupled change |
| C2 | `implementations/javascript/scripts/validate-pep-evidence.mjs` | Complete coupled change |
| C3 | `implementations/rust/README.md` | Current scope corrections; curate navigation as specified |
| C2 | `implementations/rust/scripts/restricted-agent-acceptance.sh` | Complete coupled change |
| C1 | `implementations/rust/src/authority.rs` | Complete dependent fix + tests |
| C1 | `implementations/rust/src/shell_runner.rs` | Complete dependent fix + tests |
| C2 | `implementations/rust/tests/restricted_pep.rs` | Complete coupled change |
| C1 | `implementations/rust/tests/trusted_time_production.rs` | Complete dependent fix + tests |
| C3 | `tests/README.md` | Current scope corrections; curate navigation as specified |
| C3 | `tests/reports/review-followup-builder-verification-2026-09-05.md` | Preserve working-tree evidence as historical |
| C3 | `tests/reports/review-followup-builder-verification-2026-09-05.sha256` | Preserve working-tree evidence as historical |

Planned additional C3 files: this delivery plan, `docs/reviews/CURRENT-REVIEW.md`, and `.gitignore` only if its narrowly scoped runtime exclusions need updating. C4 filenames are generated from the actual future run; do not prefill results or commit IDs.
