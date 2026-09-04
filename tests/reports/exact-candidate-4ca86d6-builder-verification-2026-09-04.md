# Exact candidate `4ca86d6` builder verification — 2026-09-04

**Candidate:** `4ca86d61ab61e8350bfce9ece8771bcb0fea870e`
**Candidate state:** clean exact commit before verification
**Disposition:** PASS for the recorded local builder matrix and bounded macOS slice 3.9 administrator gate

This report combines the exact-candidate checks performed after the previously dirty working-tree work was divided into reviewable commits. It is local builder evidence, not independent re-review, Section 3 or Phase 4 acceptance, production-readiness evidence, a portable performance claim, or proof of universal forced mediation.

## Candidate contents

The exact candidate contains:

- Phase 4 status-document alignment;
- trusted-time transition hardening and post-reconciliation claim-window timing;
- an authenticated human-review-to-execution composition test;
- the pinned local Trust Layer lineage reference; and
- the one-command slice 3.9 administrator runner and macOS `chown` path correction.

## Non-privileged Rust verification

The following checks passed against the exact candidate:

```text
cargo fmt --all -- --check
PASS

cargo clippy --all-targets --offline --features deterministic-time -- -D warnings
PASS

cargo test --all-targets --offline --features deterministic-time
All application assertions passed after the restricted local-socket test was rerun with desktop socket permission:
126 passed, 0 application failures, 1 dedicated volume probe ignored

cargo test --lib --offline
PASS — 8 passed, 0 failed

cargo build --offline --bin tlpx-run
PASS
```

The initial all-targets invocation was denied permission to bind the temporary restricted-PEP Unix socket by the desktop sandbox. The isolated `restricted_pep` suite was rerun with local socket permission and passed 2/2. This was an execution-environment denial, not a Northstar assertion failure.

## Slice 3.9 administrator gate

The gate ran from a clean tree and recorded the exact candidate hash before writing its report:

```text
sudo ./run-northstar-3.9.sh
PASS
```

Authoritative output: [`slice-3.9-administrator-gate-2026-09-04-054905.md`](./slice-3.9-administrator-gate-2026-09-04-054905.md).

The bounded macOS profile established:

1. the restricted agent could not write the protected target directly or through the tested shell and symlink alternatives;
2. the agent could not modify PEP code, configuration, key material, or endpoint ownership;
3. an unrelated UID could not use the mapped agent identity;
4. caller-supplied identity, path, argv, and unsupported-action input failed closed;
5. one authenticated exact request completed under the PEP identity;
6. authorization reuse, target reopening, restart, and expiry did not repeat the effect;
7. five sealed decision/authorization/claim/execution evidence rows reconciled with zero failures; and
8. all three temporary password-disabled test identities were removed successfully.

## Remaining acceptance boundary

The independently accepted boundary remains the 2.3 evaluation/authorization schema core and bounded Rust 2.3d commit `aed80e2`. The prior Phase 4 **CHANGES REQUESTED** disposition remains authoritative until exact candidate `4ca86d6` receives independent re-review. Hostile same-UID isolation, active cancellation of already-started effects, portable revocation/expiry/handoff-link evidence, production key custody, external audit transport, rotation/HA/DR automation, and universal forced mediation remain open or explicit non-claims.
