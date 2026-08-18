# Testing

## Quick commands

```bash
cd ~/projects/northstar/implementations/javascript

npm test                      # unit + switchboard + air-gap + policy compile + 0.2 JCS/schema
npm run test:jcs              # TL-PX 0.2 JCS / hash fixtures only
npm run test:policy:0.2       # slice 2.4 provenance/precedence/ordering oracle
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
cargo test --all-targets --offline      # JCS/types + authority + evidence/outbox tests
cargo clippy --all-targets --offline -- -D warnings
cargo run --example local_authority -- /tmp/northstar-authority.sqlite request-1
```

The example performs evaluation, issuance, and one durable SQLite claim. It deliberately performs no external side effect and is not a PEP test.

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
| `scripts/test-jcs.mjs` | TL-PX 0.2 JCS / domain-hash golden fixtures (UTF-16 key sort, lone-surrogate reject, `digest_hex`) | `tests/fixtures/tlpx-0.2/jcs/` |
| `scripts/conformance.mjs` | Frozen TL-PX 0.1 spec conformance (47 fixtures) | temp files |
| `scripts/conformance-v02.mjs` | Distinct TL-PX 0.2 schema/validator suite | none |
| `scripts/validate-rust-evidence.mjs` | Rust-emitted canonical decision/error/authorization/claim records against the JS 0.2 oracle | temporary in-memory SQLite |
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

`npm test` also runs `test-policy-v02.mjs`, `test-jcs.mjs`, and `conformance-v02.mjs`. Those are 0.2 contract oracles. They do not make the gate 0.2-conforming. `npm run conformance` remains the frozen 47.

The Rust suite independently consumes the same JCS golden fixtures and policy-manifest golden hash (9 tests), adds 23 authority tests for activation failure, durable `DENY`/`EVALUATION_ERROR`, immutable idempotency, replay, restart, and one-winner claim races, 12 evidence/outbox tests for atomic coupling, explicit party type, sealing configuration, ordering, restart persistence, payload/envelope tamper detection, compatibility rejection, and reconciliation, plus 6 slice 3.1 policy-activation tests. Those new cases cover native golden parsing, content binding, broken/cyclic supersession, unavailable/ambiguous durable errors, explicit winner selection, and claim-time activity recheck. The JS crosscheck validates four Rust-emitted canonical record types. These tests do not establish execution/revocation evidence, authenticated transport, or forced mediation.

Named-commit results for `aed80e2` and its independent acceptance are recorded in [`../tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md`](../tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md) and [`../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md`](../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md). Named commit `c9bdd0f` has builder evidence in [`../tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md`](../tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md) and still requires an independent exact-commit review. Local slice 2.4 commit `a87f822` and dependent 3.1 commit `1addb5c` have pre-commit builder records in [`../tests/reports/slice-2.4-policy-ordering-builder-verification-2026-08-17.md`](../tests/reports/slice-2.4-policy-ordering-builder-verification-2026-08-17.md) and [`../tests/reports/slice-3.1-rust-policy-activation-builder-verification-2026-08-17.md`](../tests/reports/slice-3.1-rust-policy-activation-builder-verification-2026-08-17.md). Their exact-commit/full Section 3 verification is deferred until Section 3 is complete; both remain `PLANNED` and unaccepted.

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
WARN items (audit file write, bypass outside library, operator spoof) may remain by design until deployment controls exist.

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
node scripts/tech-test.mjs
```
