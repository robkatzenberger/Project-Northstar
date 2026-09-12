# Connect an agent to Apex

The agent submits its own intent through the [client](../src/agent-client.js) and [CLI](../bin/apex-agent.mjs). The human uses the auditor page to manage enrollment, exact grants, rules, escalations, and the audit trail. The page's collapsed **Developer test console** retains manual submissions for diagnostics.

The [participant instruction template](agent-instructions.md) is available from the running server at `/agent-instructions.md`. Copy it into the participating agent's supported instruction location; configure its client path and connection environment separately. This works with an agent that can invoke the Node client and follow those instructions. It does not install provider hooks or enforce tool interception.

## Operator setup

1. From `apex/`, start the synthetic walkthrough with `npm run demo:auditor`. The server and client default to `http://127.0.0.1:43129`. Use the printed URL if you select a different `PORT`, and set the agent's `APEX_URL` to that address.
2. Sign in using the local operator key. Enroll the agent and configure its whitelist status and exact action-target grants. The demo already provides `demo-agent` and synthetic grants.
3. Enter the deterministic rules. For example, `APPROVE read_file file:demo` and `ESCALATE send_email recipient:demo` produce different assessment paths for an enrolled, in-scope agent. An unmatched eligible request escalates.
4. Use the human connection area to provision that agent's credential. Save it as a private file accessible to that agent's client, outside the repository and instruction text. Supply only the file path in `APEX_CREDENTIAL_FILE`; never supply the operator key to an agent.
5. Give the agent the template, actual client path, URL, agent ID, and credential-file path. The agent can then submit intents itself; the operator does not enter each declaration.

This setup is per agent. Reissuing a credential replaces that principal's previous credential, and server restart clears all agent credentials. Provision a fresh credential after restart; saved assessments and human decisions remain in the journal.

## Agent connection and commands

Node 22 or newer is required. The agent runtime supplies these environment values; paths below are placeholders to replace with the configured locations:

```sh
export APEX_URL='http://127.0.0.1:43129'
export APEX_AGENT_ID='demo-agent'
export APEX_CREDENTIAL_FILE='/private/agent-config/demo-agent.credential'
```

From the Apex folder, the client commands are:

```sh
node bin/apex-agent.mjs submit <<'JSON'
{"agentId":"demo-agent","action":"send_email","target":"recipient:demo"}
JSON

node bin/apex-agent.mjs status saved-assessment-id
```

`submit` reads exactly `{agentId,action,target}` from standard input and attaches the file credential in the API request. Use the returned assessment ID in `status`; do not substitute the illustrative ID. The credential remains separate from declaration and audit records. Neither command operates on the declared target.

| Exit code | Meaning |
| --- | --- |
| `0` | Current gate or human approval assessment; no execution permission |
| `2` | Escalated assessment still awaiting a human decision |
| `3` | Authority blocking or recorded human denial |
| `1` | Input, transport, service, or result error; no approval can be inferred |

On ESCALATE, the agent pauses the proposed action and observes its own saved status at a controlled interval. `status` makes one lookup. The human review queue and audit trail refresh every three seconds while the signed-in auditor page is visible. The operator opens the stored assessment, chooses APPROVE or DENY, and supplies required context. A later agent lookup returns that separate human decision and comment.

`current: false` means the recorded policy or authority no longer matches current state. A previous approval is no longer a current assessment; obtain a fresh assessment. A human DENY remains a denial. BLOCKED and ERROR assessments never become reviewable by polling. Repeated submissions are separate journal entries, so a lost response needs reconciliation rather than blind resubmission.

## Agent API

The CLI uses the same local service as the human page:

| Route | Authentication and response |
| --- | --- |
| `GET /agent-instructions.md` | Public, read-only participant template; contains no credentials |
| `POST /api/gate/evaluate` | Body `{credential,intent:{agentId,action,target}}`; returns the saved assessment with `audit.id` and `audit.recordedAt` |
| `GET /api/agent/assessments/:id` | Agent credential as Bearer authentication; returns only an assessment owned by that authenticated principal |

The status response is:

```json
{
  "assessment": {
    "id": "saved-assessment-id",
    "recordedAt": "recorded timestamp",
    "request": {"agentId":"demo-agent","action":"send_email","target":"recipient:demo"},
    "result": {"status":"ESCALATE"}
  },
  "review": null,
  "current": true
}
```

This shows the envelope; the real `result` contains the complete stored assessment, and a resolved `review` contains its separate event, decision, and context comment. `current` reports the policy/authority comparison, not execution permission. Agents do not receive operator rules, general audit access, another agent's records, or human-resolution rights through this route.

## Scope and verification

The API and client provide a cooperative submission-and-observation path in Auditor v0.0.2. The template asks the agent to invoke that path automatically when a task requires an Apex assessment. Markdown alone cannot force mediation, intercept tools, or prove the agent follows the instructions. Full material-action validation, execution authorization, runtime hooks, and protected-resource adapters remain future work.

The [agent-workflow report](../test/reports/agent-workflow-2026-09-10/REPORT.md) records 116 passing tests, automatic dashboard arrival, and a guided AI participant submitting and observing through this client. Existing [v0.0.2 evidence](../test/reports/auditor-v0.0.2-2026-09-10/REPORT.md) retains its earlier source and scope. See the [auditor guide](auditor-v0.0.2.md) for policy, review, persistence, and migration details.
