# Slice 3.9 restricted PEP candidate — acceptance not yet verified

**Date:** 2026-08-27  
**Builder:** Codex  
**Named candidate:** local commit `e6f2bb0a511628dc94f619716c3682b506d5aaf4`  
**Disposition:** implementation candidate committed; ordinary builder checks passed; the defining separate-OS-identity acceptance test was not run; slice 3.9 and Section 3 remain incomplete and unaccepted

> **TEST REQUIRED:** Run `sudo ./scripts/restricted-agent-acceptance.sh` from `implementations/rust` before claiming that slice 3.9 is implemented, verified, or accepted. The command asks for the macOS administrator password because the acceptance bar requires real separate OS identities. This run is intentionally deferred.

## Candidate scope

- A bounded Unix-socket `tlpx-run` service exposes one operation, `CREATE_MARKER`.
- The service authenticates the connected caller from kernel peer UID/GID and maps only the configured restricted-agent identity.
- The caller cannot supply a principal, target path, executable, arguments, environment, or authorization object.
- The service authors and evaluates the complete intent internally, claims one exact authorization, requires a fresh durable execution start, invokes the bounded runner, and records terminal evidence.
- Configuration, seal-key, state, endpoint-parent, protected-target, and executable permission checks fail closed.
- Rust reconciles canonical outbox payloads, hashes, HMAC seals, source linkage, and record coverage. The JavaScript oracle validates the exported denial, claim, and execution records against the 0.2 schemas and linkage rules.
- The acceptance script is designed to run the PEP, restricted agent, and unrelated caller under distinct OS identities in a random `/tmp/northstar-3.9.*` tree and clean that tree on exit.

## Checks completed before the candidate commit

From `implementations/rust`:

```text
cargo fmt --all -- --check                              PASS
cargo test --all-targets --offline                     PASS (91 tests)
cargo clippy --all-targets --offline -- -D warnings    PASS
```

The 91 Rust tests include two same-identity restricted-PEP integration tests. Those tests exercise the protocol, denial, mutation, replay, expiry, permission validation, evidence reconciliation, and JavaScript evidence validator, but they do not prove separate-identity OS enforcement.

From `implementations/javascript`:

```text
npm test                         PASS (935 checks across the combined suite)
npm run conformance             PASS (47/47 TL-PX 0.1)
npm run tech-test               PASS (29/29)
npm run redteam                 PASS (16 PASS, 0 FAIL, 4 documented WARN)
npm run test:rust-evidence      PASS (10/10 records)
```

The combined suite includes 82/82 TL-PX 0.2 contract checks. Shell syntax validation for `restricted-agent-acceptance.sh` also passed.

These were pre-commit checks over the source tree that became `e6f2bb0`; they are not an exact-commit rerun and do not substitute for the test below.

## Required acceptance run not performed

From the repository on macOS:

```bash
cd /Users/home/projects/northstar/implementations/rust
sudo ./scripts/restricted-agent-acceptance.sh
```

The script must prove every Build Specification §14.1 assertion with real identity separation:

1. Direct marker writes and alternate-path writes fail for the restricted agent.
2. The restricted agent cannot read the PEP configuration or seal key, alter the PEP executable, replace its endpoint, or acquire the PEP identity.
3. An unrelated OS identity cannot use the interface.
4. Caller-supplied principal, path, and argument material is rejected.
5. The exact allowed request creates the protected marker under the PEP identity.
6. Replay, mutation, restart, and expired authorization cannot create another marker.
7. Denial, claim, and terminal execution evidence passes both Rust integrity reconciliation and JavaScript schema/linkage validation.

Until this command completes successfully, the candidate does **not** establish forced mediation, alternate-route resistance, slice 3.9 completion, Section 3 completion, or TL-PX 0.2 runtime acceptance.

## After the required run

If the separate-identity suite passes, record an exact-commit rerun and then run the full Section 3 matrix over the named 2.4/3.x history. Independent review is still required before acceptance.
