# Apex Auditor v0.0.1 — local verification

**Result:** local builder verification passed on 2026-09-10. This verifies the small auditor service and human rule editor in the uncommitted Apex working tree. It is not independent security acceptance or a deployed third-party service.

The repository HEAD was `b97c18ed56d1113ec88856309ef57150e675b1e3`; that commit does not contain these new Apex changes. [source.json](source.json) identifies the tested source files by SHA-256. Environment: macOS arm64, Node v25.5.0, installed Chrome 152.0.7977.85. Local HTTP and browser checks required permission to run outside the shell sandbox.

## Results

| Check | Result | Evidence |
| --- | --- | --- |
| `npm test` in `apex/` | 69 passed; 0 failed | [Raw test output](tests.txt) |
| New gate and auditor tests within that suite | 18 passed | [Gate tests](../../gate.test.js), [auditor tests](../../auditor.test.js) |
| Browser workflow | Passed with 4 recorded assessments and no page errors | [Browser results](browser.json), [reusable check](../../browser/auditor.mjs) |
| Desktop and 390 px layout | Inspected; no horizontal overflow | [Desktop](desktop.png), [mobile](mobile.png) |
| Pinned APEX-Lite snapshot | All 29 files match original lengths and SHA-256 hashes | [Source verification](source.json), [reference manifest](../../../reference/apex-lite.manifest.json) |

The 69-test total includes existing Switchboard, registry, and separate audit-prototype tests. Passing those tests does not make the separate audit prototype part of the new auditor's authority path.

The browser check signs in as the operator, enters rules, rejects an invalid wildcard with a line error, issues real temporary agent credentials, and checks `read_file` (ALLOW), `delete_file` (DENY), `send_email` (REQUIRE_APPROVAL), and unmatched `write_file` (DENY). It verifies receipt presence, credential clearing, persisted assessment outcomes, saved rules after reload, and mobile overflow. Temporary browser credentials and data are removed after the check.

The backend tests cover operator/agent separation, authenticated scope checks before policy, deterministic precedence and default denial, concurrent edit conflicts, serialized registry revocation, restart recovery, audit write failure, incomplete-journal rejection, and credential redaction. An assessment is returned only after its event has been written, synced, and closed successfully.

## Scope

This is one local auditor process with an operator-controlled journal. It screens exact action/target declarations and records results. It does not validate full action material, collect human approvals, execute operations, issue execution credentials, protect against a hostile same-UID process, or provide a tamper-proof ledger. `REQUIRE_APPROVAL` remains blocked. Earlier Northstar marker and Slice 3.9 checks were not part of this verification.

See the [auditor guide](../../../docs/auditor-v0.0.1.md) for startup, rule syntax, API, and storage behavior. The optional browser check requires Playwright in the verification environment; `APEX_PLAYWRIGHT_MODULE`, `APEX_BROWSER_EXECUTABLE`, and `APEX_BROWSER_REPORT` select its module, installed browser, and evidence directory. They do not add runtime dependencies to Apex.
