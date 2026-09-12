# Apex Auditor v0.0.2

The decision gate has two business outcomes: **APPROVE** or **ESCALATE**. An authenticated human resolves an escalation with **APPROVE** or **DENY** and a required context comment. The original assessment and human decision remain separate saved records.

Switchboard verifies the agent's credential, whitelist, and exact action-target grant before the decision gate runs. Authority failures return **BLOCKED**; malformed or unsupported evaluations return **ERROR**. Neither is eligible for human review. This version assesses only `{agentId, action, target}` and records `NOT_EXECUTED` throughout.

The agent submits its own intent through the [client workflow](agent-integration.md), guided by a [portable instruction template](agent-instructions.md). The human page handles enrollment, connection, rules, reviews, and audit. The agent retrieves its own saved assessment and human context; the human does not enter every agent request into a form.

## Run the local walkthrough

From `apex/`, using Node 22 or newer:

```sh
npm test
npm run demo:auditor
```

Open the printed local URL, normally `http://127.0.0.1:43129`. Startup prints the path to `operator.key`; open that file locally and enter its value in Auditor sign-in. The operator key controls rules, registry changes, credential issuance, human review, and audit access. An agent credential cannot use those operator routes.

The demo keeps its journal and registry state in `var/auditor-demo/`. Its synthetic `demo-agent` has these grants:

| Action | Exact target |
| --- | --- |
| `read_file`, `write_file`, `delete_file` | `file:demo` |
| `execute_file` | `executable:demo` |
| `send_email` | `recipient:demo` |

Initial rules contain only a comment. Every authenticated, in-scope supported action therefore starts with ESCALATE. Enter and save:

```text
# Human rules for the demo
APPROVE read_file file:demo
ESCALATE delete_file file:demo
ESCALATE send_email recipient:demo
```

Use the connection area to provision a credential for `demo-agent`. Put that credential in the agent's private file and configure `APEX_URL`, `APEX_AGENT_ID`, and `APEX_CREDENTIAL_FILE`. Give the agent the [participant instructions](agent-instructions.md) and actual client path as described in the [connection guide](agent-integration.md). The template is also served read-only at `/agent-instructions.md`; copying it into an agent's supported instruction location is an explicit integration step. Never give the agent the operator key or put its credential into instruction text.

An instructed agent submits `read_file` with target `file:demo` through `apex-agent.mjs submit`, with the exact three-field intent on standard input. That example has an APPROVE rule. For `send_email` and target `recipient:demo`, the agent submits the intent and receives ESCALATE. It retains the assessment ID, pauses the proposed action, and uses `apex-agent.mjs status <assessment-id>` to observe its own saved outcome. Submitting an assessment does not read the file or send an email.

The human review and audit lists refresh every three seconds while the signed-in page is visible. Inspect the pending assessment's saved identity, request, policy, matching rule, and receipt. Choose APPROVE or DENY and enter a context comment explaining that choice. The service records the human outcome against that assessment; it does not rewrite ESCALATE into another machine result. The agent's next status lookup receives that separate outcome and comment.

An in-scope `write_file` request has no matching rule in this example and also escalates. A request for `file:other` returns BLOCKED because the grant does not cover it. No review button should offer to override that authority failure. Nothing is read, written, deleted, executed, or sent by these assessments or reviews.

The collapsed **Developer test console** retains manual intent submission for diagnostics. It is optional; normal agent participation uses the client. Issuing a replacement credential invalidates that agent's previous credential, and server restart requires provisioning a new one.

The retained registry UI is at `/index.html`; sign in on the auditor page first. Its version-checked edits use the same journal. The older `/api/credentials` token utility produces an unregistered random token; principal-bound issuance uses `/api/gate/credentials`.

`npm run gate` (also `npm run operator`) uses `var/auditor/` and initializes from `config/agents.json`. The existing sample registry does not automatically acquire demo grants. `APEX_DATA_DIR`, `APEX_REGISTRY_FILE`, and `PORT` select the data directory, seed registry, and port. Use one server process per data directory.

## Agent participation

The client takes `APEX_URL` (default `http://127.0.0.1:43129`), `APEX_AGENT_ID`, and `APEX_CREDENTIAL_FILE`. Its `submit` command reads exactly `{agentId,action,target}` from standard input and attaches the agent credential separately. Its `status <assessment-id>` command performs one authenticated lookup of that principal's saved assessment and review. The [connection guide](agent-integration.md) contains copyable examples and exit-code meanings.

