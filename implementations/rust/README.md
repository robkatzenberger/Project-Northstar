# Rust implementation

**Status:** Planned authoritative core — not started

Intended use: small security-critical TL-PX 0.2 authority and **policy enforcement point** (PEP) that must fail closed. Implementation begins only after the v0.2 contract, canonicalization profile, schemas, and golden conformance fixtures are accepted.

## Planned shape (draft)

```text
rust/
  Cargo.toml
  src/lib.rs
  src/bin/tlpx.rs
  tests/
```

## Requirements when implemented

- Follow the proposed [hardened build specification](../../docs/BUILD-SPEC-SHEET.md), not by rewriting frozen TL-PX v0.1
- Conform to the future TL-PX v0.2 spec, schemas, JCS/hash fixtures, state model, receipts, and reason codes
- Deterministic fail-closed policy and authenticated single-use atomic authorization
- Preserve distinct intent, authorized-action, executed-action, and observed-result evidence
- No `unsafe` Rust in the authorization path
- Minimal reviewed dependencies; no in-process LLM or arbitrary policy code
- Forced mediation and a strong embedding/sidecar story next to protected agent runtimes

## Why Rust later

Memory safety + performance for the hot path that wraps tool I/O.
