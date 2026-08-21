# Independent review for the builder — slices 2.4 through 3.8

**Date:** 2026-08-20  
**Reviewer:** Grok 4.6 (architecture / security / conformance, collapsed into one pass)  
**Audience:** the implementing builder  
**Owner:** Robert decides. This is not a work order until he says which items to address.

**Reviewed artifacts**

| Artifact | Commit / state |
| --- | --- |
| Accepted baseline | `aed80e2527f05a3730b1057f2d90c55a6c3eb646` (2.3d). Out of scope except as the last accepted line. |
| Outbox | `c9bdd0fcd2d4c51fda9f3861724db0fd97524003` |
| 2.4 | `a87f82271a12843c120d9a1e6ee238957f285c2f` |
| 3.1 | `1addb5c6a0ede31d754ac0bd47d7ef1f3a05e6d4` |
| 3.2 | `e835c4e` |
| 3.3 | `c19b1d2` |
| 3.4 | `133cd94` |
| 3.5 | `ad95653` |
| 3.6 | `9028346` |
| 3.7 | `518899a` / HEAD docs `4f4f608` |
| 3.8 | **uncommitted working tree** (`shell_pep.rs`, `tlpx-run.rs`, tests, SPEC §13.1, `PEP_*` reason codes) |

**Method:** static inspection of code, schemas, SPEC, build sheet, and builder reports; plus `cargo test --test shell_pep --offline` on the working tree (**7 passed**). This reviewer did **not** re-run the 3.3–3.7 exact-commit matrices and did **not** `git archive` each named commit.

**Disposition:** findings only. **Not independently accepted. Not Section 3 complete. Not a 0.2 runtime. Not forced mediation.**

---

## How to use this

1. Do not edit, commit, or push until Robert names the items in scope.
2. Do not treat this document as acceptance of 2.4–3.8.
3. Do not “close” a finding by inventing protocol, portable records, M-of-N, ZK, dual ledgers, or a second policy evaluator.
4. For any trust-path change (identity APIs, execution start semantics, evidence emission, SPEC/catalog), propose the plan and wait. Do not start with a large `authority.rs` refactor.
5. Preserve disagreements. If you think a finding is wrong, say so with code/test evidence. Do not manufacture consensus.
6. Reply in the format at the bottom.

---

## Verdict for the builder

The 2.4–3.7 library work is real and mostly honest: fail-closed SQLite CAS, distinct intent/authorized/claim/receipt objects, sealed outbox atomicity, and builder reports that usually list non-claims. Keep that discipline.

The defects that matter are not a second `ALLOW`. They are:

1. **Identity is still embedding-chosen.** Kernel UID/GID plus a process-owned map, with public string APIs still live. Tests and `tlpx-run` mint requester/executor/adapter/authority from the same eUID via separate authenticators.
2. **3.8 is a cooperative runner using production names and protocol surface.** `ShellPep` / `tlpx-run` plus SPEC §13.1 `MUST` plus seven `PEP_*` codes in the 0.2 catalog will be quoted as a PEP. It is not.
3. **Portable evidence and “complete stream” contradict each other.** Revocation, approval expiry, and unknown/reconciliation do not emit sealed portable records, but some of them consume authority sequence. Do not certify exports as complete.
4. **Review debt.** `c9bdd0f` was supposed to get independent exact-commit review before more stack. It did not. Do not self-accept 2.4–3.8 from builder matrices.

Nothing here makes A16 go away. Skip the crate, do the side effect.

---

## Standing orders — do not undo the good

Do not regress these. They are the reason the stack is worth keeping.

- Snapshot `AUTHORIZED` token stays abandoned.
- `DENY` vs `EVALUATION_ERROR` stay distinct. Frozen 0.1 DENY/schema defect stays on the 0.2 line; do not rewrite 0.1 fixtures.
- No LLM on allow/deny/require-approval.
- Invalid / ambiguous / unavailable inputs fail closed.
- One authorization, one named executor, one exact action, one time. Claim remains atomic CAS.
- Do not invent `tlpx.revocation` or an `EXPIRE` operator-action outcome to make evidence look complete. The current explicit gap is better than a private record type.
- Two-connection races for cancel, approve/reject, claim/revoke, and terminal finish stay tested.
- Outbox insert failure must still roll back the matching authority state.
- 3.8 spawn path must remain: validate complete Authorized Action → claim → require `STARTED` → `Command::new(path).args(argv)` with `env_clear()`, null stdin, no shell interpolation, output content withheld from the receipt summary.
- JS remains the 0.1 reference and 0.2 oracle. Rust remains the authority. Do not make JS a 0.2 decision engine.

