# Apex

**Northstar's third-party transparency layer, developed from APEX-Lite.**

**Stage:** Apex Auditor **v0.0.2** provides human-entered deterministic rules, authenticated Switchboard screening, and saved assessments. A participating agent submits intent through the client using its configured instructions; the human manages rules, reviews, and the audit trail. The decision gate returns **APPROVE** or **ESCALATE**. An authenticated operator resolves a saved escalation with **APPROVE** or **DENY** and required context. Full action validation, multiple reviewer roles, and execution remain planned. The pinned APEX-Lite source is preserved separately.

An agent arrives with its declared intent. A deterministic system checks:

1. Does the agent have authority for the intended action?
2. What boundaries or constraints apply, and which require human escalation and verification?

The human interface shows the declared action, verified identity, applicable rules, assessment reason, saved receipt, and separate human outcome. The agent can retrieve its own saved status and human context. This is one local auditor with one operator role and cooperative clients. Full material-action verification and hosting beyond that boundary remain future work.

## Try the auditor

From this folder, using Node 22 or newer:

```sh
npm test
npm run demo:auditor
```

Open the local address printed at startup, normally `http://127.0.0.1:43129`. The server prints the path to `operator.key`, **not the key**; open that local file and use its value in Auditor sign-in. The demo keeps its registry, rules, and records under `var/auditor-demo/`.

Enter and save rules such as:

```text
APPROVE read_file file:demo
ESCALATE delete_file file:demo
ESCALATE send_email recipient:demo
```

Use the connection area to provision an enrolled agent's credential into a private file, then give the agent the [participant instructions](docs/agent-instructions.md), client path, and connection environment. Follow the [agent connection guide](docs/agent-integration.md). The agent submits its exact intent automatically as part of its instructed workflow; the human does not type each intent into the auditor.

An eligible request with no matching rule escalates. The agent pauses that action and checks its own saved status. The human opens the saved review, chooses APPROVE or DENY, and enters context; that context returns to the agent on status lookup. Review and audit lists refresh every three seconds while the signed-in page is visible. Authority failures return BLOCKED and invalid evaluations return ERROR; neither enters human review. A collapsed **Developer test console** retains manual assessment tools. Every assessment and human decision records `NOT_EXECUTED`. See the [v0.0.2 guide](docs/auditor-v0.0.2.md) for the API and persistence boundary.

The instructions and client provide cooperative integration. Markdown does not intercept tools or force an agent through the gate. Neither machine nor human approval supplies execution permission.

The smaller Switchboard-only demonstration remains available with `npm run demo:switchboard`. Its PASS result permits progression to the gate.

## Start here

| File | Purpose |
| --- | --- |
| [Auditor v0.0.2](docs/auditor-v0.0.2.md) | Run the auditor, enter APPROVE/ESCALATE rules, review escalations, and inspect records |
| [Agent connection guide](docs/agent-integration.md) and [participant instructions](docs/agent-instructions.md) | Configure an agent to submit its own intent and observe its saved review |
| [Agent-workflow verification](test/reports/agent-workflow-2026-09-10/REPORT.md) | 116 tests, automatic dashboard arrival, and one actual AI participant using the instruction template and CLI |
| [v0.0.2 verification](test/reports/auditor-v0.0.2-2026-09-10/REPORT.md) | Recorded human-review tests, browser evidence, source hashes, and migration check; later client changes need their own verification |
| [Historical v0.0.1 guide](docs/auditor-v0.0.1.md) and [verification](test/reports/auditor-v0.0.1-2026-09-10/REPORT.md) | Preserved prior behavior and evidence; these results do not verify v0.0.2 |
| [Apex build plan](BUILD-PLAN.md) | Build sequence from APEX-Lite, milestone completion checks, and the next implementation increment |
| [Gate Actions / Grok UI handoff](docs/gate-language.md), [action catalog](config/action-catalog.json) | Enforced action IDs and exact rules; later material-field contract |
| [Switchboard guide](docs/switchboard.md) | Registry format, credential boundary, results, and access changes |
| [Ten-agent stress test](docs/stress-test.md) | Increasing-load experiment with separate gate and agent records |
| [Actual AI-agent test](docs/ai-agent-test.md) | AI participants declare intent, invoke one gate, and report observed decisions |
| [Switchboard source](src/switchboard.js), [sample registry](config/agents.json), [tests](test/switchboard.test.js) | Implemented whitelist foundation |
| [Foundation design](design/0001-foundation.md) | Proposed architecture and the changes Apex needs beyond APEX-Lite |
| [First workflow](design/first-workflow.md) | Initial participant experience, cases, and next design decisions |
| [APEX-Lite reference](reference/README.md) | Exact source provenance, code map, and historical behavior |
| [Source snapshot](reference/apex-lite/) | Original evaluator, console, examples, configuration, and tests |
| [Working instructions](AGENTS.md) | How to continue work in this folder |

The [Northstar roadmap](../docs/roadmap/transparency-layer.md) sets product priority. Apex owns the working design for that priority. Existing Northstar code and evidence stay in place; macOS enforcement continuation remains on the back burner.

## What we are carrying forward

APEX-Lite provides a small, readable starting point: an intent form, deterministic policy examples, visible decision reasons, a human queue, and a receipt history. Its original files are preserved under `reference/apex-lite/` so design work can cite and compare actual behavior.

Switchboard provides credential-based agent screening, whitelist and exact action-target checks, and blocking failure results. The v0.0.2 gate records each assessment before returning it. Its separate human review is bound to the stored assessment and requires operator authentication, a context comment, and unchanged policy/authority for approval. Typed material constraints, per-condition reviewer routes, expiry, and approvals bound to a complete action remain planned. None of these guarantees comes from copying APEX-Lite: its original `ALLOW_OR_ESCALATE` receipts explicitly have `blocking: false` and are not execution permission.

## Complete workflow still to build

The [build plan](BUILD-PLAN.md) still calls for the complete report-sharing workflow: full material intent, trusted facts, reviewer routes, expiry, and end-to-end evidence across actual clients. The local v0.0.2 review and cooperative client are bounded foundations for that work and do not complete those milestones.

Keep the intent declaration, system verification results, human approval, authorization, and observed execution distinct. The first walkthrough does not send messages, operate protected resources, or require macOS administrator setup.

Original APEX-Lite files retain their [Apache-2.0 license](reference/apex-lite/LICENSE) and attribution. Northstar's [repository license](../LICENSE) remains in place.
