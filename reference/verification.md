# APEX-Lite snapshot verification

**Date:** 2026-09-09

**Scope:** Copy integrity and three original evaluation fixtures for the historical APEX-Lite snapshot. This is not a new Apex implementation test or security acceptance.

| Item | Observed result |
| --- | --- |
| Source commit | `b0dadd6d0f9c08f7c74e8524d324afd4623610aa` |
| Source tree | `48c7904183a80fc916a3a8e314db484a05d66b41` |
| Source inventory | All 29 tracked regular files copied from Git objects; 105,133 bytes |
| Integrity | Every copied file matches the manifest's SHA-256, Git blob ID, size, and executable mode |
| Directory inventory | Exactly the manifest-listed files, with no extras or symlinks |
| License and attribution | Upstream LICENSE and README preserved byte-for-byte |
| Runtime for fixture checks | Node `v25.5.0`, local macOS environment |
| Existing Northstar source/evidence | No changes under `implementations/`, `schemas/`, `tests/`, or `docs/reviews/` |

The fixture check imported the snapshot's `src/index.js` and called `evaluateFiles` with each existing example and `examples/policy.yaml`. It fixed `evaluatedAt` to `2026-01-01T00:00:00.000Z`, asserted the decision and policy ID expected by the original tests, and confirmed each receipt remained `blocking: false`.

| Original fixture | Decision | Policy ID | Result |
| --- | --- | --- | --- |
| `examples/intent-high-risk.json` | REQUIRE_APPROVAL | `rule_01` | PASS |
| `examples/intent.json` | REQUIRE_APPROVAL | `email_gate` | PASS |
| `examples/intent-safe.json` | ALLOW | `null` | PASS |

The integrity and fixture checks each exited 0. No server, audit-log write, notification call, dependency installation, or protected operation was involved. The complete upstream HTTP/UI test suite was not run. These checks reproduce the reference's existing behavior; they do not establish authenticated identity, whitelist enforcement, or any proposed Apex feature.
