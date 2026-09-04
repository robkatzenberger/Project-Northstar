# TL-PX 0.2 schemas

**Status:** Accepted slice 2.3 object and evaluation/authorization record schema core plus later unaccepted contract deltas. Slice 3.6 supplies an implementation-driven terminal execution-receipt candidate with bounded result/cancellation fields and durable reconciliation states, but it is not independently accepted and does not retroactively broaden 2.3 acceptance. Portable approval-expiry and revocation evidence remain deferred. Not a 0.2 runtime.

Normative semantics: [`../../docs/standard/SPEC-v0.2.md`](../../docs/standard/SPEC-v0.2.md).  
JCS/hash fixtures: [`../../tests/fixtures/tlpx-0.2/jcs/`](../../tests/fixtures/tlpx-0.2/jcs/).  
Reason codes: [`reason-codes.json`](./reason-codes.json).
Policy manifest fixture: [`../../tests/fixtures/tlpx-0.2/policy/manifest-golden.json`](../../tests/fixtures/tlpx-0.2/policy/manifest-golden.json).
Typed-action/hash fixture: [`../../tests/fixtures/tlpx-0.2/actions/golden.json`](../../tests/fixtures/tlpx-0.2/actions/golden.json).

These schemas are the contract Rust and other languages must implement. The JS `validate-v02.mjs` oracle checks them. The 0.1 gate must not emit these records until a separately versioned 0.2 adapter exists.

An adapter-started `tlpx.execution` binds `adapter_principal` and `adapter_binary_hash`; both may be explicit null only for `LEASE_EXPIRED` recovered before adapter start. They must otherwise be present or absent together.

Accepted 2.3 conformance covers Submitted Intent, Authorized Action, Executed Action, decision, evaluation error, operator action, authorization, and authorization claim. It does not establish the later slice 3.6 execution-receipt candidate or any portable revocation record. See the 2026-08-14 acceptance clarification in the normative specification before implementing those surfaces.

Local slice 2.4 commit `a87f822` adds `policy-bundle.schema.json`, the `northstar:policy-bundle:v1\0` hash domain, strict provenance/active-window/supersession rules, and a deterministic JavaScript contract oracle. Local slice 3.1 commit `1addb5c` consumes that schema in Rust and binds it to the configured exact-match policy content. Exact-commit/full Section 3 verification is deferred; neither increment is accepted, standardizes the JavaScript 0.1 YAML language, or makes either implementation a conforming 0.2 runtime.

Local slice 3.2 commit `e835c4e` makes the Authorized Action schema enforce `effective_risk >= derived_risk`, matching the Rust type invariant. Shared fixtures and strict JavaScript/Rust tests pin validation-before-hash behavior, required nullable digests, optional retry omission, the three distinct object hashes, and the exact nine-field Action Binding. Bounded exact-commit checks passed; this is not runtime acceptance.

Local slice 3.3 commit `c19b1d2` uses the accepted `tlpx.operator_action` `CANCEL` shape for pending cancellation. Local slice 3.4 commit `133cd94` additionally emits schema-valid `APPROVE`/`REJECT` records with the displayed action hash, policy route, and renderer identity/version. The reason catalog includes stable approval authorization/presentation/expiry blocks. The operator-action schema has no expiry outcome, so expiry remains durable authority state without a claimed portable record.

Local slice 3.6 commit `9028346` makes `tlpx.execution` terminal-only, adds bounded result and cancellation evidence, and treats `EXECUTION_OUTCOME_UNKNOWN` plus `RECONCILIATION_REQUIRED` as authority-process states rather than receipt outcomes. Its full exact-commit builder matrix passed; the contract remains unaccepted pending full Section 3 and independent review. No `tlpx.revocation` record exists.

Do not place 0.2 documents under `../tlpx-0.1/`.
