# Changelog

All notable changes to the Northstar / TL-PX reference project.

Format: newest first.

---

## 0.1.0 — 2026-07-22

### Documentation

- Full documentation suite under `docs/` (hub, getting started, concepts, API, CLI, configuration, security, testing, integration, glossary, changelog).

### Added

- TL-PX 0.1 Minimum Profile draft (`docs/standard/SPEC-v0.1.md`)
- JSON schemas under `schemas/tlpx-0.1/`
- Reference implementation: Prism, policy, Glass evaluate/resolve/execute
- Switchboard: whitelist, credibility 0–0.99, approval routes, hard DENY
- Air-gapped mode: mandatory `auditPath`, chain-derived authorization
- Fail-closed `executeAuthorized`
- Single terminal operator outcome (state machine)
- Accountability reports (human + machine parties)
- CLI (`bin/glass.mjs`)
- Conformance suite, unit/switchboard/air-gap tests
- Formal technical test #1 (`scripts/tech-test.mjs`) — **PASS 29/29**
- Adversarial red-team script
- Apache-2.0 LICENSE
- Canonical audit path `var/tech-test-audit.jsonl`

### Security hardening

- Reject forged in-memory AUTHORIZED for execute
- Ignore stub operator_action for authorization
- Block double-resolve and REJECT→APPROVE reopening
- Receipt ids include entropy (no same-second collisions)

### Known residual risks

- Audit file write access implies forge capability (protect with OS ACLs)
- No operator authentication (free-string operator ids)
- Processes that never call the gate are unconstrained
- Policy expression engine requires trusted policy authors
- `allowEphemeral` exists for tests only

---

## Unreleased

### Security hardening

- **Strict policy compile** (Phase 1): parse → validate → compile → evaluate; malformed packs fail load and issue no authorization. `evaluateCondition` no longer treats parse errors as a non-match.
- Compiled-pack identity is a module-private `WeakSet`. Caller-supplied markers cannot skip validation. Invalid supplied `policy_pack_id` fails compile.

### Standards

