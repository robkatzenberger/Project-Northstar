# Northstar — Trust Layer

Northstar is a pre-execution authorization boundary for human- and machine-initiated actions. Its invariant is:

> One authorization permits one authenticated executor to perform one exact action, one time.

The repository contains the versioned TL-PX specifications, JSON Schemas, a Rust local authority, and the JavaScript reference and conformance oracles. License: [Apache-2.0](LICENSE).

**Current disposition: SCOPED ACCEPT for the bounded macOS separate-identity marker PEP only.** The reviewer accepted source `82f5cd6` with evidence child `6bb0a07` and administrator report `172840`. The independently accepted implementation baseline remains the bounded Rust evaluate/issue/claim MVP at `aed80e2`. See the [recorded disposition](docs/reviews/restricted-marker-scoped-acceptance-2026-09-05.md) and [current review packet](docs/reviews/CURRENT-REVIEW.md) for exact hashes, evidence, and limits.

The historical `f025332` / `064717` administrator run established a bounded macOS separate-identity marker result. It did not cover the later lock-wait deadline finding and does not establish acceptance of the remediation. Section 3, Phase 4, production readiness, hostile same-UID containment, and network-egress mediation remain open.

## Start here

| Need | Read |
| --- | --- |
| Understand the project | [Plain-language overview](docs/SHARE.md) |
| Build or integrate | [Documentation hub](docs/README.md) and [implementation roles](implementations/README.md) |
| Review this candidate | [Current review packet](docs/reviews/CURRENT-REVIEW.md) |
| Run verification | [Testing commands and prerequisites](docs/testing.md) |
| Follow build/acceptance scope | [Build specification](docs/BUILD-SPEC-SHEET.md) |
| Prepare GitHub delivery | [Delivery plan](docs/GITHUB-DELIVERY-PLAN.md) |
| Work as a contributor | [Session instructions](NORTHSTAR-SESSION-START.md) |

## Local development

The exact-candidate matrix was run with Rust/Cargo 1.97.1 and Node 25.5.0 on macOS. See [testing](docs/testing.md) for the full matrix and platform limits. Passing these commands does not extend the reviewer's acceptance beyond the named local marker profile.

```bash
cd implementations/rust
cargo build --locked
cargo test --all-targets --offline
cargo test --all-targets --offline --features deterministic-time
```

The JavaScript gate remains the frozen cooperative TL-PX 0.1 reference. Its 0.2 checks are contract/hash/evidence oracles:

```bash
cd implementations/javascript
npm test
npm run conformance
npm run tech-test
```

The separate macOS administrator gate is documented in [testing](docs/testing.md). It requires a production-feature binary and dedicated temporary OS identities; ordinary local tests do not replace it.

## Repository map

| Path | Purpose |
| --- | --- |
| `implementations/rust/` | Local authority, cooperative demo, bounded marker PEP candidate, and tests |
| `implementations/javascript/` | Frozen cooperative 0.1 reference and 0.2 oracles |
| `implementations/go/`, `implementations/java/` | Historical secondary references; not 0.2 authority/PEP endpoints |
| `implementations/python/` | Placeholder for later adapter work |
| `schemas/tlpx-0.1/`, `schemas/tlpx-0.2/` | Separate versioned schema lines |
| `docs/standard/` | Frozen 0.1 specification and draft 0.2 contract |
| `tests/fixtures/` | Shared canonical/hash/policy/action fixtures |
| `tests/reports/`, `docs/reviews/` | Dated evidence, findings, and current review index |
| `docs/operations/` | Bounded local readiness and incident procedures |

Historical reports are indexed in [test evidence](tests/README.md). Superseded and future ideas remain under [roadmap](docs/roadmap/README.md). Private notes, runtime data, generated output, and local reference checkouts stay outside the published delivery.
