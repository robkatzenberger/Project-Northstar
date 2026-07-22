# Trust Layer Pre-Execution Minimum Standard (TL-PX)

**Version:** 0.1.0  
**Status:** Draft for implementers  
**Profile:** Minimum  
**Date:** 2026-07-22  

This document defines a **minimum interoperable standard** for pre-execution trust checkpoints between intent and action. It is designed so that:

1. Independent implementations can **interoperate** on shared record shapes and decision semantics.
2. Real applications can **adopt a small, testable contract** without buying a full enterprise suite.
3. Open-source and commercial products can **extend** the standard without breaking the minimum profile.

**Not legal advice. Not a patent claim set.** Product and protocol language only.

---

## 0. Relationship to existing pieces

| Piece | Role relative to this standard |
| --- | --- |
| **Prism** | Optional open **intent signal** (metadata only). Implementations MAY accept Prism v0.1 and MUST map it into an Evaluation Request before judging. |
| **APEX-Lite / Trust-Engine** | Early reference implementations of the gate idea. This standard **formalizes and hardens** the minimum contract for production adopters. |
| **Glass** | Product/enterprise profile that MAY extend this minimum (tokens, multi-tenant, quorum, etc.). Extensions MUST NOT break Minimum Profile semantics when advertising TL-PX 0.1 conformance. |
| **This repository (Northstar)** | Reference implementation + **conformance suite** for TL-PX 0.1. |

---

## 1. Goals

### 1.1 What the standard guarantees

A conforming system provides a **checkpoint** such that:

1. **Intent is declared before execution** as structured data (not only free-form logs after the fact).
2. **Policy evaluation is deterministic** for a given Evaluation Request + Policy Snapshot (no generative model in the decision path).
3. **Decisions are recorded** as durable Decision Records with stable identifiers.
4. **Authorization state is explicit** (`AUTHORIZED` | `PENDING_HUMAN_APPROVAL` | `DENIED`).
5. **Execution is accountable**: attempts to act are linked to a Decision Record and MUST NOT claim success without `AUTHORIZED` (in conforming runtimes that honor the gate).
6. **Humans and machines are first-class parties** on the chain (declarer, evaluator, authorizer, executor), enabling post-incident review of **human error and machine error**.

### 1.2 What the standard does not guarantee

- That a malicious or non-conforming runtime cannot bypass the gate (enforcement is a deployment property; the standard defines the **contract** and **evidence**).
- Semantic truth of declared intent (the system trusts structured declaration; deep content inspection is out of Minimum Profile).
- Cryptographic non-repudiation (signed tokens are an **extension**).
- Ethical or legal liability assignment (records support **accountability evidence**, not automatic legal judgment).

---

## 2. Conformance keywords

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, **MAY** are to be interpreted as in RFC 2119.

An implementation that claims **"TL-PX 0.1 Minimum conforming"** MUST satisfy all MUST / MUST NOT requirements in this document and **MUST pass the conformance suite** published with the reference implementation (`npm run conformance`).

---

## 3. Architecture (normative separation)

A conforming deployment separates three concerns:

```text
(1) Intent declaration     → Evaluation Request
(2) Policy decision        → Decision Record (+ optional Operator Action)
(3) Execution attempt      → Execution Record
(4) Optional incident      → Accountability Report
```

**MUST:** Policy decision logic MUST NOT depend on generative model sampling for `ALLOW` / `REQUIRE_APPROVAL` outcomes.

**MUST NOT:** The intent signal layer MUST NOT be treated as a decision. Judgment lives only in the decision step.

**SHOULD:** Accept Prism v0.1 fields as an input dialect and map them into Evaluation Request fields.

---

## 4. Party model (human + machine)

Every durable record that participates in the trust chain MUST be attributable to parties with:

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | string | Stable identifier in the deploying system |
| `type` | `"human"` \| `"machine"` | Kind of principal |

### 4.1 Roles

| Role | Who | Required when |
| --- | --- | --- |
| **declarer** | Principal that stated intent | Always on Decision Record |
| **evaluator** | Policy engine identity (typically machine) | Always on Decision Record |
| **authorizer** | Human (or designated principal) who resolved escalation | When decision was `REQUIRE_APPROVAL` and resolved |
| **executor** | Principal that attempted the side effect | On Execution Record |

**MUST:** `declarer.type` and `executor.type` (when present) MUST be `human` or `machine`.  
**MUST:** Evaluator in Minimum Profile is recorded as `type: "machine"` (the policy engine).  
**MUST:** Operator / authorizer for escalations is recorded as `type: "human"` in Minimum Profile.

---

## 5. Decision model

### 5.1 Control mode

Minimum Profile control mode identifier:

```text
ALLOW_OR_ESCALATE
```

