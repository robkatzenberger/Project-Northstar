# Apex agent-submission workflow — local verification

Verified on 2026-09-10 America/Chicago (artifact timestamps use UTC). The human page now receives agent requests automatically and reserves manual submission for a collapsed developer console. The agent uses a participant instruction template and a credentialed client to submit intent and retrieve its own human-review outcome.

This is local builder verification for the uncommitted Apex working tree at repository HEAD `b97c18ed56d1113ec88856309ef57150e675b1e3`. That commit does not contain the new Apex files. [source.json](source.json) records the tested source hashes and environment: Node v25.5.0, macOS arm64. The pinned APEX-Lite snapshot still matches all 29 original file hashes.

| Verification | Result | Evidence |
| --- | --- | --- |
| Full `npm test` | 116 passed, 0 failed | [Test output](tests.txt) |
| Browser with independent HTTP agent | Six incoming assessments, three human outcomes, automatic arrival and draft preservation; no page errors | [Browser results](browser.json), [desktop](desktop.png), [mobile](mobile.png) |
| Actual AI participant following Markdown instructions | One CLI submission and one status lookup, exactly matched to the running auditor's journal | [Agent observation](ai-agent-observation.md), [reconciliation](ai-verification.json) |
| Agent request evidence | `read_file file:demo`, Switchboard PASS, gate ESCALATE, pending review at lookup | [Submission](ai-submission.json), [status](ai-status.json), [saved gate event](ai-gate-record.json) |

The browser driver sent requests through an agent credential without operator cookies or a manual request form. It verified automatic queue arrival, three-second visible-page updates, preservation of focused/unsent human context, separate credential provisioning, the closed developer console, both human outcomes, stale approval handling, receipt binding, and 390 px overflow. [The review form](human-review.png) and [mobile outcome](human-outcome-mobile.png) were visually inspected.

The new client tests use real gate records with mocked HTTP transport to check receipt/schema validation, exact binding, malformed responses, timeout/cancellation, error cleanup, credential redaction, and stale status handling. The auditor integration tests exercise the live private status endpoint: agents cannot inspect another principal's record or gain operator access, and policy changes make an old approval noncurrent.

The separate AI participant read [the instruction template](../../../docs/agent-instructions.md), chose `read_file` for its assigned synthetic resource, invoked the CLI itself, and observed its own receipt. No human entered that declaration. The participant stopped when the result required review. Its complete response bytes match the stored assessment; the retained evidence contains no credential. Its temporary credential file was removed after verification. The synthetic pending review remains visible in the running demo.

## Scope

This proves one guided cooperative AI-to-Apex submission plus the tested client/dashboard paths. It does not prove that arbitrary agents always obey Markdown, that all tools are intercepted, or that the host isolates hostile agents. A connected runtime must be given the instructions and its own credential configuration. No hooks, protected tool adapters, full material-action validation, or execution permission were added. All tested outcomes remain `NOT_EXECUTED`.

Prior [v0.0.2 evidence](../auditor-v0.0.2-2026-09-10/REPORT.md) remains unchanged and retains its original source scope. Use the [connection guide](../../../docs/agent-integration.md) to configure a participant.
