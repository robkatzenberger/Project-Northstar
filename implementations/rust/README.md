# Rust implementation

**Status:** Placeholder — not started  

Intended use: high-assurance **policy enforcement point** (PEP) / sidecar that must fail closed under load.

## Planned shape (draft)

```text
rust/
  Cargo.toml
  src/lib.rs
  src/bin/tlpx.rs
  tests/
```

## Requirements when implemented

- Conform to [TL-PX SPEC v0.1](../../docs/standard/SPEC-v0.1.md)
- Deterministic evaluate path; chain-verified authorize
- Strong story for embedding next to agent runtimes

## Why Rust later

Memory safety + performance for the hot path that wraps tool I/O.
