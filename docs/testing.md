# Testing

## Quick commands

```bash
cd ~/projects/northstar/implementations/javascript

npm test                      # unit + switchboard + air-gap + policy/0.2 oracles + PEP-evidence negatives
npm run test:jcs              # TL-PX 0.2 JCS / hash fixtures only
npm run test:policy:0.2       # slice 2.4 provenance/precedence/ordering oracle
npm run test:actions:0.2      # slice 3.2 typed-action/hash oracle
npm run test:pep-evidence     # exact 3.9 JSONL oracle: one positive + adversarial negatives
npm run test:unit
npm run test:switchboard
npm run test:airgap
npm run conformance           # TL-PX 0.1 Minimum Profile
npm run test:rust-evidence    # validate Rust-emitted 0.2 records with the JS oracle
node scripts/tech-test.mjs    # formal technical test #1
node scripts/adversarial-redteam.mjs
npm run demo
npm run demo:switchboard
```

Rust local authority MVP (separate toolchain):

```bash
cd ~/projects/northstar/implementations/rust
cargo fmt --all -- --check
cargo test --all-targets --offline --features deterministic-time
cargo test --all-targets --offline      # production-time and restricted-PEP paths
cargo clippy --all-targets --offline --features deterministic-time -- -D warnings
cargo clippy --all-targets --offline -- -D warnings
cargo run --example local_authority -- /tmp/northstar-authority.sqlite request-1
```

The example performs evaluation, issuance, one durable SQLite claim, and—when emitting evidence—a kernel-authenticated pending cancellation. It deliberately performs no external side effect. The separate `tlpx-run-demo` binary exercises the bounded same-UID cooperative runner; neither is a forced-mediation PEP test. The production-mode suite includes lock-wait regressions for claim, human-approval, and execution-start expiry after SQLite write-transaction acquisition. It also checks that identical completion and reconciliation retries return the durable receipt despite a later server-clock sample, while changed reconciliation results remain rejected.

The [2026-09-05 working-tree review report](../tests/reports/review-followup-builder-verification-2026-09-05.md) records the reproduced reconciliation-retry and UTF-8 BOM validation defects, their fixes, the added approval-expiry regression, and the complete non-administrator verification matrix. It is builder evidence, not exact-commit or independent acceptance.

The macOS separate-identity gate is a distinct administrator test:

```bash
cd ~/projects/northstar
./run-northstar-3.9.sh --check
sudo ./run-northstar-3.9.sh
```

Run it only against the exact candidate intended for review. It creates and removes dedicated temporary identities and writes a dated report plus canonical JSONL under `tests/reports/`.

For a direct oracle check, `validate-pep-evidence.mjs` requires exactly two arguments after the script name: the JSONL path and the independently computed staged-binary digest in canonical `sha256:` plus 64 lowercase hexadecimal form. The administrator harness supplies both; `npm run test:pep-evidence` runs the committed positive fixture and adversarial mutations.

---

## Suite map

| Script | Purpose | Audit |
| --- | --- | --- |
| `scripts/test.mjs` | Core Prism/gate/accountability | temp files |
| `scripts/test-switchboard.mjs` | Whitelist, credibility, DENY paths | temp files |
| `scripts/test-airgap.mjs` | Chain auth, state machine, anti-forgery, executor | temp files |
| `scripts/test-policy-ops.mjs` | Safe expressions + operator allowlist | temp files |
| `scripts/test-policy-compile.mjs` | Phase 1 negative compile / fail-closed load | temp files |
| `scripts/test-policy-v02.mjs` | Slice 2.4 policy manifest, precedence, supersession, and trusted-ordering negatives | `tests/fixtures/tlpx-0.2/policy/` |
| `scripts/test-action-types-v02.mjs` | Slice 3.2 schema-bound action types, nullable/optional semantics, exact binding, distinct hashes, and mutation negatives | `tests/fixtures/tlpx-0.2/actions/` |
| `scripts/test-jcs.mjs` | TL-PX 0.2 JCS / domain-hash golden fixtures (UTF-16 key sort, lone-surrogate reject, `digest_hex`) | `tests/fixtures/tlpx-0.2/jcs/` |
| `scripts/conformance.mjs` | Frozen TL-PX 0.1 spec conformance (47 fixtures) | temp files |
| `scripts/conformance-v02.mjs` | Distinct TL-PX 0.2 schema/validator suite | none |
| `scripts/validate-rust-evidence.mjs` | Rust-emitted canonical decision/error/authorization/claim records against the JS 0.2 oracle | temporary in-memory SQLite |
| `scripts/validate-pep-evidence.mjs` | Validates one exact ordered 3.9 DENY/ALLOW/authorization/claim/completion chain and binds it to a caller-supplied tested-binary SHA-256 | administrator-gate JSONL |
| `scripts/test-pep-evidence.mjs` | Positive fixture plus cardinality, ordering, linkage, binary, encoding, and framing negatives for the 3.9 evidence validator | committed `064717` JSONL + temp mutations |
| `scripts/tech-test.mjs` | **Formal E2E technical test #1** | monorepo `var/tech-test-audit.jsonl` |
| `scripts/adversarial-redteam.mjs` | Red team / residual risk | temp files |
| `scripts/no-bs.mjs` | Earlier theory scoreboard | temp files |
| `scripts/demo.mjs` | Narrative demo | `var/demo-audit.jsonl` |
| `scripts/demo-switchboard.mjs` | Switchboard demo | `var/demo-switchboard-audit.jsonl` |

