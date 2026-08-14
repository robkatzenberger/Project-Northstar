/**
 * TL-PX / Glass core — air-gapped by default.
 *
 * Durable operations require opts.auditPath (append-only log).
 * Authorization for execution is derived from the audit chain, not caller-supplied status fields.
 */

import { receiptId, nowIso, uuid } from "./ids.mjs";
import { compilePolicy, evaluateRules } from "./policy.mjs";
import { appendAudit, readAudit } from "./audit.mjs";
import {
  CONTROL_MODE,
  STANDARD_ID,
  STANDARD_VERSION,
  standardStamp
} from "./standard.mjs";
import { validateEvaluationRequest } from "./validate.mjs";
import {
  routeThroughSwitchboard,
  enrichIntentWithSwitchboard
} from "./switchboard.mjs";
import {
  assertNotAlreadyResolved,
  findDecisionInRecords,
  resolveAuthorizationFromAudit
} from "./chain.mjs";
import { assertOperatorAllowed } from "./operators.mjs";

export const GLASS_VERSION = "0.1.0";
export { CONTROL_MODE, STANDARD_ID, STANDARD_VERSION, resolveAuthorizationFromAudit };

/**
 * Air-gapped mode requires an audit path unless explicitly marked ephemeral
 * (unit tests of pure decision logic only — never for authorize/execute).
 */
function requireAuditPath(opts, fnName) {
  if (opts?.allowEphemeral) return;
  if (!opts?.auditPath) {
    throw new Error(
      `${fnName}: auditPath required for air-gapped operation. ` +
        `Pass { auditPath: "…/audit.jsonl" }. ` +
        `For non-durable unit tests only: { allowEphemeral: true }.`
    );
  }
}

function persist(opts, record) {
  if (opts.auditPath) {
    appendAudit(opts.auditPath, record);
  }
  return record;
}

/**
 * @param {object} intent
 * @param {object} policy
 * @param {{ auditPath?: string, switchboard?: object, allowEphemeral?: boolean }} [opts]
 */
export function evaluateIntent(intent, policy, opts = {}) {
  requireAuditPath(opts, "evaluateIntent");
  const compiledPolicy = compilePolicy(policy);

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

  if (opts.switchboard) {
    switchboard_context = routeThroughSwitchboard(opts.switchboard, normalized);
    normalized = enrichIntentWithSwitchboard(normalized, switchboard_context);
  }

  const reqCheck = validateEvaluationRequest(normalized);
  if (!reqCheck.ok) {
    throw new Error(`evaluateIntent: invalid request: ${reqCheck.errors.join("; ")}`);
  }

  let outcome;
  if (switchboard_context?.gate) {
    outcome = { ...switchboard_context.gate };
  } else {
    outcome = evaluateRules(normalized, compiledPolicy);
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
    policy_pack_id: compiledPolicy.policy_pack_id || "default",
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
      router: opts.switchboard
        ? {
            id: opts.switchboard.switchboard_id || "switchboard",
            type: "machine"
          }
        : null
    },
    authorization_status,
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

  return persist(opts, decision);
}

/**
 * Resolve REQUIRE_APPROVAL using audit as source of truth.
 * Single terminal operator outcome per receipt (state machine).
 *
 * @param {object} input - { receipt_id } or decision object with receipt_id
 * @param {{ operator_id: string, outcome: "APPROVE"|"REJECT", note?: string }} action
 * @param {{ auditPath: string, allowEphemeral?: boolean }} opts
 */
export function resolveEscalation(input, action = {}, opts = {}) {
  requireAuditPath(opts, "resolveEscalation");

  const operator_id = action.operator_id;
  const outcome = action.outcome;
  const note = action.note;

  if (!operator_id) throw new Error("resolveEscalation: operator_id required");
  if (outcome !== "APPROVE" && outcome !== "REJECT") {
    throw new Error('resolveEscalation: outcome must be "APPROVE" or "REJECT"');
  }

  const receiptId = input?.receipt_id || input?.linked_receipt_id;
  if (!receiptId) {
    throw new Error("resolveEscalation: receipt_id required (pass decision or { receipt_id })");
  }

  const records = readAudit(opts.auditPath);
  const decision = findDecisionInRecords(records, receiptId);
  if (!decision) {
    throw new Error(`resolveEscalation: no decision in audit for receipt_id=${receiptId}`);
  }

  if (decision.decision !== "REQUIRE_APPROVAL") {
    throw new Error(
      `resolveEscalation: only REQUIRE_APPROVAL can be resolved (got ${decision.decision})`
    );
  }

  assertOperatorAllowed(operator_id, decision, opts);
  assertNotAlreadyResolved(opts.auditPath, receiptId);

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

  return persist(opts, record);
}

/**
 * Record execution. Authorization is ALWAYS re-derived from audit.
 * Caller-supplied decision.authorization_status is IGNORED.
 */
export function recordExecution(input = {}, opts = {}) {
  requireAuditPath(opts, "recordExecution");

  const executor_id = input.executor_id;
  const executor_type = input.executor_type || "machine";
  const status = input.status;
  const result_summary = input.result_summary ?? null;
  const error = input.error ?? null;

  if (!executor_id) throw new Error("recordExecution: executor_id required");
  if (!["EXECUTED", "BLOCKED", "FAILED"].includes(status)) {
    throw new Error("recordExecution: invalid status");
  }

  const receiptId = input.receipt_id || input.decision?.receipt_id;
  if (!receiptId) {
    throw new Error("recordExecution: receipt_id required");
  }

  const auth = resolveAuthorizationFromAudit(opts.auditPath, receiptId);
  const decision = auth.decision;
  const operator_action = auth.operator_action;
  const authStatus = auth.authorization_status;

  if (status === "EXECUTED" && authStatus !== "AUTHORIZED") {
    throw new Error(
      `recordExecution: cannot EXECUTED when audit authorization_status=${authStatus} (source=${auth.source})`
    );
  }
  if (status === "EXECUTED" && decision.decision === "DENY") {
    throw new Error("recordExecution: cannot EXECUTED after DENY");
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
    authorization_source: auth.source,
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

  return persist(opts, record);
}
