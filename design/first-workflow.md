# Apex's first workflow

**Status:** Full report-sharing workflow remains planned; v0.0.2 local assessment/review foundation updated 2026-09-10

**Foundation available:** [Auditor v0.0.2](../docs/auditor-v0.0.2.md) checks credentials, whitelist, and exact grants through [Switchboard](../docs/switchboard.md), then returns APPROVE or ESCALATE from human-entered rules. Its local operator resolves stored escalations with APPROVE or DENY and a context comment. The full report/digest/trusted-fact verification, reviewer routes, and expiry described below remain planned.

**Build sequence:** [Apex build plan](../BUILD-PLAN.md), with concrete completion checks for this workflow and a first increment covering full intent/result contracts.

## Scenario

An agent asks to share an identified report with an identified recipient. This is the synthetic example from the [Northstar milestone](../../docs/roadmap/transparency-layer.md). No real report is uploaded and no message is sent.

Use APEX-Lite's declaration, result, human queue, and receipt feed as the interaction starting point. The current auditor already links a three-field assessment to a separate human decision; extend that link to every material report and recipient field. The proposed `report.share` is not a current catalog action and cannot run through v0.0.2 without a separately defined contract.

## What the participant sees

| View | Questions it answers | Data ownership |
| --- | --- | --- |
| Declaration | Who proposes the action? What will be done, to which target, and why? | The request supplies action data and a separate explanation. Authenticated context establishes identity. |
| Authority check | Is this agent whitelisted? Which grant covers the action and recipient scope? Who issued that grant? | Registry/grant data controlled outside the proposing agent. |
| Constraint result | Which policy was applied? Which conditions passed, failed, or require verification? | Deterministic evaluation of validated action data and trusted context. |
| Human review | What needs verification? Which action and report digest is being reviewed? Who can decide, and what context supports APPROVE or DENY? | The system prepares the stored action display and authenticates the reviewer. Context is a human statement, not an automatic source of trusted facts. |
| Decision trail | Why did the request pass, stop, or escalate? What did the human decide? Has execution actually been observed? | Linked decision/operator evidence and, only when available, separate execution evidence. |

The first design must distinguish the report's declared description from its trusted classification, and the displayed actor name from authenticated identity. A declaration or readable receipt cannot grant the missing authority.

## Cases to walk through

| Case | Expected result |
| --- | --- |
| Authenticated agent outside the whitelist or granted scope | BLOCKED with the underlying Switchboard DENY and authority reason; no human-review bypass. |
| Identity cannot be authenticated | Blocking authentication failure; no authorization. |
| Agent has scope and applicable policy conditions are satisfied | APPROVE assessment linked to the exact evaluated action and policy. |
| Agent has scope but policy requires recipient verification, or no rule matches | ESCALATE with the actual reason and stored request; use the permitted review route for the full workflow. |
| Correct human approves the unchanged pending action | Record separate human APPROVE with required context and the assessment binding; no execution permission. |
| Human denies | Record separate human DENY with required context; preserve the original ESCALATE assessment. |
| Wrong reviewer, expired request, or changed recipient/report | Block approval. A material action change requires a new evaluation. |
| Material information or policy verification is unavailable | Blocking evaluation error; explain the missing condition without inventing an outcome. |

## Local v0.0.2 review boundary

The operator selects a saved current-model escalation. The backend binds the review to its assessment ID/hash and accepts only APPROVE or DENY with nonblank context of at most 2000 UTF-8 bytes after trimming. A repeated or conflicting decision returns 409. Before approval, policy and registry must still match, and the exact grant must remain available; otherwise obtain a new assessment. BLOCKED and ERROR never enter the review list.

One local operator role handles these decisions. Full reviewer routing, material report binding, trusted classification, and expiry are still design work. The context comment does not supply missing authority or prove the declared report contents. Every local record still says `NOT_EXECUTED`.

## Full-workflow design decisions

1. **Authority source:** define who enrolls an agent, grants its scope, and may revoke it. Pick an authentication mechanism separately from the displayed identity.
2. **Human route:** define who verifies the recipient, what trusted information they use, and what the approval display binds.
3. **Record mapping:** decide how the console projects existing versioned decision/operator records and labels any demonstration-only data.
4. **Prototype boundary:** choose fixture-backed presentation or a bounded connection to the existing authority. Keep that choice explicit before building a functioning approval path.

The walkthrough succeeds when a reviewer can reconstruct every case from the visible declaration, authority source, constraints, and decision trail. If an execution adapter is absent, the interface must show that no execution was observed. The first milestone does not depend on a new macOS administrator gate.