---

## Findings the builder must not “fix” by expanding architecture

| ID | Finding | Forbidden response | Allowed response (only if Robert asks) |
| --- | --- | --- | --- |
| X1 | No portable revocation/expiry/unknown records | Add `tlpx.revocation`, private EXPIRE records, or new outbox types | Leave the gap explicit, **or** stop allocating `authority_sequence` for events that cannot be exported. Propose first. |
| X2 | 2.4 six-stage precedence is hashed, not executed | Build an EMERGENCY_DENY / TENANT / HUMAN_APPROVAL evaluator in Rust | Keep exact-match + Switchboard. If SPEC wording overclaims, tighten SPEC, do not grow the engine. |
| X3 | 2.4 complete-stream oracle ≠ outbox ordinals | Feed outbox rows into `verifyCompleteAuthoritySequence` and “make it pass” | Leave the oracle as a sequence-number tool. Do not claim complete export. |
| X4 | 3.8 cannot confine a privileged PEP UID | Declare 3.8 forced mediation; add sandbox/seccomp/landlock as the 3.9 substitute | Keep 3.8 labeled cooperative. Argv/cwd confinement and fd-exec belong in a **3.9 plan**, not a silent 3.8 widen. |
| X5 | `authority.rs` is 4,276 lines | Drive-by split of the TCB while “addressing review” | No split unless Robert asks for a dedicated refactor slice. |
| X6 | Independent review was deferred | Mark 2.4–3.7 `IMPLEMENTED`/`accepted` because this file exists | This file is findings. Acceptance is a separate Robert decision at named commits. |
| X7 | Signing keys / HMAC key id | Implement slice 4.1/4.2 inside 3.8 | Leave 4.1/4.2 alone. |

---

## Actionable findings

Priority: **P1** = do not land more trust-path on top until this is decided. **P2** = fix in the current candidate if Robert includes it. **P3** = hygiene.

### P1 — identity and APIs

**B1. Public string principal APIs are still the real interface.**

- `Authority::evaluate_and_issue` / `evaluate_and_issue_at`
- `Authority::claim` / `claim_at`
- `Authority::revoke_at` (actor recorded as `trusted-embedding.local`)
- Also unauthenticated control plane: `expire_pending_at`, `recover_expired_claim_at`

Facades (`evaluate_authenticated*`, `claim_authenticated*`, `revoke_authenticated*`) require roles, then call the string methods. Examples (`implementations/rust/examples/local_authority.rs`) and most tests, including 3.8 `issue()` in `tests/shell_pep.rs`, still use the string path.

If an untrusted adapter can call `claim("anyone", …)`, slice 3.3 did not happen.

**Required if in scope:** propose a fail-closed gate (`cfg(test)`, embedding-only module, or `pub(crate)` plus an explicit trusted-embedding trait). Untrusted paths must not compile against the string APIs. Do not silently delete the embedding seam without a plan: 2.3d tests depend on it.

**B2. Same-UID identity factory in 3.8 and 3.7 tests.**

`tlpx-run.rs` `identity()` / `adapter_session()` and `tests/shell_pep.rs` build a **fresh** `LocalAuthenticator` per role, mapping `Uid::effective()` to whichever principal the test wants, then `UnixStream::pair()` in-process.

`LocalAuthenticator` forbids two principals per UID **inside one map**. Separate maps make one OS identity into requester, executor, adapter, and authority.

**Required if in scope:** stop describing this as mutually authenticated distinct peers in README/SPEC. Tests may keep socketpair for unit coverage only if labeled “same-UID mapping lookup, not peer separation.” Distinct OS users are 3.9, not a 3.8 unit-test patch.

**B3. Do not land 3.8 as protocol.**

Uncommitted working tree already added:

