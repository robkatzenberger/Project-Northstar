# Apex Switchboard foundation — builder verification

**Date:** 2026-09-09

**Scope:** The new Apex pre-policy whitelist foundation in this working tree. This report is builder evidence, not independent security acceptance or a TL-PX conformance claim.

**Repository HEAD:** `b97c18ed56d1113ec88856309ef57150e675b1e3` (unchanged base). Apex is new, uncommitted work; the exact tested implementation inputs are fingerprinted below. Earlier documentation/design changes were already present when this increment started.

**Environment:** Local macOS, Node `v25.5.0`, npm `11.8.0`. No dependencies installed.

## Results

| Check | Result |
| --- | --- |
| `npm test` from `apex/` | Exit 0; 28 tests passed, 0 failed, 0 skipped. Count includes 13 configuration subtests. |
| `npm run demo:switchboard` from `apex/` | Exit 0; exact whitelist grant passes, wrong target denies, identity substitution denies, missing credential denies. No requested action is executed. |
| Separate code-review probe after remediation | Strict unhandled-rejection probe exited 0 for async resolve/reject, ordinary rejected-Promise return, and rejecting thenable. All four block with `AUTHENTICATION_UNAVAILABLE` and no principal. |
| `git diff --check` | Exit 0 for tracked changes; authored Apex files checked separately for whitespace. |
| Existing Northstar paths | No changes under `implementations/`, `schemas/`, `tests/`, or `docs/reviews/`. |
| Preserved APEX-Lite reference | All 29 source hashes, sizes, and modes unchanged; three original evaluation fixtures still pass under the new parent package. |
| Documentation and source fingerprints | All five input hashes below match the final files; 218 relative link destinations resolve and authored-file whitespace checks pass. |

The suite exercises real generated-token verification, unknown/non-whitelisted agents, empty scope, exact action-target pairing, cross-pair denial, case/prefix mismatch, identity substitution, request-supplied authority fields, malformed input/configuration, credential uniqueness/redaction, immutable configuration/results, deterministic outputs, and explicit instance replacement after whitelist removal.

Review found that an unsupported asynchronous verifier could cause an unhandled rejection. The implementation now consumes Promise/thenable rejections and returns a blocking error. The final suite includes regression coverage for that failure.

## Exact implementation inputs

SHA-256 values identify bytes, not a committed or independently accepted source release.

| Path relative to `apex/` | SHA-256 |
| --- | --- |
| `package.json` | `41071b7624ab1050693d3b6971c6498715f9a0e0e369a3e2f39b1adb17f7b489` |
| `src/switchboard.js` | `b079dfd4b00424e588b2ac8cf855c34ee7fcfcc5588ad2d56036cbe10832b63e` |
| `config/agents.json` | `9ca300c6d6f41c63061969bf493c00a29aec2c19b0e6ecd4f37b7e1972e8451b` |
| `examples/switchboard.js` | `41fe5ce37fc4f459a3a06fa5e877a9c838d95b15baced7313fff40cb372653fc` |
| `test/switchboard.test.js` | `8f3d988ec3d0766d2903f0f3176b08f22ddfc00b3b9ecf1357a91d514d6ba280` |

## Boundary

Switchboard returns `PASS` only for progression to constraint screening. It does not issue one-time authorizations, evaluate the full action policy, perform human approval, execute a capability, or emit sealed TL-PX evidence. The local token adapter verifies possession of a host-supplied secret; trusted registry/host control and credential protection remain embedding responsibilities. Existing instances do not hot-reload configuration or revocation. These results do not broaden the accepted Northstar implementation or marker profile.
