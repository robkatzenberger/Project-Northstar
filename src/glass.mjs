/**
 * TL-PX / Glass core: evaluate declared intent against policy, emit decision receipt.
 * Reference implementation of Trust Layer Pre-Execution Minimum Standard 0.1.
 */

import { receiptId, nowIso, uuid } from "./ids.mjs";
import { evaluateRules } from "./policy.mjs";
import { appendAudit } from "./audit.mjs";
import {
  STANDARD_ID,
  STANDARD_VERSION,
  CONTROL_MODE,
  standardStamp
} from "./standard.mjs";
import { validateEvaluationRequest } from "./validate.mjs";
import {
  routeThroughSwitchboard,
  enrichIntentWithSwitchboard
} from "./switchboard.mjs";

export const GLASS_VERSION = "0.1.0";
export { CONTROL_MODE, STANDARD_ID, STANDARD_VERSION };

/**
 * @param {object} intent - evaluation intent (from toEvaluationIntent or raw)
 * @param {object} policy
 * @param {{ auditPath?: string, switchboard?: object }} [opts]
 */
export function evaluateIntent(intent, policy, opts = {}) {
  if (!intent?.intent_id && !intent?.prism_id) {
    throw new Error("evaluateIntent: intent_id or prism_id required");
  }

  let normalized = {
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

  /** @type {object|null} */
  let switchboard_context = null;

  // Switchboard: identity → whitelist → credibility → approval route
  if (opts.switchboard) {
    switchboard_context = routeThroughSwitchboard(opts.switchboard, normalized);
    normalized = enrichIntentWithSwitchboard(normalized, switchboard_context);
  }

  const reqCheck = validateEvaluationRequest(normalized);
  if (!reqCheck.ok) {
    throw new Error(`evaluateIntent: invalid request: ${reqCheck.errors.join("; ")}`);
  }

  let outcome;

  // Hard switchboard gate (unknown / not whitelisted / action not permitted)
  if (switchboard_context?.gate) {
    outcome = { ...switchboard_context.gate };
  } else {
    outcome = evaluateRules(normalized, policy);
  }

  const evaluatedAt = nowIso();
  const rid = receiptId(normalized.intent_id, new Date(evaluatedAt));

  let authorization_status;
  let blocking;
  let reward_signal;

  if (outcome.decision === "ALLOW") {
    authorization_status = "AUTHORIZED";
    blocking = false;
    reward_signal = "AUTO_ALLOW";
  } else if (outcome.decision === "DENY") {
    authorization_status = "DENIED";
    blocking = true;
    reward_signal = "SWITCHBOARD_DENIED";
  } else {
    authorization_status = "PENDING_HUMAN_APPROVAL";
    blocking = true;
    reward_signal = "TRANSPARENCY_REWARDED";
  }

  const decision = {
    record_type: "tlpx.decision",
    ...standardStamp(),
    glass_version: GLASS_VERSION,
    control_mode: CONTROL_MODE,
    blocking,
    receipt_id: rid,
    evaluated_at: evaluatedAt,
    decision: outcome.decision,
    reason: outcome.reason,
    policy_id: outcome.policy_id,
    policy_pack_id: policy.policy_pack_id || "default",
    reward_signal,
    parties: {
      declarer: {
        id: normalized.actor,
        type: normalized.actor_type,
        credibility: normalized.credibility ?? null,
        whitelisted: normalized.whitelisted ?? null
      },
      evaluator: {
        id: "tlpx-reference",
        type: "machine"
      },
      authorizer: null,
      // Switchboard as routing machine on the chain
      router: opts.switchboard
        ? {
            id: opts.switchboard.switchboard_id || "switchboard",
            type: "machine"
          }
        : null
    },
    authorization_status,
    // Who should handle escalation (from switchboard route)
    approval_route: switchboard_context?.approval_route || [],
    switchboard: switchboard_context
      ? {
          switchboard_id: switchboard_context.switchboard_id,
          lookup: switchboard_context.lookup,
          whitelisted: switchboard_context.whitelisted,
          credibility: switchboard_context.credibility,
          credibility_band: switchboard_context.credibility_band,
          flags: switchboard_context.flags,
          approval_route: switchboard_context.approval_route
        }
      : null,
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
  const isDecision =
    decision &&
    (decision.record_type === "tlpx.decision" || decision.record_type === "glass.decision");
  if (!isDecision) {
    throw new Error("resolveEscalation: tlpx.decision (or glass.decision) required");
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
    record_type: "tlpx.operator_action",
    ...standardStamp(),
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

  // Guardrail: cannot claim EXECUTED without authorization (includes switchboard DENY)
  if (status === "EXECUTED" && authStatus !== "AUTHORIZED") {
    throw new Error(
      `recordExecution: cannot EXECUTED when authorization_status=${authStatus}`
    );
  }
  if (status === "EXECUTED" && decision.decision === "DENY") {
    throw new Error("recordExecution: cannot EXECUTED after switchboard DENY");
  }

  const record = {
    record_type: "tlpx.execution",
    ...standardStamp(),
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