- `docs/standard/SPEC-v0.2.md` §13.1 with `MUST` language for a prototype command plan
- `PEP_CONFIG_INVALID`, `PEP_REQUEST_INVALID`, `PEP_COMMAND_DENIED`, `PEP_EXECUTABLE_INTEGRITY_INVALID`, `PEP_EXECUTION_NOT_STARTED`, `PEP_IO_FAILED`, `PEP_TIME_INVALID` to `schemas/tlpx-0.2/reason-codes.json`, SPEC §9.3, and `conformance-v02.mjs` string-membership checks
- Production names `ShellPep` and `tlpx-run` (the 3.9 acceptance binary name)
- Tests using `environment: "production"`, `tenant: "tenant_abc"`

JS conformance “includes PEP configuration failure” proves catalog membership, not enforcement.

**Required if in scope, before any 3.8 commit:**

- Keep crate-local error strings if needed; **do not** grow the portable 0.2 reason catalog for a cooperative demo unless Robert explicitly wants those codes frozen now.
- Move SPEC §13.1 out of claims-discipline `MUST`, or label it explicitly `PLANNED` / prototype non-normative. Preferred: delete §13.1 from SPEC until a named 3.8 commit exists and Robert accepts a command-plan contract.
- Do not use `tlpx-run` as the demo binary name if 3.9 will own that name. Call the cooperative demo something else (`tlpx-run-demo` / `examples/`).
- Tests must not pretend a production tenant.

### P1 — execution start API

**B4. `begin_execution_authenticated_at` returns `Ok(lease)` for non-start states.**

`implementations/rust/src/authority.rs` (~1380–1504):

- Exact retry of an existing execution returns `Ok(existing.lease())` regardless of `STARTED` / `LEASE_EXPIRED` / unknown / terminal.
- First start after lease expiry **inserts** `LEASE_EXPIRED`, enqueues a terminal receipt, and still `Ok(stored.lease())`.

3.8 currently guards spawn with `lease.state != ExecutionState::Started` → `PEP_EXECUTION_NOT_STARTED`. That is necessary and must stay. It does not make `Ok` mean “you may execute.”

A caller that checks only `begin_execution_*(...).is_ok()` will spawn after the authority closed the attempt. That violates “a crash after a possible side effect must not auto-replay.”

**Required if in scope:** `begin_execution_*` must return `Err` (or a distinct non-`Result<Lease>` type) unless this call is the first durable `STARTED`. Exact retries of `STARTED` must not be treated by any PEP as permission to spawn again. Propose the type/error codes first; do not silently change the 3.6 receipt schema.

Note: `ShellPep::execute` re-claims first, so a full `execute()` retry after a successful claim hits `ALREADY_CLAIMED` rather than double-spawn. The footgun is the `begin_execution` API and any split claim/start caller.

### P2 — 3.8 mechanism (keep cooperative; harden the demo)

**B5. Hash-then-`execve(path)` TOCTOU.**

`shell_pep.rs` `validate_request` canonicalizes and `sha256_file`s the target, then later `Command::new(&plan.executable).spawn()`. Same for cwd. SPEC §13.1 already admits replacement races. That is not “integrity.”

Do not claim executable integrity in 3.8. Fd-based exec (`fexecve` / equivalent) is a 3.9/platform plan, especially on macOS. If you add a comment, say check-then-exec, not integrity.

**B6. Policy authorizes `shell.exec` + executable target, not the plan.**

`policy.rs` `evaluate` matches `intent.action`, then capability, then `intent.target == resource_scope` entry. `argv` / `cwd` / `env` become exact only because they sit in the Action Binding after issuance.

`touch /anywhere-the-uid-can-write` is in scope. Cwd allowlist does not confine operands. For same-UID 3.8 this is theater. For a privileged 3.9 PEP UID it is a confused-deputy write gadget.

**Do not** bolt path confinement into 3.8 as if that made it 3.9. **Do** record in the 3.8 report: argv is not a sandbox; 3.9 must confine operands before the PEP UID is the only writer.

**B7. Interpreter denylist is cosmetic; `/usr/bin/env` is a positive test.**

`reject_shell_interpreter` is basename equality on `sh|bash|dash|zsh|fish|ksh|csh|tcsh`. Tests pin `/usr/bin/env`, `/bin/echo`, `/bin/sleep`. Env, python, perl, busybox, shebang scripts, xargs remain valid if configured.

“Never invokes a shell” only means “we do not call `/bin/sh -c`.” Keep that sentence accurate. Add a test that documents env-as-launcher as **out of 3.8 enforcement**, or stop using `/usr/bin/env` as a happy-path pin.

**B8. Child isolation.**