**MUST:** Implementations advertising TL-PX 0.1 Minimum MUST use this control mode for the core evaluate path.

### 5.2 Decisions

| Decision | Meaning |
| --- | --- |
| `ALLOW` | Policy does not require further human approval for this request. |
| `REQUIRE_APPROVAL` | Execution MUST NOT proceed until an Operator Action `APPROVE` is recorded (or the attempt is `BLOCKED`). |

**MUST NOT:** Minimum Profile MUST NOT require a generative model to choose between these values.

**MAY:** Future profiles add `DENY`. Minimum Profile does not define automatic hard-deny rules as mandatory (APEX-compatible escalate philosophy). Operator `REJECT` produces authorization `DENIED`.

### 5.3 Authorization status

| Status | Meaning |
| --- | --- |
| `AUTHORIZED` | Execution is permitted under this standard. |
| `PENDING_HUMAN_APPROVAL` | Waiting for Operator Action. |
| `DENIED` | Operator rejected (or future hard deny profiles). |

**Mapping (normative):**

- `ALLOW` → `authorization_status = AUTHORIZED`
- `REQUIRE_APPROVAL` (unresolved) → `PENDING_HUMAN_APPROVAL`
- Operator `APPROVE` → `AUTHORIZED`
- Operator `REJECT` → `DENIED`

---

## 6. Record types (Minimum Profile)

All records are JSON objects. Field names are **snake_case**.

### 6.1 Evaluation Request (input)

Logical fields used for policy evaluation (after Prism mapping if any):

| Field | Required | Description |
| --- | --- | --- |
| `intent_id` | MUST | Unique id for this intent instance |
| `actor` | MUST | Declarer id |
| `actor_type` | MUST | `human` \| `machine` |
| `declared_intent` | MUST | Human-readable summary of intended action |
| `action` | SHOULD | Machine-oriented action code |
| `target` | SHOULD | Target system/resource |
| `risk` | SHOULD | `low` \| `medium` \| `high` (default `low` if omitted) |
| `data_classes` | SHOULD | Array of strings (e.g. `PII`, `FINANCIAL`) |
| `timestamp` | SHOULD | ISO-8601 or deploy-defined time |
| `prism_id` | MAY | If emitted from Prism |
| `prism_version` | MAY | e.g. `prism_v0.1` |

### 6.2 Decision Record

`record_type`: **`tlpx.decision`** (alias accepted in this reference: `glass.decision`)

| Field | Required |
| --- | --- |
| `record_type` | MUST |
| `standard` | MUST = `TL-PX` |
| `standard_version` | MUST = `0.1.0` |
| `receipt_id` | MUST |
| `evaluated_at` | MUST (ISO-8601) |
| `control_mode` | MUST = `ALLOW_OR_ESCALATE` |
| `decision` | MUST = `ALLOW` \| `REQUIRE_APPROVAL` |
| `reason` | MUST |
| `policy_id` | MUST (string or `null` if no rule matched) |
| `authorization_status` | MUST |
| `parties.declarer` | MUST |
| `parties.evaluator` | MUST |
| `original_intent` | MUST (snapshot of evaluation fields) |

**SHOULD:** Include `reward_signal`: `TRANSPARENCY_REWARDED` when escalating, `AUTO_ALLOW` when allowing without escalation.

### 6.3 Operator Action Record

`record_type`: **`tlpx.operator_action`** (alias: `glass.operator_action`)

| Field | Required |
| --- | --- |
| `record_type` | MUST |
| `standard` / `standard_version` | MUST |
| `linked_receipt_id` or `receipt_id` | MUST |
| `acted_at` | MUST |
| `operator.id` / `operator.type` | MUST (`type` = `human`) |
| `outcome` | MUST = `APPROVE` \| `REJECT` |
| `authorization_status` | MUST = `AUTHORIZED` \| `DENIED` |

**MUST:** Only `REQUIRE_APPROVAL` decisions may be resolved by Operator Action in Minimum Profile.  
**SHOULD:** Reject double-resolution of the same receipt (single outcome).

### 6.4 Execution Record

`record_type`: **`tlpx.execution`** (alias: `glass.execution`)

| Field | Required |
| --- | --- |
| `record_type` | MUST |
| `standard` / `standard_version` | MUST |
| `linked_receipt_id` or `receipt_id` | MUST |
| `executed_at` | MUST |
| `status` | MUST = `EXECUTED` \| `BLOCKED` \| `FAILED` |
| `executor.id` / `executor.type` | MUST |
| `authorization_status` | MUST |

**MUST:** A conforming **executor adapter** MUST NOT emit `status: EXECUTED` unless `authorization_status` is `AUTHORIZED` for that receipt chain.

**MUST:** Execution records MUST link to the Decision Record via receipt id.

### 6.5 Accountability Report (optional but standardized)

