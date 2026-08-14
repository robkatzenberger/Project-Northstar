# TL-PX 0.2 schemas

**Status:** Slice 2.3 — record and object schemas. Not a 0.2 runtime.

Normative semantics: [`../../docs/standard/SPEC-v0.2.md`](../../docs/standard/SPEC-v0.2.md).  
JCS/hash fixtures: [`../../tests/fixtures/tlpx-0.2/jcs/`](../../tests/fixtures/tlpx-0.2/jcs/).  
Reason codes: [`reason-codes.json`](./reason-codes.json).

These schemas are the contract Rust and other languages must implement. The JS `validate-v02.mjs` oracle checks them. The 0.1 gate must not emit these records until a separately versioned 0.2 adapter exists.

Do not place 0.2 documents under `../tlpx-0.1/`.