Agents receive `{assessment:{id,recordedAt,request,result},review,current}`. A `null` review means no human outcome has been recorded. `current: false` means policy or authority has changed and an old approval cannot support progression; a fresh assessment is required. A recorded human DENY remains a denial. The status route does not expose another principal's assessment, general audit access, rule editing, or human-resolution privileges.

Participant instructions ask the agent to invoke this path automatically during its task and pause an escalated action until it observes the recorded outcome. This is cooperative integration. Markdown does not intercept tools, install a runtime hook, or force an agent through Apex, and the client does not execute the action. Other user and tool authorization requirements still apply after any assessment approval.

## Deterministic human rules

Each line has exactly three space- or tab-separated fields:

```text
APPROVE action_id exact_target
ESCALATE action_id exact_target
```

The five action IDs are `send_email`, `delete_file`, `execute_file`, `write_file`, and `read_file`. Targets are opaque, case-sensitive identifiers of at most 256 characters: letters, digits, `.`, `_`, `:`, `/`, and `-`, starting with a letter or digit. The gate does not resolve them to actual recipients, paths, or executables.

Blank lines and whole-line `#` comments are allowed with LF or CRLF endings. There are no wildcards, aliases, inline comments, executable expressions, or natural-language conditions. `read_only` is a planned permission preset, not an action. The whole policy is validated before activation. Malformed or duplicate rules, more than 200 rules, or more than 32 KiB of UTF-8 text block the save.

ESCALATE takes precedence over APPROVE for the same action and target, independent of line order. All matching lines appear in the result. No matching rule means ESCALATE after successful authority screening. An invalid or unknown action is an ERROR, not an escalation. New policy saves reject the historical terms `ALLOW`, `DENY`, and `REQUIRE_APPROVAL`.

Assessments use saved rules. Saves include the loaded version; stale edits return HTTP 409 and invalid edits leave the active policy unchanged. Gate results identify `version: "0.0.2"` and `decisionModel: "approve_escalate_v1"`.

## Human decisions with context

Only a saved v0.0.2 ESCALATE assessment that passed Switchboard is reviewable. The operator submits its assessment ID, APPROVE or DENY, and a nonblank comment of at most 2000 UTF-8 bytes after trimming. Context explains the human decision; it does not replace missing material verification or grant new authority.

The service obtains the declaration and assessment from its journal. The client cannot submit a replacement intent, policy, verified principal, or reviewer identity. A separate `human_review` event binds the assessment ID and hash, decision, comment, and authenticated `operator` role. This is a single local role, not attribution to multiple named human reviewers.

Each assessment can receive one human decision. A repeated or conflicting resolution returns HTTP 409, including an identical retry. Before APPROVE, the service checks the recorded policy version/hash, registry identity/version, whitelist membership, and exact grant against current state. A change requires a fresh assessment before approval. DENY can close an existing escalation after state has changed. BLOCKED, ERROR, machine APPROVE, and historical assessments cannot be resolved through this route.

Removing machine DENY does not by itself prevent gaming. Separate agent/operator credentials, binding to the saved assessment, one recorded resolution, and current-state checks provide the concrete controls here. The local host and operator remain trusted, and the three-field intent does not bind complete email/file/execution details. Human approval is an audit outcome, not an execution credential.

## Journal, restart, and migration

Every successful `store.assess` / `/api/gate/evaluate` completion journals an `assessment` receipt before the response returns — including **ERROR** and **BLOCKED**, not only APPROVE and ESCALATE. The receipt carries id, recordedAt, request projection, and the full result (`status`, `code`, `reason`, stage, policy hash, switchboard). ERROR/BLOCKED remain non-reviewable; the journal exists so operators can audit and debug. Transport and pre-assess failures (invalid JSON, oversized body, faults before assess) stay HTTP errors and are not assessment records.

`audit.jsonl` stores policy saves, complete registry snapshots, credential-issuance events, assessments, and human-review events. State changes and assessments share one serialized queue. A successful response follows writing, syncing, and closing its journal record. Storage failure blocks successful assessment/review responses and state activation. Transport failures such as invalid JSON are not assessment records.

