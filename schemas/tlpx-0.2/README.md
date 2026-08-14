# TL-PX 0.2 schemas

**Status:** Accepted slice 2.3 object and evaluation/authorization record schema core. The execution-receipt schema is provisional; cancellation/reconciliation and revocation evidence closure is deferred. Not a 0.2 runtime.

Normative semantics: [`../../docs/standard/SPEC-v0.2.md`](../../docs/standard/SPEC-v0.2.md).  
JCS/hash fixtures: [`../../tests/fixtures/tlpx-0.2/jcs/`](../../tests/fixtures/tlpx-0.2/jcs/).  
Reason codes: [`reason-codes.json`](./reason-codes.json).

These schemas are the contract Rust and other languages must implement. The JS `validate-v02.mjs` oracle checks them. The 0.1 gate must not emit these records until a separately versioned 0.2 adapter exists.

Accepted 2.3 conformance covers Submitted Intent, Authorized Action, Executed Action, decision, evaluation error, operator action, authorization, and authorization claim. It does not establish a complete execution receipt, cancellation/reconciliation evidence, or revocation record. See the 2026-08-14 acceptance clarification in the normative specification before implementing those surfaces.

Do not place 0.2 documents under `../tlpx-0.1/`.
