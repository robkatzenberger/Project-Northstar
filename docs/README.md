# Northstar documentation

**Current disposition: SCOPED ACCEPT for the bounded macOS separate-identity marker PEP only.** The named source is `82f5cd6`, evidence child `6bb0a07`, administrator report `172840`. The independently accepted implementation baseline remains `aed80e2`; Phase 4, Section 3, network/egress, hostile same-UID, production, and universal forced mediation remain open. See the [current review packet](reviews/CURRENT-REVIEW.md).

## Current implementation and review

| Document | Purpose |
| --- | --- |
| [Project overview](../README.md) | Scope, quick start, and repository map |
| [Plain-language introduction](SHARE.md) | What Northstar is and its boundaries |
| [Current review packet](reviews/CURRENT-REVIEW.md) | Candidate identity, evidence, findings, and outstanding gates |
| [Latest reviewer disposition](reviews/restricted-marker-scoped-acceptance-2026-09-05.md) | Owner-supplied scoped acceptance of the named macOS marker profile and explicit remaining limits |
| [Build specification](BUILD-SPEC-SHEET.md) | Delivery sequence, maturity, and acceptance requirements |
| [GitHub delivery plan](GITHUB-DELIVERY-PLAN.md) | Scoped commits, history preservation, and publication steps |
| [Architecture](architecture.md) | Components and trust boundaries |
| [Implementation roles](../implementations/README.md) | Rust authority versus historical references and future adapters |
| [Contributor session instructions](../NORTHSTAR-SESSION-START.md) | Working rules and required reading |

## Standards and concepts

| Document | Purpose |
| --- | --- |
| [Standards overview](standard/README.md) | Version discipline and minimum-standard scope |
| [TL-PX 0.1 specification](standard/SPEC-v0.1.md) | Frozen historical minimum profile |
| [TL-PX 0.2 specification](standard/SPEC-v0.2.md) | Accepted evaluation/authorization schema core and later draft contracts |
| [0.1 schemas](../schemas/tlpx-0.1/) / [0.2 schemas](../schemas/tlpx-0.2/) | Versioned JSON Schema definitions |
| [Concepts](concepts.md), [glossary](glossary.md), [vision](vision.md) | Vocabulary, intent, and non-goals |

## Testing and operations

| Document | Purpose |
| --- | --- |
| [Testing](testing.md) | Full local matrix and separate administrator-gate prerequisites |
| [Evidence index](../tests/README.md) | Current builder evidence and dated historical reports |
| [JCS fixtures](../tests/fixtures/tlpx-0.2/jcs/), [policy fixtures](../tests/fixtures/tlpx-0.2/policy/), [action fixtures](../tests/fixtures/tlpx-0.2/actions/) | Cross-language conformance inputs |
| [Security](security.md) | Threat model and residual boundaries |
| [Local operations](operations/README.md) | Readiness, backup/restore, and evidence reconciliation |
| [Incident response](operations/INCIDENT-RESPONSE.md) | Local severity and recovery procedures |
| [Multi-agent handoff](MULTI-AGENT-HANDOFF.md) | Optional co-presentation preflight; no transferable permission |
| [Changelog](changelog.md) | Implementation and documentation changes |

## Historical reference interfaces

These guides describe the JavaScript/Go cooperative reference surfaces. They do not establish Rust 0.2 transport, a production service, or HTTP/network-egress mediation.

| Document | Purpose |
| --- | --- |
| [Getting started](getting-started.md) | Reference setup and demonstration |
| [JavaScript API](api-reference.md), [CLI](cli-reference.md), [configuration](configuration.md) | Reference interfaces and sample configuration |
| [Switchboard](switchboard.md), [air-gap](airgap.md), [logging](logging.md) | Cooperative reference behavior |
| [Integration](integration.md), [historical HTTP API](http-api.md) | Existing integration sketches and explicit limits |

## History and deferred work

Historical findings and reports retain their filenames and original scope; they are not current acceptance statements.

- [Accepted 2.3d independent review](../tests/reports/phase-2.3d-rust-authority-independent-crosscheck-2026-08-14.md)
- [Phase 4 independent disposition](reviews/phase-4-independent-review-disposition-2026-08-28.md) and [original review package](reviews/phase-4-external-security-review-readiness-2026-08-27.md)
- [August 20 builder-facing review](reviews/slices-2.4-through-3.8-builder-review-2026-08-20.md)
- [Build-spec review](reviews/build-spec-review-2026-08-11-model-2.md) and [build-plan disposition](reviews/build-plan-review-disposition-2026-08-13.md)
- [Roadmap](roadmap/README.md) and [deferred items](roadmap/deferred.md)
- [Source-lineage reference](TRUST-LAYER-CONTINUATION-REFERENCE.md)
- [Historical EOD handoff](EOD-SUMMARY-2026-08-28.md) and [original MVP brief](MVP.md)

The future egress design is outside this delivery. No network PEP is implemented. Private session memory and local reference repository checkouts are not prerequisites for building or reviewing this repository.
