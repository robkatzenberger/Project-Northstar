/**
 * Glass core: evaluate declared intent against policy, emit decision receipt.
 */

import { receiptId, nowIso, uuid } from "./ids.mjs";
import { evaluateRules } from "./policy.mjs";
import { appendAudit } from "./audit.mjs";

export const GLASS_VERSION = "0.1.0";
export const CONTROL_MODE = "ALLOW_OR_ESCALATE";

/**
 * @param {object} intent - evaluation intent (from toEvaluationIntent or raw)
 * @param {object} policy
 * @param {{ auditPath?: string }} [opts]
 */
export function evaluateIntent(intent, policy, opts = {}) {
  if (!intent?.intent_id && !intent?.prism_id) {
    throw new Error("evaluateIntent: intent_id or prism_id required");
  }

  const normalized = {
    intent_id: intent.intent_id || intent.prism_id,
    prism_id: intent.prism_id || intent.intent_id,
    prism_version: intent.prism_version || null,
    actor: intent.actor,
    actor_type: intent.actor_type || "machine",
    declared_intent: intent.declared_intent || intent.intent_summary,
    intent_summary: intent.intent_summary || intent.declared_intent,
    action: intent.action ?? null,
    target: intent.target ?? null,
    risk: intent.risk || "low",
    data_classes: intent.data_classes || [],
    context: intent.context || {},
    timestamp: intent.timestamp || nowIso()
  };

  const outcome = evaluateRules(normalized, policy);
  const evaluatedAt = nowIso();
  const rid = receiptId(normalized.intent_id, new Date(evaluatedAt));

  const decision = {
    record_type: "glass.decision",
    glass_version: GLASS_VERSION,
    control_mode: CONTROL_MODE,
    blocking: outcome.decision === "REQUIRE_APPROVAL",
    receipt_id: rid,
    evaluated_at: evaluatedAt,
    decision: outcome.decision,
    reason: outcome.reason,
    policy_id: outcome.policy_id,
    policy_pack_id: policy.policy_pack_id || "default",
    reward_signal: outcome.decision === "REQUIRE_APPROVAL" ? "TRANSPARENCY_REWARDED" : "AUTO_ALLOW",
    // Accountability anchors
    parties: {
      declarer: {
        id: normalized.actor,
        type: normalized.actor_type
      },
      evaluator: {
        id: "glass",
        type: "machine"
      },
      authorizer: null
    },
    authorization_status:
      outcome.decision === "ALLOW" ? "AUTHORIZED" : "PENDING_HUMAN_APPROVAL",
    original_intent: normalized
  };

  if (opts.auditPath) {
    appendAudit(opts.auditPath, decision);
  }

  return decision;
}

/**
 * Human (or designated principal) resolves a REQUIRE_APPROVAL decision.
 * @param {"APPROVE"|"REJECT"} outcome
 */
export function resolveEscalation(decision, { operator_id, outcome, note } = {}, opts = {}) {
  if (!decision || decision.record_type !== "glass.decision") {
    throw new Error("resolveEscalation: glass.decision required");
  }
  if (decision.decision !== "REQUIRE_APPROVAL") {
    throw new Error("resolveEscalation: only REQUIRE_APPROVAL decisions can be resolved");
  }
  if (!operator_id) {
    throw new Error("resolveEscalation: operator_id required");
  }
  if (outcome !== "APPROVE" && outcome !== "REJECT") {
    throw new Error('resolveEscalation: outcome must be "APPROVE" or "REJECT"');
  }

  const record = {
    record_type: "glass.operator_action",
    glass_version: GLASS_VERSION,
    action_id: uuid(),
    linked_receipt_id: decision.receipt_id,
    receipt_id: decision.receipt_id,
    acted_at: nowIso(),
    operator: {
      id: operator_id,
      type: "human"
    },
    outcome,
    note: note || null,
    original_intent: decision.original_intent,
    // Dual accountability: human now on the authorization chain
    parties: {
      declarer: decision.parties.declarer,
      evaluator: decision.parties.evaluator,
      authorizer: {
        id: operator_id,
        type: "human",
        outcome
      }
    },
    authorization_status: outcome === "APPROVE" ? "AUTHORIZED" : "DENIED"
  };

  if (opts.auditPath) {
    appendAudit(opts.auditPath, record);
  }

  return record;
}

/**
 * Record that an action was executed (or blocked) after the trust checkpoint.
 * This is the machine/human execution side of the accountability chain.
 */
export function recordExecution(
  {
    decision,
    operator_action = null,
    executor_id,
    executor_type = "machine",
    status, // "EXECUTED" | "BLOCKED" | "FAILED"
    result_summary = null,
    error = null
  },
  opts = {}
) {
  if (!decision) throw new Error("recordExecution: decision required");
  if (!executor_id) throw new Error("recordExecution: executor_id required");
  if (!["EXECUTED", "BLOCKED", "FAILED"].includes(status)) {
    throw new Error("recordExecution: invalid status");
  }

  const authStatus = operator_action
    ? operator_action.authorization_status
    : decision.authorization_status;

  // Guardrail: cannot claim EXECUTED without authorization
  if (status === "EXECUTED" && authStatus !== "AUTHORIZED") {
    throw new Error(
      `recordExecution: cannot EXECUTED when authorization_status=${authStatus}`
    );
  }

  const record = {
    record_type: "glass.execution",
    glass_version: GLASS_VERSION,
    execution_id: uuid(),
    linked_receipt_id: decision.receipt_id,
    receipt_id: decision.receipt_id,
    executed_at: nowIso(),
    status,
    result_summary,
    error,
    executor: {
      id: executor_id,
      type: executor_type
    },
    authorization_status: authStatus,
    decision_snapshot: {
      decision: decision.decision,
      reason: decision.reason,
      policy_id: decision.policy_id
    },
    parties: {
      declarer: decision.parties.declarer,
      evaluator: decision.parties.evaluator,
      authorizer: operator_action?.parties?.authorizer || decision.parties.authorizer,
      executor: {
        id: executor_id,
        type: executor_type
      }
    },
    original_intent: decision.original_intent
  };

  if (opts.auditPath) {
    appendAudit(opts.auditPath, record);
  }

  return record;
}
