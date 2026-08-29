# Phase 4 independent-review remediation — builder verification

**Date:** 2026-08-28  
**Builder:** Codex  
**Original reviewed security-code target:** `64d08203c2f30964b176be8872b390ba3c28e5e6`  
**Original independent disposition:** **CHANGES REQUESTED**  
**Exact remediation commit:** `ee720d44d43904612a148b8f968ea22f59b43f73`  
**Builder disposition:** exact-commit checks pass; independent re-review and the separate slice 3.9 administrator gate remain open

## Scope delivered

The remediation responds to the finding map in [`../../docs/reviews/phase-4-independent-review-disposition-2026-08-28.md`](../../docs/reviews/phase-4-independent-review-disposition-2026-08-28.md). In the exact commit it:

- makes Switchboard refusal precede tenant-policy selection and binds refusal evidence to a canonical Switchboard digest;
- rechecks authorization, principal, policy, tenant, environment, capability, and authorization-MAC-key revocation after claim and before execution start;
- enforces exact canonical-export capacity inside the authority state transaction and streams reconciliation;
- reconciles sealed evidence against source-row content, authenticates export acknowledgements with the audit HMAC key, verifies the at-rest sink, and coordinates cooperating exporters with a persistent advisory lock;
- re-fingerprints critical schema constraints, indexes, and the idempotency partial-index predicate at startup;
- names symmetric authorization proofs as MAC/HMAC and requires only the two live local roles: authorization MAC and audit sealing;
- strengthens protected-executable provenance and path-to-inode checks within the distinct-identity profile, and binds the protected marker into resource scope;
- makes handoff preflight, legacy JavaScript/Go behavior, review-package limits, and the unverified 3.9 boundary explicit; and
- adds an executable focused-repeat harness for the reviewer reproducers.

## Exact-commit verification

The following checks ran from a clean checkout state at `ee720d44d43904612a148b8f968ea22f59b43f73`.

From `implementations/rust`:

```text
cargo fmt --all -- --check                           PASS
cargo test --all-targets --offline                  PASS (125 tests)
cargo clippy --all-targets --offline -- -D warnings PASS
sh -n scripts/restricted-agent-acceptance.sh        PASS (syntax only)
```

The 125 Rust tests include the ordinary restricted-service socket integration tests. The desktop sandbox was relaxed only to bind a temporary local Unix socket; no administrator command or slice 3.9 acceptance script ran.

From the repository root:

```text
./tests/redteam-phase-4-remediation.sh       PASS (5 focused rounds; 20 handoff rounds)
./tests/check-phase-4-review-package.sh      PASS
./tests/redteam-phase-4-review-package.sh    PASS (4/4 tampering cases rejected)
./tests/check-operational-profile.sh         PASS
git diff --check                             PASS
```

From `implementations/javascript`:

```text
npm test                    PASS (935 checks across the combined suite)
npm run conformance        PASS (47/47 TL-PX 0.1)
npm run tech-test          PASS (29/29)
npm run redteam            PASS (16 PASS, 0 FAIL, 4 documented WARN)
npm run test:rust-evidence PASS (10/10 records)
```

The Go toolchain was unavailable locally (`gofmt` was not installed). The remediation changes only Go source and README labeling comments; it does not change Go behavior.

## Reviewer-finding disposition

The exact implementation evidence supports the remediation statuses recorded in the independent-review disposition document. It does not convert the original independent **CHANGES REQUESTED** result into acceptance. In particular:

- H1 remains an open, separately privileged slice 3.9 gate.
- M11 remains an explicit defect on the frozen historical 0.1 line; history was not silently rewritten.
- L1 reviewability debt in the large authority module remains deferred.
- L7 same-eUID adapter fixtures remain contract tests, not OS-identity isolation evidence.
- Active cancellation after a successful start remains deferred.

## Explicit limits and non-claims

- The password/sudo-backed `scripts/restricted-agent-acceptance.sh` was deliberately not run at the project owner's direction. Slice 3.9, Section 3 completion, forced mediation, alternate-route resistance, and protected-marker provenance are not established.
- The file exporter and lock are a bounded local cooperative profile, not hostile same-UID isolation, external audit transport, safe sink rotation, replication, retention automation, or production monitoring.
- Executable checks strengthen the distinct-identity profile; they do not contain a hostile process with the same UID or prove who created the protected marker inode.
- Handoff is optional co-presentation preflight, not a portable credential or universal handoff mediation layer.
- Validating the policy precedence manifest does not make the Rust authority a generic six-stage runtime interpreter.
- Portable revocation/expiry/handoff-link evidence, hardware-backed key custody, HA/DR automation, formal/model checking, and production hardening remain open.
- Builder verification is not independent acceptance, certification, or production readiness.

## Next gate

Send exact remediation commit `ee720d44d43904612a148b8f968ea22f59b43f73` and this report to an independent reviewer. Record a new independent disposition without revising the historical **CHANGES REQUESTED** report. Run the dedicated-identity slice 3.9 acceptance script only as a separate, explicitly authorized administrator gate.
