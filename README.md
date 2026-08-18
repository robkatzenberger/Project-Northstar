# Northstar — Trust Layer (multi-language monorepo)

**Codename:** Northstar  
**What this is:** Trust Layer pre-execution checkpoint — **TL-PX** minimum standard, docs, schemas, and language implementations.

> Intent before action. Switchboard first. Gate second. Execute only if authorized.

**License:** Apache-2.0  
**Running reference:** JavaScript TL-PX 0.1

**Emerging authoritative core:** accepted Rust local-authority MVP at `aed80e2`; builder-verified sealed-evidence commit `c9bdd0f`; local 2.4/3.1 commits `a87f822`/`1addb5c`; bounded exact-commit verified 3.2 commit `e835c4e`; and an uncommitted builder-verified 3.3 Unix peer-authentication/pending-cancellation candidate (later increments unaccepted; full Section 3 verification deferred; not a PEP)

**Who should read what**

| Audience | Start here |
| --- | --- |
| Human newcomer | this file → [`docs/SHARE.md`](docs/SHARE.md) |
| Implementer | this file → [`docs/README.md`](docs/README.md) → the relevant guide |
| AI collaborator | [`NORTHSTAR-SESSION-START.md`](NORTHSTAR-SESSION-START.md) |
| Private continuity | local `skills.md` (gitignored) |

---

## Repository layout

```text
northstar/
  docs/                      Language-agnostic documentation
  docs/roadmap/              Product ideas & phased plan (H-M/M-M, PEP, tokens)
  schemas/tlpx-0.1/          Frozen TL-PX 0.1 JSON schemas
  schemas/tlpx-0.2/          TL-PX 0.2 draft schemas and reason codes
  tests/fixtures/tlpx-0.2/   0.2 JCS, policy, and typed-action/hash golden fixtures
  LICENSE
  implementations/
    javascript/              Full Node reference
    go/                      Switchboard + sealed audit + HTTP control plane
    java/                    Policy + Switchboard evaluate
    rust/                    tlpx local authority MVP (no PEP)
    python/                  Placeholder
  var/                       Local audit logs (gitignored)
```

| Path | Role |
| --- | --- |
| [`docs/README.md`](docs/README.md) | Full docs hub |
| [`NORTHSTAR-SESSION-START.md`](NORTHSTAR-SESSION-START.md) | Fresh-context instructions and required reading order |
| [`docs/BUILD-SPEC-SHEET.md`](docs/BUILD-SPEC-SHEET.md) | Current hardened build baseline targeting TL-PX 0.2 |
| [`docs/SHARE.md`](docs/SHARE.md) | Shareable plain-language overview |
| [`docs/roadmap/`](docs/roadmap/README.md) | **Ideas & next phases** |
| [`docs/standard/SPEC-v0.1.md`](docs/standard/SPEC-v0.1.md) | Frozen TL-PX 0.1 spec |
| [`docs/standard/SPEC-v0.2.md`](docs/standard/SPEC-v0.2.md) | Draft 0.2 contract; accepted 2.3/2.3d scope plus unaccepted 2.4–3.3 candidates |
| [`tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md`](tests/reports/phase-2.3d-rust-authority-mvp-2026-08-14.md) | Named-commit Rust authority evidence and limits |
| [`tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md`](tests/reports/rust-schema-evidence-outbox-builder-verification-2026-08-14.md) | Builder verification for named commit `c9bdd0f` (not independently accepted) |
| [`tests/reports/slice-2.4-policy-ordering-builder-verification-2026-08-17.md`](tests/reports/slice-2.4-policy-ordering-builder-verification-2026-08-17.md) | Pre-commit builder verification for local slice 2.4 commit `a87f822`; not exact-commit acceptance |
| [`tests/reports/slice-3.1-rust-policy-activation-builder-verification-2026-08-17.md`](tests/reports/slice-3.1-rust-policy-activation-builder-verification-2026-08-17.md) | Pre-commit builder verification for local slice 3.1 commit `1addb5c`; not exact-commit acceptance |
| [`tests/reports/slice-3.2-typed-action-hash-builder-verification-2026-08-17.md`](tests/reports/slice-3.2-typed-action-hash-builder-verification-2026-08-17.md) | Bounded exact-commit verification for local slice 3.2 commit `e835c4e`; not full Section 3 or acceptance |
| [`tests/reports/slice-3.3-local-auth-cancellation-builder-verification-2026-08-18.md`](tests/reports/slice-3.3-local-auth-cancellation-builder-verification-2026-08-18.md) | Full builder matrix and limits for the uncommitted 3.3 Unix peer-authentication and pending-cancellation candidate |
| [`implementations/README.md`](implementations/README.md) | Multi-language guide |
| [`implementations/javascript/`](implementations/javascript/) | **Run JS code / tests here** |