`child.kill()` is the direct child. No process group. A grandchild holding stdout/stderr can hang `drain_bounded` / `join_output` and leave `STARTED` with no terminal receipt. `wait()` after `kill()` can block.

If Robert wants 3.8 hardening: process group / `killpg`, bounded wait, and a test that spawn-failure after `STARTED` records `FAILED` or `UNKNOWN`. Do not call that a sandbox.

**B9. 3.8 test holes.**

Fix these if 3.8 stays in tree (they do not require protocol changes):

- `issue()` in `tests/shell_pep.rs` uses `evaluate_and_issue("agent.requester", …)`. Use the authenticated requester facade for the library path you claim to test.
- `tlpx_run_binary_completes_narrow_marker_smoke_path` replays while the marker exists. `normalize_new_marker` rejects `path.exists()` **before** evaluate, so failure may not be `ALREADY_CLAIMED`. Delete the marker (or use a new request id against the consumed auth) if you want to prove claim consumption at the binary.
- Adapter digest in tests is `sha256:8888…`. Label it as config equality, not measurement.
- No tests for: absolute argv outside cwd, cwd symlink replace, binary replace between hash and spawn, lease expiry between validate and spawn, spawn failure after claim, concurrent `execute`, `timeout_ms >= execution_lease_ms`.

This reviewer ran:

```text
cargo test --test shell_pep --offline
# 7 passed; 0 failed
```

Passing this suite does not prove 3.8, 3.9, or PEP catalog conformance.

### P2 — evidence and policy contract

**B10. Outbox reconcile binds source IDs, not source contents.**

`evidence.rs` coverage is existence of the referenced row. An SQLite writer can mutate operational tables while sealed JSON stays original. Live enforcement reads operational rows, not the outbox.

Do not document reconcile as verifying source-row **contents**. If Robert wants a hardening slice: compare canonical operational projection to sealed payload. That is not 3.8.

**B11. HMAC seal does not include `seal_key_id`.**

MAC is prefix + chain hash. Key id is checked against **current** config. Rotation fail-closes history. Leave 4.1 alone; stop any wording that this is key-separated audit.

**B12. 2.4/3.1 exact-commit reruns were never recorded.**

Reports remain pre-commit working-tree evidence. Maturity stays `PLANNED`. 3.2 was promoted to `IMPLEMENTED` after a **bounded** rerun. Do not change maturity labels in this pass. If Robert asks, rerun the **full** builder matrix at `a87f822` and `1addb5c` and record it. Do not infer acceptance.

**B13. 3.1 does not execute 2.4 precedence stages.**

Rust: Switchboard `refusal_for_intent`, then exact action match (`policy.rs` `evaluate`). Manifests **store** the constant precedence array. Tenant/environment is a selection key.

If SPEC §10.2 reads as if six monotone stages run in the authority, tighten the SPEC sentence. Do not implement the stages.

**B14. JS `hash.mjs` `resolveDomain` falls back to the raw string for unknown domains.** Rust rejects them. Fail closed in JS to match Rust, in the oracle only. Do not silently hash with a caller-supplied prefix.

### P3 — docs and naming hygiene

**B15.** Rust README “Enforced by…” / “Next implementation” must not imply 3.8 is forced mediation. The working-tree README already has a decent “Deliberate boundary” paragraph; keep that tone everywhere, including SPEC header (header still stops at 3.7 while §13.1 exists).

**B16.** `skills.md` is stale (2026-08-14, next gate still `c9bdd0f`). Builder does not own that gitignored file unless Robert asks.

**B17.** Approval route membership is `.any()` (1-of-N), not quorum. Ordered route is not M-of-N. Do not add M-of-N. Do not call it quorum in docs.

**B18.** Renderer id/version and adapter `binary_hash` are caller/config strings. Do not say “measured” or “displayed UI bound” beyond hash-of-stored-action.

---

## Slice-by-slice notes (builder)

Use this as a checklist of what you actually shipped vs what the reports must keep saying.

### 2.4 `a87f822` — keep as oracle

Shipped: `policy-bundle.schema.json`, `northstar:policy-bundle:v1\0` hash, JS selection/supersession/sequence/maturity oracles.

Not shipped: Rust activation (that is 3.1), six-stage evaluation, complete-stream verification of the outbox.

Report status is correct: `PLANNED`, pre-commit evidence, unaccepted.