---

## Technical test #1 (passed)

**Command:** `node scripts/tech-test.mjs`  
**Log:** monorepo `var/tech-test-audit.jsonl` (from package: `../../var/…`)

| ID | Scenario | Expectation |
| --- | --- | --- |
| T1 | Safe allow | ALLOW → sideEffect runs → EXECUTED |
| T2 | Unknown principal | Switchboard DENY → sideEffect never runs → BLOCKED |
| T3 | Not whitelisted | DENY → no sideEffect |
| T4 | PII email | REQUIRE_APPROVAL → pending blocks → APPROVE → EXECUTED |
| T5 | Funds / low cred | REQUIRE_APPROVAL → REJECT → BLOCKED |
| T6 | Audit | Multi-record chain present |

**Status:** PASS (29/29) as of last formal run.

---

## Conformance

`npm test` also runs `test-policy-v02.mjs`, `test-action-types-v02.mjs`, `test-jcs.mjs`, and `conformance-v02.mjs`. Those are 0.2 contract oracles. They do not make the gate 0.2-conforming. `npm run conformance` remains the frozen 47.

The historical slice 3.8 report at commit `7c41450` counted 88 Rust tests. Later unaccepted slices add authority lifecycle, evidence/outbox, handoff, restricted-PEP, and production-time coverage, so do not reuse that historical count as the current suite size. The JavaScript combined suite now also runs the dedicated 3.9 evidence-oracle negatives. These non-privileged tests do not establish portable revocation evidence, alternate-route resistance, separate-identity enforcement, or independent acceptance.

Named-commit results for `aed80e2` and its independent acceptance are recorded in [`../tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md`](../tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md) and [`../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md`](../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md). Source `f025332` passed the bounded macOS marker gate recorded in [`../tests/reports/slice-3.9-administrator-gate-2026-09-04-064717.md`](../tests/reports/slice-3.9-administrator-gate-2026-09-04-064717.md) and preserved by direct-child evidence commit `32c049e`. A later review reproduced a production lock-wait deadline defect; its runtime remediation at `77d77b8` and stronger evidence oracle require a clean exact-candidate full rerun, new administrator report, and independent re-review. Full Section 3 remains unaccepted, and no general forced-mediation or network-egress claim follows from the marker gate.

Claims **TL-PX 0.1 Minimum Profile CONFORMING** when:

```bash
npm run conformance
# → Conformance: N passed, 0 failed
```

Categories: decision determinism, authorization mapping, execution guard, party model, chain integrity, accountability, request shape.

---

## Air-gap hardening checks

`npm run test:airgap` covers:

- Forged in-memory AUTHORIZED rejected  
- Stub operator ignored  
- Double-resolve blocked  
- REJECT terminal  
- Forged execution line does not create AUTHORIZED chain state  
- `executeAuthorized` fail-closed  
- Receipt id uniqueness  

---

## Red team

```bash
node scripts/adversarial-redteam.mjs
```

Interprets **PASS / FAIL / WARN**.  
WARN items remain deployment or trust-boundary reminders. In particular, A16 survives for every capability outside the one bounded 3.9 marker profile; even that profile requires fresh exact-candidate evidence for the current remediation.

Do not treat red team PASS as “no residual risk.”

---

## Writing new tests

1. Always pass `auditPath` (use `os.tmpdir()` files).  
2. Prefer `executeAuthorized` when testing side-effect gating.  
3. Assert on `policy_id` and Switchboard `flags` for routing.  
4. Clean up temp logs in the test.  
5. Update this doc and [changelog.md](./changelog.md) for behavioral tests.  

---

## CI notes

No CI pipeline is configured in-repo by default (local project). Suggested job:

```bash
cd implementations/javascript
node --version
npm test
npm run conformance
npm run test:pep-evidence
node scripts/tech-test.mjs
```
