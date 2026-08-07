/**
 * Operator authorization for resolveEscalation (A18 partial).
 *
 * Rules (in order):
 * 1. If decision.approval_route is non-empty → operator_id MUST be on that route.
 * 2. If operators.allowlist is non-empty and enforce !== false → operator_id MUST be listed.
 * 3. Otherwise allow (legacy / tests without routes).
 */

import fs from "node:fs";

/**
 * @param {string} operatorId
 * @param {object} decision - glass decision with optional approval_route
 * @param {{ operators?: { allowlist?: string[], enforce?: boolean }, switchboard?: object }} [opts]
 */
export function assertOperatorAllowed(operatorId, decision, opts = {}) {
  if (!operatorId || typeof operatorId !== "string") {
    throw new Error("operator_id required");
  }

  const route = Array.isArray(decision?.approval_route) ? decision.approval_route : [];
  if (route.length > 0 && !route.includes(operatorId)) {
    throw new Error(
      `operator '${operatorId}' not on approval_route [${route.join(", ")}]`
    );
  }

  const ops =
    opts.operators ||
    opts.switchboard?.operators ||
    null;

  if (ops && Array.isArray(ops.allowlist) && ops.allowlist.length > 0) {
    const enforce = ops.enforce !== false;
    if (enforce && !ops.allowlist.includes(operatorId)) {
      throw new Error(
        `operator '${operatorId}' not in operators.allowlist`
      );
    }
  }
}

export function loadOperatorsFile(filePath) {
  const raw = JSON.parse(fs.readFileSync(filePath, "utf8"));
  return {
    allowlist: Array.isArray(raw.allowlist) ? raw.allowlist : [],
    enforce: raw.enforce !== false
  };
}