### 3.1 `1addb5c` — real activation, unauthenticated publisher

Shipped: native manifest parse, content-hash bind, catalog selection, durable `POLICY_UNAVAILABLE` / `POLICY_PRECEDENCE_AMBIGUOUS` without guessed policy identity, claim-time recheck.

Not shipped: signed issuer, transactional catalog, production clock. `evaluate_and_issue_at` / `claim_at` still take trusted timestamps.

Do not treat current content type `application/vnd.tlpx.rust-exact-match+json;version=2` as the 3.1 digest (approval routes came later).

### 3.2 `e835c4e` — contract pinning

Types and binding hashes already existed in 2.3d. You pinned fixtures and JS schema-before-hash. Do not describe this as a new authority.

### 3.3 `c19b1d2` — facades, not OS users

Kernel creds + exact map: good for a local profile. Tests use current eUID. String APIs remain. Pending-cancel CAS is good. Portable `CANCEL` omits role/reason; do not add private protocol fields.

### 3.4 `133cd94` — approval lifecycle is real

Fresh post-approval ID/nonce and claim window starting at approve time: keep. Hash-only approve does not prove the operator viewed the JSON. Machine UID mapped `Human`+`Operator` can approve. Expiry is durable state without portable record — keep the gap explicit. Late cancel vs late approve error codes differ (`APPROVAL_TERMINAL` vs `APPROVAL_EXPIRED`); do not “unify” by inventing EXPIRE evidence.

### 3.5 `ad95653` — claim-time revocation only

CAS with claim is good. `revoke_at` bypasses emergency role. Post-claim authorization revoke cannot be recorded. `SIGNING_KEY` is unused. Sequence on local revocation rows is absent from export. Do not add `tlpx.revocation` here.

### 3.6 `9028346` — receipts are real; start API is a footgun

Terminal sealed `tlpx.execution`, unknown stays consumed, no auto-replay in the authority: keep. Result evidence is caller-supplied. No sweeper. See B4.

### 3.7 `518899a` — contract names, not measurement

Exact ordered `ADAPTER_MATERIAL_FIELDS`, startup coverage, start-time verify: keep. Digest is presented string vs config. Session is cloneable; sockets dropped. Material list is not a translator; runtime still accepts a fully formed `ExecutedAction`.

### 3.8 working tree — cooperative runner

Pipeline order is right. 7/7 tests pass. Same UID. Demo seal key is `vec![0x73; 32]` / `insecure-prototype-only`. Direct `/usr/bin/touch` still works. This is the already-proven cooperative gate, now with a spawn.

Do not commit 3.8 until Robert picks a naming/SPEC/catalog disposition from B3.

---

## 3.9 — do not start

Slice 3.9 is BUILD-SPEC §14.1. None of the eight acceptance bullets are met. Starting 3.9 before B1–B4 and B3 naming are decided will cement a privileged confused deputy.

Minimum 3.9 preconditions the builder should not skip:

- Separate OS identities; agent cannot write the marker.
- String APIs not reachable from the restricted agent.
- Argv/operand confinement **before** the PEP UID is the only writer.
- `tlpx-run` name reserved for that binary, not the same-UID demo.
- Independent review of the authority stack recorded at named commits.

---

## What this reviewer did not do

- Exact-commit rerun of `c9bdd0f`, `a87f822`, `1addb5c`, `e835c4e`, `c19b1d2`, `133cd94`, `ad95653`, `9028346`, `518899a`.
- Isolated checkout of those commits (inspection was the current tree plus reports).
- Independent JS 0.1 red-team / 0.2 full conformance beyond reading reports.
- Claim that 3.8 tests are the Section 3 matrix.

Builder reports for 3.3–3.7 look careful about non-claims. This review does not rubber-stamp those matrices.

---

## Builder reply format

When Robert asks you to respond, reply with:

```text
Agree
  - ID: one sentence
Disagree
  - ID: evidence (file:line or test name) and the residual risk you still accept
Out of scope without a new plan
  - ID
Proposed plan (only if Robert asked for implementation)
  - items in order
  - tests you will add, including negatives
  - SPEC/schema files you will not touch
  - what will remain unclaimed
```

Do not update maturity to `IMPLEMENTED` or write “independently reviewed” into SPEC/build sheet unless Robert accepts a named-commit crosscheck. This file is not that crosscheck.
