# APEX-Lite source reference

This folder preserves the actual APEX-Lite code that Apex's design starts from. The snapshot is historical reference behavior, not a newly implemented Apex authority.

| Field | Value |
| --- | --- |
| Upstream | Trust-Layer-AI/Trust-Engine |
| Source commit | `b0dadd6d0f9c08f7c74e8524d324afd4623610aa` |
| Package | `apex-lite` version `0.1.0` |
| Snapshot date | 2026-09-09 |
| Contents | All 29 tracked source files, 105,133 bytes; no nested `.git`, dependencies, or runtime logs |
| License | Original [Apache-2.0 LICENSE](apex-lite/LICENSE) and attribution retained |
| Provenance | [Manifest](apex-lite.manifest.json): source commit/tree, Git blob IDs, modes, sizes, and SHA-256 for every file |

The snapshot was copied directly from the named Git objects in the local reference checkout. It is self-contained and does not depend on that ignored checkout remaining available. Keep these files unchanged; implement Apex outside this directory.

## Code map

| File | Original responsibility |
| --- | --- |
| [src/index.js](apex-lite/src/index.js) | Normalize intent, evaluate gates/policy, combine results, construct and optionally log the receipt |
| [src/engine.js](apex-lite/src/engine.js), [src/gates.js](apex-lite/src/gates.js) | Ordered rule evaluation and keyword screening |
| [public/index.html](apex-lite/public/index.html), [public/app.js](apex-lite/public/app.js) | Declaration form, decision display, human queue, and receipt feed |
| [src/operator-action.js](apex-lite/src/operator-action.js) | Approve/escalate entries linked to a recorded escalation |
| [src/receipt.js](apex-lite/src/receipt.js), [src/audit.js](apex-lite/src/audit.js) | Nonblocking receipts and plain JSONL audit |
| [bin/apex-lite.js](apex-lite/bin/apex-lite.js) | File-based evaluator entry point |
| [src/server.js](apex-lite/src/server.js) | Historical console HTTP endpoints |
| [scripts/test-apex-lite.mjs](apex-lite/scripts/test-apex-lite.mjs) | Original tests, including expectations specific to the old behavior |

## Behaviors to distinguish in Apex

- Actor and operator names are caller data; the reference has no authenticated whitelist or grant enforcement.
- `deny: true` becomes `REQUIRE_APPROVAL`; unmatched rules default to `ALLOW`. Receipts carry `blocking: false`.
- Human actions are approve/escalate, without authenticated routes, reject/expiry semantics, or exact displayed-action binding.
- Keyword matches are demonstration inputs, not trusted risk classification. The old log retains the normalized declaration without transactional or cryptographic evidence protection.
- The notification module includes an optional external SMS integration. The preserved configuration disables it; external messages are outside the design exercise.

Preserving these files does not adopt their permission model. See the [foundation design](../design/0001-foundation.md) for Apex's proposed changes and the [verification record](verification.md) for checks performed on this snapshot.