---

## Quick start (JavaScript reference)

```bash
cd implementations/javascript

npm test
npm run conformance
npm run tech-test

node bin/glass.mjs evaluate examples/intent-safe.json config/policy.yaml \
  --log ../../var/tech-test-audit.jsonl
```

Canonical audit log (monorepo root):

```text
var/tech-test-audit.jsonl
```

---

## Adding another language

1. Use the placeholder under `implementations/<lang>/` (or create one).  
2. Implement 0.1 against [`docs/standard/SPEC-v0.1.md`](docs/standard/SPEC-v0.1.md) and `schemas/tlpx-0.1/`. Match 0.2 hashes against `tests/fixtures/tlpx-0.2/jcs/`.  
3. Keep 0.1 records interoperable with the JS reference. Do not silently emit 0.2 shapes as 0.1.  
4. Update [`implementations/README.md`](implementations/README.md).

You do **not** need every language for the open standard — one solid reference + schemas is enough. Extra languages are ports for specific environments (JVM, Go services, Rust PEPs, etc.).

## Hardened-core direction

TL-PX 0.1 and the current JavaScript implementation remain the verified historical reference. The current hardened build baseline targets a separately versioned TL-PX 0.2 contract rather than silently rewriting v0.1.

The accepted direction is a small Rust authoritative core/PEP, a TypeScript-readable reference and conformance oracle, and other languages as SDKs/adapters. The foundational rule is:

> One authorization permits one authenticated executor to perform one exact action, one time.

See [`docs/BUILD-SPEC-SHEET.md`](docs/BUILD-SPEC-SHEET.md) for planned requirements. The 2.3 evaluation/authorization schema core is accepted; execution-receipt, post-claim cancellation/reconciliation, and revocation-evidence closure remains deferred. Rust commit `aed80e2` independently passes the bounded local evaluate → authorize → SQLite atomic-claim acceptance gate. Named commit `c9bdd0f` adds builder-verified canonical records and a sealed durable outbox. Local commits `a87f822`, `1addb5c`, and `e835c4e` add the 2.4 contract, 3.1 activation, and 3.2 typed hashes. The 3.3 working tree adds kernel-derived Unix peer roles and atomic pending cancellation with sealed operator evidence; its builder matrix passed. Full exact-commit Section 3 verification is deferred. None of these later increments is independently accepted, and none mediates a protected capability. The running gate is still TL-PX 0.1.

---

## Status

- [x] TL-PX 0.1 + JS reference + air-gap hardening  
- [x] Technical test #1 PASS (JS)  
- [x] Multi-language folder layout  
- [x] JavaScript 0.1 audit hash-chain + HMAC seal (`glass verify`)
- [x] Go: Switchboard + sealed audit + CLI + HTTP `tlpxd`  
- [x] Java: Switchboard + evaluate  
- [x] Rust 0.2 types, canonical hashes, local authority MVP, and builder-verified bounded evidence outbox
- [x] Slice 2.4 policy manifest, precedence, and ordering local commit `a87f822` (full Section 3 verification deferred; not accepted)
- [x] Slice 3.1 Rust policy-manifest activation local commit `1addb5c` (full Section 3 verification deferred; not accepted)
- [x] Slice 3.2 typed-action/hash local commit `e835c4e` (bounded exact-commit checks passed; not independently accepted)
- [ ] Slice 3.3 Unix peer authentication and pending cancellation (builder-verified working tree; commit/exact-commit check pending)
- [ ] Independent review of `c9bdd0f`, then named/full Section 3 verification and independent review of 2.4/3.x
- [ ] Rust authenticated transport, execution evidence, and protected-execution PEP
- [ ] Python adapter
- [ ] Java sealed audit / full ops parity  
- [ ] Production HA multi-tenant control plane

---

## Related

- [Trust Layer AI site](https://trust-layer-ai.github.io/Trust-Layer-AI/)  
- [Prism](https://github.com/Trust-Layer-AI/prism-protocol)  
- [Trust-Engine](https://github.com/Trust-Layer-AI/Trust-Engine)  
