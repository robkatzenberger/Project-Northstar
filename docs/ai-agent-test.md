# Testing Switchboard with actual AI agents

The [completed Cedar and Maple run](../test/reports/ai-live-20260910T043406057Z/REPORT.md)
records twenty AI-initiated requests, both participants' observations, the gate
decisions, and their reconciliation.

Two AI subagents, Cedar and Maple, formulate declared intents and invoke a client
through their own tool calls. One persistent gate process imports the actual
Switchboard module. The client attaches the caller's temporary credential and
records what it submitted and received; it neither constructs the intent nor
decides the screening outcome.

The initial experiment uses these synthetic participants:

| AI participant | Registry identity | Independent probe proposed by the agent |
| --- | --- | --- |
| Cedar | `agent.ai.cedar` | Append `/` to `gate:test` to test exact target matching |
| Maple | `agent.ai.maple` | Add an unexpected `note` field inside intent to test strict validation |

Each also submits a normal ping, an ungranted target, the other participant's
claimed identity with its own credential, and a request without a credential.
There are five requests per agent per round. The controller swaps whitelist
membership between two rounds after both participants have received every
response in the first round. Each participant is asked to inspect each response
and author a separate narrative of its observations. No action is executed.

## Local transport

From Apex, start `node test/ai-live/gate.mjs`. It creates a new evidence directory
and a private temporary mailbox, then prints their paths. The gate remains alive
while AI participants work. Credentials exist only in that temporary mailbox and
gate memory. The mailbox is removed on normal shutdown. The previous live test
and process-agent stress-test directories are preserved.

Each AI participant invokes:

```sh
node test/ai-live/client.mjs <temporary-mailbox> <assigned-agent-id>
```

The client accepts one JSON object on standard input with `requestId`, `round`,
`credentialMode` (`own` or `missing`), and the agent's `intent`. It loads only the
assigned credential, submits the request, waits for the actual gate response,
and writes its own transport log. The AI inspects that response before continuing.
Credentials are not printed or included in retained records.

The controller publishes `control.json` changes through temporary-file rename
to change rounds or request shutdown. Gate shutdown counts twenty requests,
requires round two and an empty request queue, checks source hashes and retained
evidence for literal credentials, and removes its temporary mailbox. A timeout
or incomplete shutdown is recorded as failure. The local transport is solely a
test adapter, with no public network listener or production-hosting claim.

## Evidence and verification

Before the participants start, the controller saves their proposed cases and
expected outcomes in `test-plan.json`. Registry snapshots, source hashes, host
context, gate decisions, and client observations are retained per run. The two
AI participants write their own narrative reports in `agents/`.

After the gate stops, run:

```sh
node test/ai-live/verify.mjs test/reports/<run-directory>
```

This reconciles every planned request with exactly one submission, gate
decision, and agent observation. It compares the full results, identities,
intents, scope grants, registry versions, and expected screening codes. It also
requires a complete shutdown, unchanged source, and credential cleanup. It
writes a new `verification.json`; it does not overwrite an existing report.

The evidence supports a guided AI-to-Switchboard integration test. Unlike the
ten-process stress test, AI participants decide how to express the given probes,
invoke the tool, read its responses, and report their interpretation. This does
not establish that arbitrary AI agents will always use the gate, correctly
translate arbitrary user requests, or lack alternative access to tools. All
participants share the host environment; separate credentials and instructions
do not establish hostile-process isolation. PASS remains eligibility for later
constraint screening, and the full human-verification workflow is still planned.
