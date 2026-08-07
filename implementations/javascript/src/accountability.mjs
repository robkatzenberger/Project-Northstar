/**
 * Accountability: reconstruct the trust chain and attribute failure surfaces
 * to human and/or machine parties.
 *
 * This is not legal liability assignment — it is an inspectable evidence graph
 * for post-incident review.
 */

import { chainForReceipt, readAudit, appendAudit } from "./audit.mjs";
import { nowIso, uuid } from "./ids.mjs";
import { standardStamp } from "./standard.mjs";

/**
 * Build a chronological chain for a receipt.
 */
function typeRank(t) {
  const order = {
    "tlpx.decision": 1,
    "glass.decision": 1,
    "tlpx.operator_action": 2,
    "glass.operator_action": 2,
    "tlpx.execution": 3,
    "glass.execution": 3,
    "tlpx.incident": 4,
    "glass.incident": 4,
    "tlpx.accountability_report": 5,
    "glass.accountability_report": 5
  };
  return order[t] || 9;
}

export function buildChain(auditRecords, receiptId) {
  const chain = chainForReceipt(auditRecords, receiptId);
  return [...chain].sort((a, b) => {
    const ta = a.evaluated_at || a.acted_at || a.executed_at || a.recorded_at || "";
    const tb = b.evaluated_at || b.acted_at || b.executed_at || b.recorded_at || "";
    if (ta !== tb) return ta < tb ? -1 : 1;
    return typeRank(a.record_type) - typeRank(b.record_type);
  });
}

/**
 * Derive accountability findings from a chain + optional incident facts.
 *
 * @param {object[]} chain
 * @param {{ what_went_wrong?: string, observed_action?: string, severity?: string }} [incident]
 */
function isType(r, kind) {
  const map = {
    decision: ["tlpx.decision", "glass.decision"],
    operator_action: ["tlpx.operator_action", "glass.operator_action"],
    execution: ["tlpx.execution", "glass.execution"]
  };
  return map[kind]?.includes(r.record_type);
}

export function analyzeAccountability(chain, incident = {}) {
  const decision = chain.find((r) => isType(r, "decision"));
  const operator = chain.filter((r) => isType(r, "operator_action")).at(-1);
  const execution = chain.filter((r) => isType(r, "execution")).at(-1);

  /** @type {object[]} */
  const findings = [];

  if (!decision) {
    findings.push({
      code: "NO_DECISION",
      party_type: "machine",
      party_role: "evaluator",
      severity: "critical",
      summary: "No Glass decision receipt found — action may have bypassed the trust layer."
    });
    return summarize(findings, decision, operator, execution, incident);
  }

  const declarer = decision.parties?.declarer;
  const intent = decision.original_intent;

  // Machine or human declared intent
  findings.push({
    code: "INTENT_DECLARED",
    party_type: declarer?.type || "unknown",
    party_role: "declarer",
    party_id: declarer?.id,
    severity: "info",
    summary: `${declarer?.type || "unknown"} '${declarer?.id}' declared: ${intent?.declared_intent || intent?.intent_summary}`
  });

  findings.push({
    code: "POLICY_EVALUATED",
    party_type: "machine",
    party_role: "evaluator",
    party_id: "glass",
    severity: "info",
    summary: `Glass decided ${decision.decision} (${decision.reason})`
  });

  if (decision.decision === "REQUIRE_APPROVAL") {
    if (!operator) {
      findings.push({
        code: "ESCALATION_UNRESOLVED",
        party_type: "human",
        party_role: "authorizer",
        severity: "high",
        summary: "Human approval was required but no operator action is on the audit trail."
      });
    } else if (operator.outcome === "APPROVE") {
      findings.push({
        code: "HUMAN_APPROVED",
        party_type: "human",
        party_role: "authorizer",
        party_id: operator.operator?.id,
        severity: "info",
        summary: `Human '${operator.operator?.id}' approved escalated intent.`
      });
    } else if (operator.outcome === "REJECT") {
      findings.push({
        code: "HUMAN_REJECTED",
        party_type: "human",
        party_role: "authorizer",
        party_id: operator.operator?.id,
        severity: "info",
        summary: `Human '${operator.operator?.id}' rejected escalated intent.`
      });
    }
  }

  if (!execution) {
    findings.push({
      code: "NO_EXECUTION_RECORD",
      party_type: "unknown",
      party_role: "executor",
      severity: "medium",
      summary: "No execution record — cannot confirm whether the action ran."
    });
  } else {
    findings.push({
      code: "EXECUTION_RECORDED",
      party_type: execution.executor?.type,
      party_role: "executor",
      party_id: execution.executor?.id,
      severity: "info",
      summary: `Executor '${execution.executor?.id}' status=${execution.status}`
    });

    // Machine executed without authorization
    if (execution.status === "EXECUTED" && execution.authorization_status !== "AUTHORIZED") {
      findings.push({
        code: "UNAUTHORIZED_EXECUTION",
        party_type: execution.executor?.type || "machine",
        party_role: "executor",
        party_id: execution.executor?.id,
        severity: "critical",
        summary: "Execution recorded without AUTHORIZED status — machine or process bypassed the gate."
      });
    }

    // Human approved something that later failed
    if (
      execution.status === "FAILED" &&
      operator?.outcome === "APPROVE"
    ) {
      findings.push({
        code: "APPROVED_BUT_FAILED",
        party_type: "shared",
        party_role: "authorizer+executor",
        severity: "high",
        summary:
          "Human authorized execution and the run failed — review operator judgment and executor reliability."
      });
    }

    // Declared intent vs observed action divergence (incident-supplied)
    if (incident.observed_action && intent?.action && incident.observed_action !== intent.action) {
      findings.push({
        code: "INTENT_EXECUTION_MISMATCH",
        party_type: declarer?.type || "machine",
        party_role: "declarer",
        party_id: declarer?.id,
        severity: "critical",
        summary: `Declared action '${intent.action}' differs from observed '${incident.observed_action}' — possible misdeclaration or post-gate mutation.`
      });
    }
  }

  // If incident says something went wrong, add a human-readable root-surface split
  if (incident.what_went_wrong) {
    const surfaces = attributeFailureSurfaces({ decision, operator, execution, incident });
    for (const s of surfaces) findings.push(s);
  }

  return summarize(findings, decision, operator, execution, incident);
}

