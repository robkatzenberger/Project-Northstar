# Slice 3.8 cooperative shell runner — builder verification

**Date:** 2026-08-20  
**Builder:** Codex  
**Initial artifact under test:** uncommitted working tree based on `4f4f608`, with the independent findings preserved unchanged at local commit `e6ef726`  
**Named artifact:** local commit `7c41450ff4e4bd22155b91149a2c0ef6366cf5f6`  
**Disposition:** full exact-commit builder matrix passed; not independently reviewed or accepted

## Scope exercised

- A bounded `CooperativeShellRunner` and deliberately named `tlpx-run-demo` verify the complete authority-issued Authorized Action, its full hash and Action Binding, and an exact `{argv, cwd, env, timeout_ms}` plan before claim.
- Startup activates a canonical executable and digest, canonical working-directory allowlist, environment-name allowlist, output bounds, duration bounds, and an execution lease longer than the requested timeout.
- Execution requires an authenticated executor, authenticated adapter contract, one-time atomic claim, and a newly created durable `STARTED` transition. A retry or already-closed attempt returns a non-permission outcome and cannot be mistaken for spawn authority.
- The runner uses direct argv execution, clears inherited environment, supplies null stdin, creates a new process group, bounds and drains stdout/stderr, and excludes output content from audit evidence.
- Timeout and terminal paths kill the process group so descendants cannot retain output pipes. Spawn failure after durable start records `FAILED`; failure to record a terminal result attempts the existing unknown-outcome transition.
- Public authority mutation paths require `AuthenticatedIdentity`; raw principal-string evaluation and claim functions are private internals. Authenticated authority/reconciler roles also guard expiry and lease-recovery mutation.
- The JavaScript 0.2 hash oracle rejects unknown symbolic hash domains rather than incorporating a caller-controlled domain prefix.
- The demo authors its own narrow request and may create one previously absent marker with `/usr/bin/touch` under the caller's UID. The production-facing `tlpx-run` name remains reserved for slice 3.9.

## Commands and results

From `implementations/rust`:

```text
cargo fmt --all -- --check                              PASS
cargo test --all-targets --offline                     PASS (88 tests)
cargo clippy --all-targets --offline -- -D warnings    PASS
```

The Rust count comprises 3 typed-action tests, 47 authority tests, 10 cooperative-runner tests, 13 evidence tests, 9 JCS/hash tests, and 6 policy-activation tests.

From `implementations/javascript`:

```text
npm test                         PASS (935 checks across the combined suite)
npm run conformance             PASS (47/47 TL-PX 0.1)
npm run tech-test               PASS (29/29)
npm run redteam                 PASS (16 PASS, 0 FAIL, 4 documented WARN)
npm run test:rust-evidence      PASS (10/10 records)
```

The combined suite includes 82/82 TL-PX 0.2 contract checks.

The real demo smoke test created the configured marker once. A changed request under the same id failed with `IDEMPOTENCY_CONFLICT`; after deleting the original marker, exact replay reached claim and failed with `ALREADY_CLAIMED` rather than recreating it.

## Fail-closed evidence

- Shell interpreters, a wrong executable digest, a non-canonical/unactivated executable, an unactivated working directory, unauthorized environment names, and an incomplete or mutated command plan block before spawn.
- Executable replacement detected by the pre-spawn digest check blocks before claim.
- A requested duration equal to the execution lease blocks before claim.
- Action metacharacters remain literal argv data rather than shell syntax.
- The child receives only explicitly allowed environment entries; inherited environment is cleared.
- Output is bounded and drained without placing captured content in the receipt summary.
- Timeout kills the child process group and records `FAILED`.
- A non-executable activated file fails after durable `STARTED` and records `FAILED`.
- Two concurrent attempts yield one success and one `ALREADY_CLAIMED`, with one marker side effect.
- An exact execution-start retry yields `NotStarted`; only a fresh `Started` value can produce spawn permission.
- Unknown JavaScript hash-domain names are rejected.

## Independent review disposition

The review preserved at `docs/reviews/slices-2.4-through-3.8-builder-review-2026-08-20.md` is a findings document, not acceptance evidence. This increment agrees with findings B1–B9, B14, and B15 as real issues or claims-discipline gaps.

Addressed before the named commit:

- B1: public raw principal-string mutation methods were removed from the external API.
- B2: same-UID socket-pair fixtures are labeled as role/contract tests, not distinct-peer authentication.
- B3/B15: the cooperative binary is named `tlpx-run-demo`; crate-local runner errors were not added to the portable 0.2 reason catalog or normative specification.
- B4: execution-start retry/closed state is a distinct non-permission result.
- B5: executable integrity is described as a check-before-spawn digest comparison, not a sealed or race-free guarantee.
- B7: environment behavior is tested with `/usr/bin/printenv`; interpreter/launcher limitations are explicit.
- B8: the runner creates and terminates a process group, including after direct-child exit before reader joins.
- B9: authenticated-path, replay, spawn-failure, lease-bound, and concurrency cases were added.
- B14: unknown JavaScript symbolic domains fail closed.

Intentionally deferred or outside this bounded increment:

- B6 operand confinement and alternate-route resistance belong to the capability-specific, separate-identity 3.9 PEP.
- B10/B11 concern earlier outbox/source-state and signing-key architecture, not this runner.
- B12 exact-commit reruns for the earlier 2.4/3.1 history remain part of the full Section 3 matrix.
- B13 and X1–X7 remain specification, policy-engine, key-management, portability, or production-hardening work unless the 3.9 acceptance bar directly exercises them.

## Explicit limits and non-claims

- This is cooperative same-UID execution, not forced mediation. A caller that can reach the capability directly can bypass it.
- Identity objects and process-owned UID/GID mappings are trusted local configuration. Same-UID unit fixtures use separate maps to exercise lookup behavior and do not prove OS identity separation.
- Although public mutation paths require authenticated identities, there is no hardened service/socket ownership boundary in this slice.
- The demo authors its own request and uses a fixed insecure local seal key.
- Exact argv binding is not an operand sandbox. A privileged runner requires capability-specific target confinement.
- Digest comparison has a check-to-exec replacement window; no file-descriptor-based execution or equivalent race closure is implemented.
- Interpreter rejection is defense in depth. An otherwise activated executable or launcher may itself interpret arguments or start other programs.
- The demo uses the platform-specific `/usr/bin/touch` path and does not claim portable command support.
- Active post-claim cancellation, portable revocation/expiry evidence, signing-key enforcement, production clock/service supervision, OS sandboxing, and alternate-route protection remain open.
- The independent review did not rerun or accept this artifact. Passing this builder matrix does not establish TL-PX 0.2 runtime conformance, independent acceptance, or production readiness.

## Exact-commit rerun

After local commit `7c41450ff4e4bd22155b91149a2c0ef6366cf5f6`, the complete Rust and JavaScript command matrix above was rerun without source working-tree changes. Results matched the pre-commit run: 88 Rust tests, clean formatting and clippy, 935 combined JavaScript checks, 47/47 TL-PX 0.1 conformance, 82/82 TL-PX 0.2 contract checks, 29/29 technical checks, 16 PASS / 0 FAIL / 4 documented WARN in the historical JavaScript red team, and 10/10 Rust evidence rows.

## Next gate

Build slice 3.9 as the separate-OS-identity, protected-target, alternate-route-resistant acceptance environment in Build Specification §14.1. After 3.9, run the full exact-commit Section 3 matrix over the named history and obtain independent review. Do not infer forced mediation or acceptance from this cooperative runner.
