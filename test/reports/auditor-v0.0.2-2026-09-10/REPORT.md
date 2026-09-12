# Apex Auditor v0.0.2 — approve/escalate and human review

Local builder verification passed on 2026-09-10 for the uncommitted Apex working tree. The decision gate now returns APPROVE or ESCALATE. Only the authenticated operator resolves a saved escalation with APPROVE or DENY and a required context comment.

Repository HEAD is `b97c18ed56d1113ec88856309ef57150e675b1e3`, which does not contain these Apex changes. [source.json](source.json) identifies the tested files by SHA-256. Environment: macOS arm64, Node v25.5.0, Chrome 152.0.7977.85. HTTP/browser checks used authorized localhost execution outside the shell sandbox.

| Verification | Result | Evidence |
| --- | --- | --- |
| Full `npm test` | 106 passed, 0 failed | [Raw output](tests.txt) |
| Browser workflow | Six assessments and three human outcomes reconciled with the journal; no page errors | [Browser results](browser.json), [reusable check](../../browser/auditor.mjs) |
| Human review form and outcome | Desktop and 390 px mobile inspected; no horizontal overflow | [Review form](human-review.png), [mobile outcome](human-outcome-mobile.png), [full desktop](desktop.png), [full mobile](mobile.png) |
| Existing demo upgrade | Original journal prefix unchanged; fresh current-model policy appended | [Migration evidence](demo-migration.json) |
| Pinned APEX-Lite reference | All 29 files match the original manifest | [Source verification](source.json) |

The full suite includes 14 gate tests, 7 auditor integration/storage tests, 34 human-review store tests, and the existing Switchboard, registry, and separate audit-prototype tests. The separate audit prototype remains outside this new human-review path.

The browser check verifies automatic APPROVE, explicit and unmatched ESCALATE, and nonreviewable authority BLOCKED and evaluation ERROR. It exercises required context, both human decisions, whitespace trimming, literal display of HTML-looking comments, stale approval rejection with context retained and denial still available, and read-only resolved outcomes after reload. Returned review receipts bind exact assessment IDs and hashes; original assessments remain unchanged. Temporary credentials are absent from the journal and are removed with the fixture after the run.

The backend checks cover operator/agent separation, immutable request binding, version and authority rechecks, concurrent and repeated decisions, restart/replay validation, policy migration, and fail-closed storage. No human decision overrides a failed Switchboard check. A separate read-only integration review found no remaining blocker within this local scope.

This is a local single-operator auditor, not a deployed independent service or an execution authority. It checks three-field action/target declarations; full material-action validation, reviewer-specific routes, expiry, remote identity, and tamper-proof evidence remain outside this increment. Removing automated DENY alone does not establish resistance to gaming. Earlier v0.0.1, APEX-Lite, and Northstar evidence retains its original scope.

See the [current guide](../../../docs/auditor-v0.0.2.md) for rule entry, review behavior, API, and migration. The [prior report](../auditor-v0.0.1-2026-09-10/REPORT.md) remains unchanged.
