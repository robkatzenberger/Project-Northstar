/**
 * Audit-chain authorization — single source of truth for air-gapped mode.
 *
 * Callers MUST NOT trust in-memory decision/operator objects for EXECUTED.
 * Authorization is derived only from the append-only audit log.
 */

import { readAudit } from "./audit.mjs";

export function isDecisionRecord(r) {
  return r?.record_type === "tlpx.decision" || r?.record_type === "glass.decision";
}

export function isOperatorRecord(r) {
  return (
    r?.record_type === "tlpx.operator_action" ||
    r?.record_type === "glass.operator_action"
  );
}

export function isExecutionRecord(r) {
  return r?.record_type === "tlpx.execution" || r?.record_type === "glass.execution";
}

export function findDecisionInRecords(records, receiptId) {
  return records.find((r) => isDecisionRecord(r) && r.receipt_id === receiptId) || null;
}

export function findOperatorActionsInRecords(records, receiptId) {
  return records.filter(
    (r) => isOperatorRecord(r) && (r.receipt_id === receiptId || r.linked_receipt_id === receiptId)
  );
}

export function findExecutionsInRecords(records, receiptId) {
  return records.filter(
    (r) => isExecutionRecord(r) && (r.receipt_id === receiptId || r.linked_receipt_id === receiptId)
  );
}

/**
 * Derive authorization solely from audit history for a receipt.
 *
 * @returns {{
 *   authorization_status: "AUTHORIZED"|"PENDING_HUMAN_APPROVAL"|"DENIED",
 *   decision: object,
 *   operator_action: object|null,
 *   source: string,
 *   terminal: boolean
 * }}
 */
export function resolveAuthorizationFromAudit(auditPath, receiptId) {
  if (!auditPath) {
    throw new Error("resolveAuthorizationFromAudit: auditPath required");
  }
  if (!receiptId) {
    throw new Error("resolveAuthorizationFromAudit: receiptId required");
  }

  const records = readAudit(auditPath);
  const decision = findDecisionInRecords(records, receiptId);
  if (!decision) {
    throw new Error(
      `resolveAuthorizationFromAudit: no decision in audit for receipt_id=${receiptId}`
    );
  }

  const operators = findOperatorActionsInRecords(records, receiptId);
  if (operators.length > 1) {
    throw new Error(
      `resolveAuthorizationFromAudit: corrupt audit — ${operators.length} operator actions for ${receiptId}`
    );
  }

  const operator_action = operators[0] || null;

  // Policy ALLOW — auto authorized, no human required
  if (decision.decision === "ALLOW") {
    if (operator_action) {
      throw new Error(
        `resolveAuthorizationFromAudit: operator action on ALLOW receipt is invalid (${receiptId})`
      );
    }
    return {
      authorization_status: "AUTHORIZED",
      decision,
      operator_action: null,
      source: "policy_allow",
      terminal: true
    };
  }

  // Switchboard / hard DENY
  if (decision.decision === "DENY") {
    if (operator_action) {
      throw new Error(
        `resolveAuthorizationFromAudit: operator action on DENY receipt is invalid (${receiptId})`
      );
    }
    return {
      authorization_status: "DENIED",
      decision,
      operator_action: null,
      source: "gate_deny",
      terminal: true
    };
  }

  // REQUIRE_APPROVAL — needs exactly zero or one operator action
  if (decision.decision === "REQUIRE_APPROVAL") {
    if (!operator_action) {
      return {
        authorization_status: "PENDING_HUMAN_APPROVAL",
        decision,
        operator_action: null,
        source: "awaiting_operator",
        terminal: false
      };
    }
    if (operator_action.outcome === "APPROVE") {
      return {
        authorization_status: "AUTHORIZED",
        decision,
        operator_action,
        source: "human_approve",
        terminal: true
      };
    }
    if (operator_action.outcome === "REJECT") {
      return {
        authorization_status: "DENIED",
        decision,
        operator_action,
        source: "human_reject",
        terminal: true
      };
    }
    throw new Error(
      `resolveAuthorizationFromAudit: unknown operator outcome ${operator_action.outcome}`
    );
  }

  throw new Error(
    `resolveAuthorizationFromAudit: unknown decision ${decision.decision}`
  );
}

/**
 * Assert receipt has not already been resolved by an operator (state machine).
 */
export function assertNotAlreadyResolved(auditPath, receiptId) {
  const records = readAudit(auditPath);
  const ops = findOperatorActionsInRecords(records, receiptId);
  if (ops.length > 0) {
    throw new Error(
      `receipt ${receiptId} already resolved by operator '${ops[0].operator?.id}' (${ops[0].outcome}) — single terminal outcome only`
    );
  }
}
