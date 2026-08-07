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


