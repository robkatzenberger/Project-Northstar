# Rust implementation

**Status:** Skeleton started — contract types + JCS/hash oracle  
**Crate:** `tlpx` 0.2.0  
**Not yet:** evaluate/claim state machine, SQLite, PEP / `tlpx-run`

This is the start of the authoritative 0.2 core. It does not replace the JS 0.1 gate. It must match `tests/fixtures/tlpx-0.2/jcs/golden.json` exactly.

## Layout

```text
rust/
  Cargo.toml
  src/lib.rs
  src/jcs.rs
  src/hash.rs
  src/types.rs
  src/error.rs
  tests/jcs_golden.rs
```

## Commands

```bash
cd implementations/rust
cargo test
```

Requires a local Rust toolchain (`rustc` / `cargo`). Prod dependency: `sha2`. `serde`/`serde_json` are test-only (load the golden file).

## Requirements

- Follow [`docs/BUILD-SPEC-SHEET.md`](../../docs/BUILD-SPEC-SHEET.md)
- Conform to [`docs/standard/SPEC-v0.2.md`](../../docs/standard/SPEC-v0.2.md) and `schemas/tlpx-0.2/`
- No `unsafe` in this crate
- No LLM, no dynamic policy code
- Do not implement the abandoned `AUTHORIZED` snapshot token

## Next (when opened)

Smallest evaluate → authorize → atomic claim path against the 0.2 schemas, then OS-enforced PEP (slice 3.9).
