# TL-PX 0.2 schemas

**Status:** Accepted slice 2.3 object and evaluation/authorization record schema core plus unaccepted slice 2.4 and 3.2 contract deltas. The execution-receipt schema is provisional; cancellation/reconciliation and revocation evidence closure is deferred. Not a 0.2 runtime.

Normative semantics: [`../../docs/standard/SPEC-v0.2.md`](../../docs/standard/SPEC-v0.2.md).  
JCS/hash fixtures: [`../../tests/fixtures/tlpx-0.2/jcs/`](../../tests/fixtures/tlpx-0.2/jcs/).  
Reason codes: [`reason-codes.json`](./reason-codes.json).
Policy manifest fixture: [`../../tests/fixtures/tlpx-0.2/policy/manifest-golden.json`](../../tests/fixtures/tlpx-0.2/policy/manifest-golden.json).
Typed-action/hash fixture: [`../../tests/fixtures/tlpx-0.2/actions/golden.json`](../../tests/fixtures/tlpx-0.2/actions/golden.json).

These schemas are the contract Rust and other languages must implement. The JS `validate-v02.mjs` oracle checks them. The 0.1 gate must not emit these records until a separately versioned 0.2 adapter exists.

Accepted 2.3 conformance covers Submitted Intent, Authorized Action, Executed Action, decision, evaluation error, operator action, authorization, and authorization claim. It does not establish a complete execution receipt, cancellation/reconciliation evidence, or revocation record. See the 2026-08-14 acceptance clarification in the normative specification before implementing those surfaces.

Local slice 2.4 commit `a87f822` adds `policy-bundle.schema.json`, the `northstar:policy-bundle:v1\0` hash domain, strict provenance/active-window/supersession rules, and a deterministic JavaScript contract oracle. Local slice 3.1 commit `1addb5c` consumes that schema in Rust and binds it to the configured exact-match policy content. Exact-commit/full Section 3 verification is deferred; neither increment is accepted, standardizes the JavaScript 0.1 YAML language, or makes either implementation a conforming 0.2 runtime.

Local slice 3.2 commit `e835c4e` makes the Authorized Action schema enforce `effective_risk >= derived_risk`, matching the Rust type invariant. Shared fixtures and strict JavaScript/Rust tests pin validation-before-hash behavior, required nullable digests, optional retry omission, the three distinct object hashes, and the exact nine-field Action Binding. Bounded exact-commit checks passed; this is not runtime acceptance.

Do not place 0.2 documents under `../tlpx-0.1/`.
