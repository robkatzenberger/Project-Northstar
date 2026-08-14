# Build Plan Review Disposition — 2026-08-13

**Artifact reviewed:** `docs/BUILD-SPEC-SHEET.md`  
**Prior review:** `docs/reviews/build-spec-review-2026-08-11-model-2.md`  
**Review type:** Independent architecture review, then human-mediated refinement  
**Status:** Accepted. Implementation must not start until this disposition is recorded.  
**Code changes in this session:** none

This record is architecture evidence, not executed test proof. Runtime proof remains under `tests/reports/`.

## Verdict

The build sheet remains the destination architecture. Its completeness is an asset because it closes ambiguity before code fossilizes it. Completeness is not permission to implement all 23 slices at once.

The hazard is confusing specification progress with enforcement progress. The two-agent proof still stands: a process that retains direct authority can bypass the gate.

**Live implementation scope:** slices 1.1–1.2 only.

## Accepted findings

| Finding | Disposition |
| --- | --- |
| Do not treat the sheet as one implementation assignment | Accepted |
| `docs/roadmap/priorities.md` still presents a superseded sequence | Accepted; label history, do not erase |
| Old `AUTHORIZED` snapshot token is incompatible with the invariant | Accepted and sharpened: **abandoned**, not deferred |
| Implicit no-match `ALLOW` is a fail-open | Accepted, with v0.1 / v0.2 version discipline |
| Slice 3.9 can be faked by a cooperative agent | Accepted and strengthened |
| Specification completeness is not itself a flaw | Accepted; the original phrasing was too broad |

## Phase 1 exact boundary

Phase 1 corrects the JavaScript/TypeScript reference compiler. It does not expand that reference into the production authority. It does not introduce TL-PX 0.2 records.

```text
parse → validate → compile → evaluate
```

### In scope

- Compile the entire policy pack before any decision.
- Reject the known malformed condition `risk ==` and every other invalid rule, operator, field, id, effect, or structure listed in the build sheet.
- A rejected pack issues no authorization.
- `evaluateCondition()` must not swallow compile/parse errors as a non-match.
- Add a negative suite **beside** the frozen 47 TL-PX 0.1 fixtures. Do not rewrite those fixtures to pretend they already tested fail-closed compilation.
- At startup, a malformed pack fails process start. The previously loaded policy must not remain active accidentally. For this slice there is no last-known-good fallback.
- Later hot reload, when designed, must fully compile the replacement and atomically activate it. Partial load is forbidden.

### Out of scope

- `DENY` and `EVALUATION_ERROR` record types
- TL-PX 0.2 schemas, hashes, or conformance
- Rust authority
- Claim tickets, tokens, or `issueAuthzToken`
- PEP / `tlpx-run`, including a cooperative `pep-run.mjs` sold as forced mediation
- Growing Go `tlpxd` authorization semantics

### Negative-test treatment

- Existing tests that passed because a malformed rule silently failed to match must become compile or startup failures. That is the intended correction, not a regression to paper over.
- The 47 v0.1 conformance fixtures remain historical evidence of the frozen minimum profile. They are not the Phase 1 negative suite.

## Explicit policy defaults

| Version | Rule |
| --- | --- |
| TL-PX 0.1 / Phase 1 | Malformed policy fails pack load or process startup. It must not emit a v0.2 `EVALUATION_ERROR`. Implicit no-match `ALLOW` remains the frozen v0.1 evaluate rule for **valid** packs. |
| TL-PX 0.2 | Every policy bundle requires an explicit default outcome. Missing or invalid default prevents bundle activation. If that state is reached at runtime, emit `EVALUATION_ERROR` and issue no authorization. |

An explicit `default: ALLOW` remains legal in the minimum standard after Switchboard has authenticated and scoped the principal. The security defect is **implicit** allowance combined with optional or bypassed scope enforcement.

Hardened profiles MAY require `default: DENY` or `default: REQUIRE_APPROVAL`. The minimum standard MUST NOT assume every deployment has that posture.

## Portable proof versus transactional consumption

For Northstar’s ordinary software threat model:

> Authorization proof may be portable; single-use consumption remains online and transactional.

A PEP may verify locally:

- signature
- authenticated executor binding
- `authorized_action_hash`
- adapter / environment / tenant scope
- expiry

It must still atomically claim the authorization from authoritative state before any side effect begins. Offline verification cannot establish global non-consumption.

A later **authorization claim ticket** or **capability proof** may exist if it is bound to one authenticated executor, one `authorized_action_hash`, one adapter/environment/tenant, one short claim window, and one atomic server-side consumption record.

Exotic hardware-backed non-copyable capabilities could change the consumption story someday. They must not complicate this design.

### Abandoned design

The August 7 `tlpx.authz_token` **AUTHORIZED snapshot** in `docs/roadmap/phase-b-authz-tokens.md` is **abandoned**, not deferred. It must not return under its old semantics:

- bearer token with minutes-long TTL
- no proof-of-possession
- single `actor` instead of requester/executor split
- `intent_hash` only
- offline verify treated as sufficient permission
- one-time use listed as optional because it is “harder offline”

Do not implement `issueAuthzToken` / `verifyAuthzToken` against that sketch.

## Slice 3.9 enforcement acceptance

“The PEP identity is the only writer” is necessary and not sufficient. A Node script that chooses to call the PEP is cooperative mediation, which the two-agent test already proved.

The acceptance environment must use **separate OS identities** for the restricted agent and the PEP. Separate UIDs are necessary. Socket access alone is not impersonation: the agent must be able to contact the PEP to request authorized work.

The test must establish all of the following:

1. The agent identity cannot write the protected target.
2. The agent cannot acquire or impersonate the PEP identity: it cannot read PEP keys, modify PEP code or configuration, or bind a replacement service.
3. The agent UID may connect to the PEP only through a narrowly permitted interface.
4. The PEP service UID owns the protected capability and secrets.
5. The PEP authenticates the caller independently rather than trusting a caller-supplied principal string.
6. The PEP accepts only authenticated, exact, unexpired, unclaimed authorization and cannot be induced to execute an unclaimed or mismatched action.
7. Reuse, mutation, direct access, alternate binaries, alternate paths, and alternate privileged interfaces all fail.
8. The resulting execution and denial evidence is independently verified.

If the suite runs as the same OS user that owns the PEP socket, keys, or target path, it has not tested impersonation resistance.

An earlier unauthenticated `tlpx-run` may exist only as an explicitly labeled prototype. It is not the 3.9 acceptance test.

## Roadmap continuity

| Document | Status after this disposition |
| --- | --- |
| `docs/BUILD-SPEC-SHEET.md` | Current hardened build baseline |
| `docs/roadmap/priorities.md` | **Superseded** as current direction; retained as history |
| `docs/roadmap/language-strategy.md` | **Superseded** as current direction; Rust is the planned authority |
| `docs/roadmap/phase-a-pep.md` | Problem statement retained; sequence and acceptance bar superseded by this sheet |
| `docs/roadmap/phase-b-authz-tokens.md` | Old snapshot token **abandoned** |
| `docs/roadmap/phase-c-mm-handoff.md` | Handoff goal retained; must not depend on the abandoned snapshot token |

The August 7 rule of thumb remains useful as history:

> Tokens without mediation are theater; handoff without tokens doesn’t travel.

It is now restated as:

> A portable claim ticket without transactional consumption is theater; handoff without a separately named executor authorization does not travel.

## Current next work

When Robert explicitly opens implementation: Phase 1 only, in `implementations/javascript/`, against the boundary above.

No Rust. No token. No PEP. No commit or push unless asked.
