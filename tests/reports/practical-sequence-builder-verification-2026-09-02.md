# Practical sequence builder verification — 2026-09-02

**Run time:** 2026-09-02 16:14 CDT
**Builder:** Codex
**Base HEAD:** `b83c28002e17e543d540e8e54e5681ae96ba046f` (`main`)
**Candidate state:** uncommitted working-tree changes; no exact candidate commit exists
**Disposition:** H1/M2 changes and the human-review composition path pass builder verification; the separate-identity slice 3.9 administrator gate was not run

This is builder evidence over a dirty working tree. It is not an independent re-review, Phase 4 acceptance, Section 3 acceptance, production-readiness evidence, portable handoff evidence, or proof of forced mediation. Unrelated documentation changes were already present when this run began and remain outside this report's candidate scope.

## Implemented candidate scope

- Normal evaluation opens the transaction and fully reconciles sealed evidence before sampling `evaluated_at_ms`. Reconciliation time therefore no longer consumes the authorization claim window.
- Full admission-time evidence reconciliation remains unchanged. No evidence watermark, partial verification scheme, retention policy, or hard row limit was added.
- Caller-supplied timestamps on authenticated `_at` APIs are rejected before a transaction when they exceed a one-second local clock-skew bound. Within that bound, production-default builds still ignore the supplied value and use the authority clock; only the explicit deterministic-time test feature can override it. The cooperative runner also takes time from the internal authority surface.
- The durable trusted-time floor tolerates at most one second of bounded clock jitter while retaining the maximum observed time; larger rollback still fails closed. This is clock state, not an evidence-verification watermark.
- A composition test now covers `REQUIRE_APPROVAL` through authenticated human approval, exact rendered-action binding, one execution side effect, raw-payload exclusion, and sealed decision/operator/authorization/claim/execution evidence.
- The focused Phase 4 remediation script explicitly enables the deterministic-time test feature for fixed-time integration suites.

## Verification results

### Rust

```text
cargo fmt --all -- --check
PASS

cargo clippy --all-targets --offline --features deterministic-time -- -D warnings
PASS

cargo test --all-targets --offline --features deterministic-time
PASS — 126 passed, 0 failed, 1 dedicated volume probe ignored

cargo test --lib --offline
PASS — 8 passed, including the production-default caller-time bound
```

The explicit optimized volume probe also passed:

```text
cargo test --release --offline --features deterministic-time \
  --test evidence_outbox \
  full_reconciliation_volume_probe_stays_inside_the_claim_window \
  -- --ignored --nocapture

PASS — 999 sealed rows
evaluation: 81.435125 ms
claim:      63.668833 ms
```

This measurement is local builder evidence, not a portable performance guarantee. The complete reconciliation walk remains O(rows); its scaling constant and a future incremental-verification or retention design remain open architecture work. The correctness fix is that the claim window begins after the walk.

The human-review composition case passed inside the full matrix and established:

1. no marker side effect while the decision was pending;
2. an authenticated human operator on the required route approved the exact `authorized_action_hash`;
3. the approved authorization executed once and produced the marker;
4. the approval view contained only the payload digest, not the raw private payload;
5. sealed evidence excluded the raw payload and contained linked `tlpx.decision`, `tlpx.operator_action`, `tlpx.authorization`, `tlpx.authorization_claim`, and terminal `tlpx.execution` records; and
6. full evidence reconciliation returned exactly five records.

### Phase 4 focused and operational checks

```text
./tests/redteam-phase-4-remediation.sh
PASS — 5 focused repetitions; 20 handoff repetitions

./tests/check-phase-4-review-package.sh
PASS

./tests/redteam-phase-4-review-package.sh
PASS — 4/4 tampering cases rejected

./tests/check-operational-profile.sh
PASS
```

### JavaScript and cross-language oracles

```text
npm test
PASS

npm run test:jcs
PASS

npm run conformance
PASS — 47/47 TL-PX 0.1 checks

npm run tech-test
PASS — 29/29

npm run redteam
PASS — 16 pass, 0 fail, 4 documented boundary warnings

npm run test:rust-evidence
PASS — 10/10 runtime evidence records
```

The four red-team warnings remain the documented reference/deployment boundaries: declared-intent honesty, bypass by a process that never calls the cooperative gate, the test-only ephemeral escape hatch, and actor-type defaulting in the historical JavaScript reference.

## Slice 3.9 administrator gate

The production-default `tlpx-run` binary built successfully. No disposable Linux/container runtime and no pre-created dedicated macOS test identities were available on this host. Two secure administrator-prompt attempts were made (an app terminal prompt and the native macOS authentication dialog), but neither received a password response, so the harness never started.

No temporary `_tlpx39_pep`, `_tlpx39_agent`, or `_tlpx39_other` account was created. The helper is deliberately scoped to create those three hidden, password-disabled accounts at UIDs 61001–61003, run the existing acceptance harness, and delete only accounts it created on every exit path.

Therefore, slice 3.9 remains **UNRUN AND UNVERIFIED**. This report does not establish separate-identity enforcement, alternate-route resistance, protected-marker provenance, or forced mediation.

To resume the gate from an interactive macOS terminal:

```bash
sudo /bin/sh /Users/home/projects/northstar/.codex-run-northstar-3.9.sh
```

After that command completes, record its exact output in a separate administrator-gate report. Do not revise this partial result into a pass.

## Required next acceptance step

Create a new exact commit containing only the intended H1/M2/test changes, rerun this matrix against that clean commit, complete and separately report the slice 3.9 administrator gate, and send the exact candidate plus both reports for independent review. The older accepted 2.3/2.3d boundary remains unchanged.
