# Gate Actions — contract for Grok's UI

**Status:** action IDs and exact scope rules are implemented in [Apex Auditor v0.0.2](auditor-v0.0.2.md). The [catalog](../config/action-catalog.json) is the shared machine-readable vocabulary, served by `GET /api/action-catalog`. Its `materialFields`, permission presets, and full action definitions remain design metadata; v0.0.2 does not validate or execute a complete action.

Give the operator a **Gate Actions** section alongside the agent whitelist. This is where Apex defines the exact actions the gate understands. Use those definitions to populate the **Action** selector when configuring a grant or declaring intent. The agent's natural-language explanation is a separate descriptive field.

An action catalog supplies a controlled vocabulary. An agent grant supplies authority for an exact action and target. Human-entered policy supplies the gate's deterministic assessment. The auditor owns that policy separately from the agent's declaration.

## Human rule entry in v0.0.2

The auditor page provides a text area for exactly one rule per line:

```text
APPROVE read_file file:demo
ESCALATE delete_file file:demo
ESCALATE send_email recipient:demo
```

Each rule is an uppercase decision, a catalog action ID, and one exact opaque target. Blank lines and whole-line `#` comments are allowed. No wildcards, prose, aliases, or executable expressions are interpreted. Save validates the complete text; invalid input blocks the save. Among matching rules, ESCALATE wins over APPROVE. No matching rule escalates an eligible request to a human. The gate has no DENY rule; DENY is an authenticated human outcome on a stored escalation, with required context.

The server runs Switchboard first. A rule cannot override identity, whitelist, or grant failure. After Switchboard PASS, the gate rejects actions outside this catalog and evaluates the saved rules. Its APPROVE means only that authenticated action-and-target scope satisfies a rule. Top-level BLOCKED preserves Switchboard authority denial; ERROR remains a nonreviewable evaluation failure. These are outside the gate's two business outcomes. Policy saves and assessments are journaled by the auditor; the browser displays the actual returned record. See the [run guide](auditor-v0.0.2.md) for syntax limits and API behavior.

## Initial vocabulary

| Stable action ID | UI label | Full-action definition planned beyond v0.0.2 |
| --- | --- | --- |
| `send_email` | Send email | One identified message, sender account, attachment set, and exact recipient |
| `delete_file` | Delete file | One identified file; explicit trash or permanent deletion; no recursive deletion |
| `execute_file` | Execute file | One identified executable, arguments, working directory, and trusted execution profile |
| `write_file` | Write file | One file operation with explicit create/replace/append mode, content, and precondition |
| `read_file` | Read file | Read the identified file's contents; no mutation, execution, or forwarding authority |

Use **Read only** (`read_only`) as a permission preset that prepares `read_file` grants for selected exact targets. It is not an action or a universal mode. Adding read grants does not remove write/execute grants that an agent already has. Before labeling the agent “read only,” show and resolve any other applicable grants explicitly; never silently delete them.

Use `write_file` for the file operation. A bare `write` could otherwise mean a file, database, network socket, or another resource. UI search may suggest `write_file` when someone types `write`, but saving requires an explicit selection. Do not silently rewrite submitted action strings or treat similar names as equivalent.

## Grok's UI handoff

| Area | Required controls and behavior |
| --- | --- |
| Gate Actions catalog | Show stable ID, readable label, meaning, target kind, required material fields, and catalog version. New/custom definitions are drafts until backend validation supports them. |
| Agent's grant editor | Choose an action, enter/select one exact target, identify the grant source, and show the proposed grant before save. No action or target is selected by default. |
| Read-only preset | Let the operator select exact file targets and preview separate `read_file` grants. Surface conflicting existing grants. |
| Intent declaration | Choose the exact action and target; show its material fields; keep the agent's explanation separate. Render all submitted text as inert data. |
| Constraint settings | The standalone auditor supports the exact rule syntax above. Later typed conditions and permitted human routes remain A2 work. Do not present prose as an enforced rule or infer risk from an action label. |
| Result | Show gate APPROVE/ESCALATE, authority BLOCKED, and evaluation ERROR distinctly. Keep a later human APPROVE/DENY separate from the immutable assessment. Unknown action IDs return ERROR after Switchboard. Neither approval type means execution. Every evaluate completion (including ERROR/BLOCKED) must show its journal receipt id for audit/debug; ERROR/BLOCKED are not human-reviewable. |
| Human review | Load stored escalations from `GET /api/reviews`. Display the stored request and assessment; submit only `{assessmentId,decision,comment}` to `POST /api/reviews/decision`. Decisions are APPROVE or DENY, with a required context comment of at most 2000 UTF-8 bytes after trimming. Show 409 conflicts/stale approval without inventing success. |