- **TL-PX 0.1 frozen** as historical evidence (spec, schemas, 47 fixtures). Known `DENY` schema mismatch left in place on the 0.1 line.
- **TL-PX 0.2 draft contract** (`docs/standard/SPEC-v0.2.md`, slice 2.1): first-class `DENY`, distinct `tlpx.evaluation_error`, authorization lifecycle including `AUTHORIZED_UNCLAIMED` / claim / unknown-outcome, and explicit 0.1 compatibility rules. Schemas, JCS fixtures, and 0.2 conformance are not in this slice.
- Crosscheck follow-up: dedicated `tlpx.authorization_claim`; claim tickets must bind adapter and still consume online; sealing is an evidence-chain property.
- **TL-PX 0.2 slice 2.2:** Northstar JCS profile, domain-separated `sha256:` hashes, and `tests/fixtures/tlpx-0.2/jcs/golden.json`. JS helpers are a fixture oracle; the gate still emits 0.1 records. 0.1 `canonicalJson` is unchanged.
- JCS follow-up: key sort is RFC 8785 UTF-16 code units; lone surrogates fail closed; fixtures include astral/BMP order and raw `digest_hex`. Independent recheck accepted `636637a`.
- Docs sweep checkpoint aligned the then-accepted slices 1.1–2.2, UTF-16 JCS profile, and the fact that the gate still emits TL-PX 0.1; later entries advance the 0.2 contract line separately.
- Document ownership: SPEC-v0.2 = protocol; BUILD-SPEC = delivery; SESSION-START = status/routes; reports = immutable; skills.md = private handoff only.
- **TL-PX 0.2 slice 2.3:** schemas under `schemas/tlpx-0.2/`, reason-code catalog, JS validators, distinct `npm run conformance:0.2`. Gate still emits 0.1.
- **TL-PX 0.2 implementation-driven 2.3d:** authenticated `(principal, request_id)` slots are immutable; every retry uses a successor id; retry links are same-principal evidence only; request refusals are `DENY`; invalid policy/capability configuration fails activation; evaluation errors are durable when storage can commit; `APPROVE`/`REJECT` require rendered action context. Implemented at `aed80e2`, builder-verified, independently exact-commit crosschecked, and accepted for the bounded local-authority scope.
- **TL-PX 0.2 slice 2.4 candidate:** local commit `a87f822` adds a strict policy-bundle provenance manifest/schema, domain-separated manifest hash with JS/Rust golden agreement, explicit active-window and supersession selection, monotone fail-closed precedence, complete authority-sequence verification, and exact requirements-maturity labels. Pre-commit builder run passed; full Section 3 verification is deferred; not accepted.
- **TL-PX 0.2 slice 3.1 candidate:** local commit `1addb5c` makes Rust parse native policy manifests, verify a reproducible JCS content hash for its exact-match policy profile, reject invalid provenance/supersession at startup, select a unique active bundle by exact scope and trusted time, record unavailable/ambiguous selection as durable evidence without guessed policy identity, and recheck policy activity at claim. Pre-commit builder run passed; full Section 3 verification is deferred; not accepted.
- **TL-PX 0.2 slice 3.2 candidate:** local commit `e835c4e` adds shared schema-bound fixtures for Submitted Intent, Authorized Action, Executed Action, and Action Binding; strict JavaScript canonical/hash wrappers; Rust typed parity tests; effective-risk schema monotonicity; and exhaustive binding/authority-only mutation negatives. Bounded exact-commit checks passed; full Section 3 verification is deferred; not accepted.
- **TL-PX 0.2 slice 3.3 candidate:** local commit `c19b1d2` adds safe kernel-derived Unix peer UID/GID authentication, exact local role mapping, authenticated requester/executor facades, policy-bound approval routes, and atomic pending cancellation with schema-valid sealed operator-action evidence. Full exact-commit builder matrix passed; full Section 3 verification and independent review are deferred; not accepted.
- **TL-PX 0.2 slice 3.4 candidate:** local commit `133cd94` adds human-only route approval/rejection, canonical Authorized Action display binding, fresh post-approval IDs/nonces, independent approval and claim deadlines, and atomic expiry/terminal races. Full exact-commit builder matrix passed; full Section 3 verification and independent review are deferred; not accepted.
- **TL-PX 0.2 slices 3.5–3.7 candidates:** local commits `ad95653`, `9028346`, and `518899a` add authenticated authority-local scoped revocation, durable terminal execution/reconciliation with sealed `tlpx.execution` evidence, and mutually authenticated activated adapter contracts. Each full exact-commit builder matrix passed; portable revocation evidence, full Section 3 verification, and independent acceptance remain deferred.
- **TL-PX 0.2 slice 3.8 candidate:** local commit `7c41450` adds a bounded same-UID cooperative direct-argv runner and `tlpx-run-demo`, fresh-start-only spawn permission, process-group cleanup, public identity-only authority mutation paths, and fail-closed unknown JavaScript hash domains. Its full exact-commit builder matrix passed. It is bypassable, has no operand sandbox or race-free executable handle, does not expand the portable reason catalog, and is not forced mediation or independently accepted.
- **TL-PX 0.2 slice 3.9 candidate:** local commit `e6f2bb0` adds a bounded restricted-marker `tlpx-run` service and a dedicated-identity acceptance harness. Ordinary checks passed, but the administrator-backed acceptance run was not performed; slice 3.9, Section 3 completion, and forced mediation remain unverified.
- **Phase 4 candidates:** local commits through package `d51e46c` add separated live key roles, bounded local audit export/reconciliation, race/time/restart assurance, a local operations profile, optional non-transitive handoff preflight, and an external-review package. The independent review of target `64d0820` returned **CHANGES REQUESTED**; the original slice reports remain point-in-time builder evidence, not acceptance.
- **Phase 4 remediation:** exact commit `ee720d4` addresses or accurately re-scopes the review findings, including Switchboard ordering, post-claim revocation, authority admission capacity, source/sink reconciliation, authenticated export acknowledgements, schema fingerprints, HMAC naming/live roles, executable provenance, handoff scope, exporter locking, legacy labels, and the 3.9 harness. The full non-privileged builder matrix passes; independent re-review and the separate 3.9 gate remain open.
- **Rust local authority baseline:** `implementations/rust` crate `tlpx` adds exact-match deterministic policy, Switchboard requester/executor scope, CSPRNG authorization/nonces/claim IDs, durable SQLite decisions/errors/authorizations, authenticated-principal idempotency and retry linkage, one authority-wide sequence, revocation, expiry, capability/resource checks, and atomic exact-action claim. Named commit `c9bdd0f` additionally emits canonical schema-valid decision/error/authorization/claim records into a hash-chained, HMAC-sealed durable outbox with ordered export acknowledgement and reconciliation. That commit is builder-verified but not independently accepted. Later unaccepted 3.x/4.x candidates add authenticated local roles, approval/execution lifecycle, adapter contracts, cooperative/restricted execution profiles, and operations hardening; they do not establish a conforming 0.2 runtime or forced mediation.
- Action Binding: PEP compares the shared projection under `executed-action`. Full `authorized_action_hash` is not compared to `executed_action_hash`. Rust canonicalize/hash is fallible; types validate first. Schema requires `risk_reasons` and `risk_source`.
- Authorization stores `action_binding_hash`; claim SQL compares the presented binding to that value. `capability`/`resource_scope` are PEP constraints, not binding fields.

### Added

- Multi-language monorepo under `implementations/` (JS active; Go/Java skeletons; Rust/Python placeholders)
- Audit **hash-chain + HMAC seal** (`prev_hash`, `audit_hash`, `seal`) with `glass verify`
- **Safe policy expression parser** (no `new Function`) — A7
- **Operator allowlist + approval_route enforcement** — A18 partial
- Go: policy evaluate + **Switchboard-first** routing + CLI + tests  
- Java skeleton: policy evaluate + Maven + tests  

### Docs

- Path sweep for monorepo (`cd implementations/javascript`, `../../var/…`)

### Also in this stream

- Go sealed audit + operator/execute + **`tlpxd` HTTP control plane**
- Java Switchboard-first evaluate

### Still planned

- Cryptographic operator identity (beyond allowlist)  
- Java sealed audit parity  
- Production HA / multi-tenant control plane
