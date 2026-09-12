# Apex Auditor v0.0.1

A small local auditor for agent intent. A human controls enrollment and deterministic rules. An agent submits its credential and `{agentId, action, target}`. Switchboard screens identity, whitelist, and exact authority first; the gate then checks the action catalog and saved rules. The auditor records the assessment before returning a receipt.

This version checks action-and-target scope. It does not validate complete email/file/execution details, collect human approvals, issue execution credentials, or execute actions. `REQUIRE_APPROVAL` remains blocked.

## Run it

From `apex/`, with Node 22 or newer:

```sh
npm test
npm run demo:auditor
```

Open the printed local URL, normally `http://127.0.0.1:43127`. Startup prints the **path** of `operator.key`; the key itself is not logged. Open that file locally and enter its value in Auditor sign-in. The operator key manages rules, registry, credential issuance, and audit access. Agent credentials cannot use those operator routes.

The demo uses `var/auditor-demo/` and a `demo-agent` with these exact grants:

| Action | Target |
| --- | --- |
| `read_file`, `write_file`, `delete_file` | `file:demo` |
| `execute_file` | `executable:demo` |
| `send_email` | `recipient:demo` |

Initial rules contain only a comment, so every in-scope request starts with a gate denial. Enter and save:

```text
# Human rules for the local demo
ALLOW read_file file:demo
DENY delete_file file:demo
REQUIRE_APPROVAL send_email recipient:demo
```

In **Check an intent**, leave the agent ID as `demo-agent`, choose `read_file`, and enter `file:demo`. Click **Issue test credential**, then **Check intent**. The server binds the new temporary credential to the enrolled ID; this is real Switchboard verification, not a fixture result. The credential clears from the form on submission. Issue another before each subsequent UI check; issuance replaces that agent's previous credential.

Try `delete_file` / `file:demo` for DENY and `send_email` / `recipient:demo` for REQUIRE_APPROVAL. A changed target such as `file:other` is denied by Switchboard because the grant does not cover it. The result shows the verified principal, decision reason, matched rule lines, policy version/hash, receipt ID, and recorded time. **Audit trail** reads saved server records; refresh it to inspect new activity.

The **Operator whitelist** link opens the retained registry UI at `/index.html`. Sign in on the auditor page first. Registry edits use this server's version-checked journal and become active for subsequent assessments. The older **Mint credential** utility only creates an unregistered random token; use the auditor's **Issue test credential** to bind a credential to an enrolled principal.

`npm run gate` (also `npm run operator`) uses `var/auditor/` and initializes from `config/agents.json`. That existing sample registry does not automatically acquire the demo grants. `APEX_DATA_DIR`, `APEX_REGISTRY_FILE`, and `PORT` select a separate data directory, initial registry, and local port. Use one server process for each data directory.

## Rule language

Each rule has exactly three space- or tab-separated fields:

```text
DECISION action_id exact_target
```

Decisions are uppercase `ALLOW`, `DENY`, or `REQUIRE_APPROVAL`. Actions are exactly `send_email`, `delete_file`, `execute_file`, `write_file`, or `read_file`. Targets are case-sensitive opaque identifiers, up to 256 characters, using letters, digits, `.`, `_`, `:`, `/`, and `-`, and beginning with a letter or digit. The gate does not resolve these names to actual resources.

Blank lines and whole-line `#` comments are allowed. Use LF or CRLF line endings. There are no inline comments, wildcards, aliases, shell expressions, or natural-language conditions. `read_only` is a planned permission preset, not an action. The complete text is validated before saving: malformed/duplicate rules, more than 200 rules, or more than 32 KiB block the save and identify a line where applicable.

For an action and target covered by several rules, precedence is **DENY → REQUIRE_APPROVAL → ALLOW**, independent of line order. All matching lines are returned. No match means DENY. Rules cannot override a Switchboard denial or error. Catalog membership is checked only after Switchboard PASS; an authenticated legacy grant for an action outside this catalog receives `UNKNOWN_ACTION` at the gate.

The editor shows unsaved changes. Assessments always use saved rules. A save includes the loaded policy version; a stale version receives HTTP 409. The UI preserves the unsaved text and requires a reload before another save. Invalid edits leave the active policy unchanged.

## Saved records and credentials

`audit.jsonl` in the selected data directory stores policy saves, registry saves, credential-issuance events, and assessments. Writes are serialized and synced before an assessment or state change is reported successful. Each assessment receipt refers to its saved event ID and timestamp. HTTP intake failures such as invalid JSON or an oversized body are transport errors and are not assessment records.

After initialization, the journal owns the active policy and registry. Startup restores them from its history; editing the original seed registry does not replace journal state. The live service applies each successful policy or registry save to subsequent assessments. Policy text hashes identify the exact saved text, and saves increment their respective version. The journal is limited to 16 MiB; full or failed storage blocks successful assessment responses. Incomplete records and invalid policy/registry history prevent normal startup.

Operator keys remain in the private local `operator.key` file. The browser uses an HttpOnly, SameSite cookie after sign-in. Sessions are replaced at server restart. Agent credentials live only in server memory, are replaced when reissued for a principal, and stop working when that server stops. Raw credential fields are excluded from assessment records; intent fields are metadata and should contain only agent/action/resource identifiers.

This is **one local server**, without multi-process journal locking, remote deployment authentication, tamper-proof evidence, or protection from an adversary with the same OS user's file access. UUIDs, timestamps, and hashes support inspection; they do not establish independent attestation. Repeated requests create separate assessments, not idempotent execution or single-use authorization.

## HTTP surface

All routes are on the local server. JSON requests are bounded to 64 KiB. Operator routes require the sign-in cookie or the operator key as a Bearer credential. Agent assessment uses the separate credential in its request body and needs no operator session.

| Route | Purpose |
| --- | --- |
| `POST /api/operator/session` with `{key}` | Establish the operator browser session |
| `GET /api/action-catalog` | Public read-only catalog; action IDs active, material-field definitions still draft |
| `GET /api/gate-rules` | Read `{version,text,hash}` as operator |
| `PUT /api/gate-rules` with `{version,text}` | Validate, persist, and activate rules as operator |
| `GET` / `PUT /api/registry` | Read or version-check/save the registry as operator; PUT body `{registry}` |
| `POST /api/gate/credentials` with `{agentId}` | Issue/replace a principal-bound temporary agent credential as operator |
| `POST /api/gate/evaluate` with `{credential,intent:{agentId,action,target}}` | Assess and persist agent intent |
| `GET /api/audit?limit=20` | Read recent records as operator, newest first; at most 100 |

Assessment results contain version `0.0.1`, status, code, reason, stage, the underlying Switchboard result, policy version/hash, matched rules, `execution: "NOT_EXECUTED"`, and the saved audit ID/time. Transport or storage errors are blocking failures; clients must not turn a missing/unreadable response into permission.

Source: [gate](../src/gate.js), [server](../src/server.js), [journal](../src/auditor-store.js), and [browser](../operator/gate.html). See the [build plan](../BUILD-PLAN.md) for the full material-intent and human-verification milestones still open. The pinned [APEX-Lite source](../reference/README.md) and its original evidence remain separate.