Grok can use `GET /api/action-catalog` from the same server. It returns this catalog directly, with `runtimeEnforcement: "action_ids_and_exact_scope_rules"` and `versionLabel: "0.0.2"`. Material-field controls and presets remain future UI/full-action work. Do not duplicate the catalog into an independently editable policy source. Existing Grok registry files are preserved; the auditor page lives in `operator/gate.html`, `gate.js`, and `gate.css`.

The backend owns operator authentication, rule evaluation, and persistence. It binds human decisions to the stored assessment ID/hash, prevents a second resolution, and rechecks policy and registry before approval. Show the original assessment and the human event together; a context comment does not change the declared action or create authority. UI additions should call the backend and display its actual results.

## Current Switchboard integration

The implemented [Switchboard](../src/switchboard.js) takes exactly three intent fields:

```json
{
  "agentId": "agent.example",
  "action": "send_email",
  "target": "recipient:demo"
}
```

The transport supplies a credential separately. The trusted registry would need an exact matching grant, such as:

```json
{
  "id": "grant.example-email",
  "action": "send_email",
  "target": "recipient:demo",
  "issuedBy": "operator:example-owner"
}
```

These are illustrative scope records, not newly installed grants. A PASS for this projection does not validate the sender, message, attachments, recipient facts, or human conditions. Those belong to the full-action contract and constraint screening. **Do not add material fields or `catalogVersion` directly inside the current three-field Switchboard intent**; it rejects extra fields.

Action IDs are exact and case-sensitive. `send_email`, `send-email`, and `Send_Email` are different strings. The catalog is data, never executable code, a shell expression, or an instruction for the gate to interpret with an LLM. The standalone Switchboard continues to accept any syntactically valid action with a matching grant; the connected gate adds the catalog check after Switchboard PASS.

Current targets are opaque exact identifiers such as `file:demo` and `recipient:demo`. They are not raw email addresses or filesystem paths, and they support no wildcard or prefix permission. A later trusted resource resolver must bind the identifier to the actual resource without changing the approved action's meaning.

Preserve every existing action string and grant during UI integration. Existing `report.read` and `switchboard.ping` records keep their identities. The build plan's proposed `report.share` is also a distinct action, not an alias for `send_email`. Display saved actions outside this catalog as existing identifiers; never silently rename, remove, or mark them as catalog-validated. They remain usable by the standalone Switchboard, while the v0.0.2 auditor blocks unsupported action IDs.

## Full-action field contract

`materialFields` describes what the UI should collect or display for the later A1 full-intent contract. It is field metadata, **not a completed validation schema**. Fields marked required have no implied default. Referenced payloads remain outside metadata-only intent/audit records.

| Field type in catalog | Intended representation |
| --- | --- |
| `opaque_reference` | Nonempty bounded identifier resolved by a trusted adapter; no authority inferred from the name |
| `opaque_version` | Nonempty resource/profile version, later verified against trusted state |
| `sha256_digest` | `sha256:` followed by 64 lowercase hexadecimal characters; unverified until matched against the referenced bytes |
| `enum` | Exactly one of the listed strings; no automatic choice |
| `reference_digest_list` | Explicit array of objects containing `ref` and `digest`; empty array means no attachments |
| `string_list` | Explicit argument array; empty array means no arguments; no shell interpolation |
| `file_precondition` | For create: `{ "kind": "absent" }`; for replace/append: `{ "kind": "matches", "version": "…", "digest": "sha256:…" }` |

Before these details influence permission, A1 must define bounded lengths/counts, nested schemas, canonical action binding, payload-digest semantics, and cross-field checks. A2 must verify the declared facts against trusted context. Changed message bytes, attachments, sender/recipient, file version, write mode, executable, arguments, working directory, or execution profile require a different bound evaluation. An approval of `execute_file` alone cannot establish control over all effects of the program it invokes.

For `send_email`, the target must match the actual delivery-envelope recipient. The trusted adapter must validate all recipient-bearing fields and reject extra or implicit recipients; checking only the displayed recipient field is insufficient for this contract.

## Full-action work still to build

1. Add the catalog-aware intake and full-action schema under A1. Validate the catalog/version and each action definition on the trusted side; reject unsupported action/version and invalid material input. Preserve the current Switchboard API through a strict projection. Document any legacy-action migration explicitly.
2. Under A2, implement typed constraints and permitted reviewer routes. Keep authority failures and missing/invalid material facts blocking and nonreviewable. For valid, authorized actions, policy either approves or escalates with its condition; a human can record APPROVE or DENY with context. Missing material facts cannot be silently replaced by an agent's explanation or a generic approval checkbox.
3. Extend the existing read-only catalog endpoint deliberately when full-action definitions become enforceable. Restrict any future action-definition mutation to the policy owner; definition versions must be reflected in full-action evaluation records and cannot silently reinterpret existing grants or pending reviews.

The v0.0.2 implementation is a bounded action-and-target foundation. It does not complete [A1–A6 in the build plan](../BUILD-PLAN.md).