function attributeFailureSurfaces({ decision, operator, execution, incident }) {
  /** @type {object[]} */
  const out = [];
  const severity = incident.severity || "high";

  // Heuristic attribution for MVP demos — explicit, inspectable rules
  if (execution?.status === "EXECUTED" && operator?.outcome === "APPROVE") {
    out.push({
      code: "ACCOUNTABILITY_SURFACE_HUMAN",
      party_type: "human",
      party_role: "authorizer",
      party_id: operator.operator?.id,
      severity,
      summary: `Human authorizer shared accountability: approved the action that led to: ${incident.what_went_wrong}`
    });
  }

  if (execution?.status === "EXECUTED" && decision?.decision === "ALLOW" && !operator) {
    out.push({
      code: "ACCOUNTABILITY_SURFACE_MACHINE_POLICY",
      party_type: "machine",
      party_role: "evaluator",
      party_id: "glass",
      severity,
      summary: `Machine policy auto-allowed this path — policy pack may be too permissive for: ${incident.what_went_wrong}`
    });
  }

  if (decision?.parties?.declarer) {
    out.push({
      code: "ACCOUNTABILITY_SURFACE_DECLARER",
      party_type: decision.parties.declarer.type,
      party_role: "declarer",
      party_id: decision.parties.declarer.id,
      severity,
      summary: `Declarer accountability: owned the pre-execution intent statement related to: ${incident.what_went_wrong}`
    });
  }

  if (execution?.executor) {
    out.push({
      code: "ACCOUNTABILITY_SURFACE_EXECUTOR",
      party_type: execution.executor.type,
      party_role: "executor",
      party_id: execution.executor.id,
      severity,
      summary: `Executor accountability: carried out (or attempted) the action: ${incident.what_went_wrong}`
    });
  }

  return out;
}

function summarize(findings, decision, operator, execution, incident) {
  const humanParties = new Set();
  const machineParties = new Set();

  for (const f of findings) {
    if (f.party_type === "human" && f.party_id) humanParties.add(f.party_id);
    if (f.party_type === "machine" && f.party_id) machineParties.add(f.party_id);
    if (f.party_type === "shared") {
      if (operator?.operator?.id) humanParties.add(operator.operator.id);
      machineParties.add("glass");
    }
  }

  return {
    record_type: "tlpx.accountability_report",
    ...standardStamp(),
    report_id: uuid(),
    generated_at: nowIso(),
    receipt_id: decision?.receipt_id || null,
    incident: incident.what_went_wrong
      ? {
          what_went_wrong: incident.what_went_wrong,
          observed_action: incident.observed_action || null,
          severity: incident.severity || null
        }
      : null,
    chain_summary: {
      decision: decision
        ? { decision: decision.decision, reason: decision.reason, policy_id: decision.policy_id }
        : null,
      operator: operator
        ? { id: operator.operator?.id, outcome: operator.outcome }
        : null,
      execution: execution
        ? { status: execution.status, executor: execution.executor }
        : null
    },
    parties_involved: {
      human: [...humanParties],
      machine: [...machineParties]
    },
    findings
  };
}

export function reportFromAuditFile(auditPath, receiptId, incident = {}, opts = {}) {
  const records = readAudit(auditPath);
  const chain = buildChain(records, receiptId);
  const report = analyzeAccountability(chain, incident);

  if (opts.persist) {
    const incidentRecord = {
      record_type: "tlpx.incident",
      ...standardStamp(),
      incident_id: uuid(),
      linked_receipt_id: receiptId,
      receipt_id: receiptId,
      recorded_at: nowIso(),
      ...incident,
      report_id: report.report_id
    };
    appendAudit(auditPath, incidentRecord);
    appendAudit(auditPath, report);
  }

  return { chain, report };
}