The journal owns active policy and registry after initialization. Seed-file edits are not a live override. Restart restores assessments and human outcomes, including unresolved escalations. Policy hashes identify exact text; registry and policy saves increment their versions. The journal has a 16 MiB capacity limit. Incomplete records or invalid policy/registry/review history block startup.

When an existing v0.0.1 journal is opened, its policy text is validated with the historical grammar. The service preserves the old bytes and appends a new, comment-only `approve_escalate_v1` policy requiring fresh human rule entry. It does not reinterpret old DENY rules as APPROVE, reactivate old rule text, or convert old assessments into current human reviews. The [v0.0.1 guide](auditor-v0.0.1.md) and [verification report](../test/reports/auditor-v0.0.1-2026-09-10/REPORT.md) retain their historical scope.

Operator keys remain in `operator.key`. Browser sign-in uses an HttpOnly, SameSite cookie replaced at restart. The server holds agent credentials only in memory; the client reads its provisioned credential from a private file outside instruction text and git. Reissuance replaces the previous credential, and server restart invalidates all agent credentials. Credential fields are excluded from assessments; the HTTP boundary also blocks a submitted credential copied into the declaration. Keep identifiers and context comments free of secrets.

This is one local server with a plain journal. There is no multi-process journal locking, remote identity deployment, tamper-proof ledger, or protection from another process with the same OS user's file access. IDs and hashes support inspection, not independent attestation. Repeated intent submissions create separate assessments. Review expiry, per-condition reviewer roles, material-action schemas, request idempotency, and execution remain future work.

## HTTP contract

All requests target the local server and JSON bodies are bounded to 64 KiB. Operator routes require the sign-in cookie or operator key as a Bearer credential. Agent assessment needs its own body credential and no operator session; agent status uses its own credential as Bearer authentication.

| Route | Request and result |
| --- | --- |
| `POST /api/operator/session` | `{key}` establishes the operator browser session |
| `GET /agent-instructions.md` | Public, read-only participant template; no embedded credentials or automatic installation |
| `GET /api/action-catalog` | Public catalog; enforced action IDs with `versionLabel: "0.0.2"`; material fields remain design metadata |
| `GET /api/gate-rules` | Operator reads `{version,text,decisionModel,hash}` |
| `PUT /api/gate-rules` | Operator sends `{version,text}` to validate, persist, and activate current-model rules |
| `GET` / `PUT /api/registry` | Operator reads or version-checks a saved registry; PUT body is `{registry}` |
| `POST /api/gate/credentials` | Operator sends `{agentId}` to issue or replace its temporary credential |
| `POST /api/gate/evaluate` | `{credential,intent:{agentId,action,target}}` creates an assessment and receipt |
| `GET /api/agent/assessments/:id` | Agent Bearer credential reads only its own `{assessment:{id,recordedAt,request,result},review,current}`; review is null or the saved human event |
| `GET /api/reviews` | Operator reads `{reviews:[...]}`, at most 100, pending first; each item contains `assessmentId`, `recordedAt`, stored `request`, `result`, and `review` or null |
| `POST /api/reviews/decision` | Exact body `{assessmentId,decision,comment}`; successful HTTP 201 returns `{review}` |
| `GET /api/audit?limit=20` | Operator reads recent records, newest first, at most 100 |

Example human-decision body, using the actual saved receipt ID in place of the illustrative value:

```json
{
  "assessmentId": "saved-assessment-id",
  "decision": "DENY",
  "comment": "Recipient verification was not established for this request."
}
```

Missing operator authentication returns 401. Invalid review input returns 400; an unknown assessment returns 404. Nonreviewable, already resolved, and stale-approval cases return 409 with a specific code. Storage failure returns a blocking error; a missing or unreadable response is never approval.

Assessment responses preserve the underlying Switchboard result, policy version/hash, matching rules, assessment status/reason, and audit ID/time. Human decisions are separate records with `execution: "NOT_EXECUTED"`; the original assessment remains immutable.

Source: [gate](../src/gate.js), [server](../src/server.js), [journal](../src/auditor-store.js), [agent client](../src/agent-client.js), [CLI](../bin/apex-agent.mjs), and [browser](../operator/gate.html). The [build plan](../BUILD-PLAN.md) identifies the full material-intent and reviewer-route milestones still open. Existing v0.0.2 reports retain their source and scope; later client/interface changes require their own verification. Earlier Northstar and APEX-Lite evidence does not verify this increment.