`record_type`: **`tlpx.accountability_report`** (alias: `glass.accountability_report`)

Produced for incident review. **MUST** list:

- `parties_involved.human` (array of ids)
- `parties_involved.machine` (array of ids)
- `findings[]` with `code`, `party_type`, `party_role`, `severity`, `summary`

Findings are **evidence labels**, not legal judgments.

---

## 7. Policy (Minimum Profile)

### 7.1 Requirements

**MUST:** Policy evaluation be deterministic: same Evaluation Request + same Policy Snapshot ⇒ same `decision` and same matching `policy_id` (modulo non-semantic fields like timestamps and receipt ids).

**MUST:** Support at least:

- Equality on string fields (`risk == "high"`)
- Membership in arrays (`"PII" in data_classes`)
- Boolean `and` / `or` combination
- Ordered rules; **first match wins**
- No match ⇒ `ALLOW`

**MUST:** Escalation rules be expressible that set `REQUIRE_APPROVAL`.

**SHOULD:** Ship human-readable rule `id` and `description` (becomes decision `reason` / `policy_id`).

### 7.2 Non-requirements (extensions)

- Full programming languages for policy
- LLM judges
- Network calls during evaluate
- Multi-document policy federation

---

## 8. Audit log

**MUST:** Conforming systems provide an append-only sequence of Decision, Operator Action, and Execution records for a deployment-defined retention period.

**SHOULD:** Use JSON Lines (JSONL) for local/simple deployments (one JSON object per line).

**MUST:** Support retrieval of the **chain** for a given `receipt_id` (all records linked to that decision).

**MUST NOT:** Rewrite or delete historical decision lines in the name of "correction" without a separate compensating record (Minimum Profile: append-only).

---

## 9. Prism interoperability

**MAY:** Accept Prism v0.1:

```json
{
  "prism_id": "uuid",
  "timestamp": "ISO-8601",
  "agent": "string",
  "intent_summary": "string",
  "prism_version": "prism_v0.1"
}
```

**MUST:** When Prism is the only input, map at least:

- `prism_id` → `intent_id` / `prism_id`
- `agent` → `actor`
- `intent_summary` → `declared_intent`

**SHOULD:** Carry additional evaluation fields (`action`, `risk`, `data_classes`, `actor_type`) in a side channel or extension object without claiming they are Prism core fields.

---

## 10. Runtime integration contract

For real-world applications, a conforming **integration** SHOULD implement:

```text
before side_effect(action):
  request  = build_evaluation_request(action)
  decision = gate.evaluate(request)
  if decision.authorization_status != AUTHORIZED:
    if decision.decision == REQUIRE_APPROVAL:
      wait_or_block_for_operator(decision.receipt_id)
    else:
      abort
  result = perform(action)
  gate.record_execution(decision, result)
```

**MUST (conformance of executor adapter):** Refuse `EXECUTED` without `AUTHORIZED`.

---

## 11. Versioning and extensions

- Standard id: **`TL-PX`**
- This document: **`0.1.0`**
- Breaking changes to required fields or decision enums require a new minor/major version per SemVer for the standard_version field.
- Extensions MUST use additional fields or new `record_type` values namespaced by the vendor (e.g. `glass.*` product fields) without removing required TL-PX fields when claiming conformance.

---

## 12. Conformance suite

The reference repository provides:

```bash
npm run conformance
```

Categories:

1. **Decision determinism** — fixtures → expected decision + policy_id  
2. **Authorization mapping** — ALLOW/REQUIRE_APPROVAL/operator outcomes  
3. **Execution guard** — unauthorized EXECUTED rejected  
4. **Party model** — human/machine roles present  
5. **Chain integrity** — receipt links decision → operator → execution  
6. **Accountability** — incident report includes both party kinds when both participated  

An alternate implementation may plug into the suite by exporting the same library surface or by emitting JSON fixtures; the reference suite targets the JS API in this repo.

---

## 13. Security considerations (informative)

- Declared intent can be wrong or incomplete; pair with operational controls.
- Expression evaluators must be sandboxed in production (avoid unsafe `eval` of untrusted policy in multi-tenant hosts).
- Protect audit storage integrity and access.
- Operator identity MUST be authenticated in real deployments (Minimum Profile records the id; authN mechanism is deployment-specific).

---

## 14. License posture (informative)

The **standard text** and **reference implementation** are intended for open-source release under a permissive license (Apache-2.0 recommended, matching Prism / APEX-Lite). Enterprise profiles and proprietary extensions MAY remain separate.

Until a public release is authorized by the project owner, this tree remains local-only.

---

## 15. Change log

| Version | Notes |
| --- | --- |
| 0.1.0 | First Minimum Profile: parties, decisions, records, policy subset, audit, execution guard, accountability report shape. |
