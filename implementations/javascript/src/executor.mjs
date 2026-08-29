/**
 * Cooperative TL-PX 0.1 reference executor.
 *
 * Side effects run ONLY when the audit chain says AUTHORIZED for the receipt.
 * This is not the TL-PX 0.2 authority/claim path and does not mediate callers
 * that retain direct access to the supplied side effect.
 */

import { resolveAuthorizationFromAudit } from "./chain.mjs";
import { recordExecution } from "./glass.mjs";

/**
 * @param {object} opts
 * @param {string} opts.auditPath
 * @param {string} opts.receipt_id
 * @param {string} opts.executor_id
 * @param {"machine"|"human"} [opts.executor_type]
 * @param {(auth: object) => (any|Promise<any>)} opts.sideEffect
 * @param {string} [opts.result_summary]
 */
export async function executeAuthorized(opts) {
  const {
    auditPath,
    receipt_id,
    executor_id,
    executor_type = "machine",
    sideEffect,
    result_summary = null
  } = opts || {};

  if (!auditPath) throw new Error("executeAuthorized: auditPath required");
  if (!receipt_id) throw new Error("executeAuthorized: receipt_id required");
  if (!executor_id) throw new Error("executeAuthorized: executor_id required");
  if (typeof sideEffect !== "function") {
    throw new Error("executeAuthorized: sideEffect function required");
  }

  let auth;
  try {
    auth = resolveAuthorizationFromAudit(auditPath, receipt_id);
  } catch (err) {
    return {
      ok: false,
      reason: err.message,
      execution: null,
      auth: null
    };
  }

  if (auth.authorization_status !== "AUTHORIZED") {
    let execution = null;
    try {
      execution = recordExecution(
        {
          receipt_id,
          executor_id,
          executor_type,
          status: "BLOCKED",
          result_summary:
            result_summary ||
            `Blocked: authorization_status=${auth.authorization_status} (${auth.source})`
        },
        { auditPath }
      );
    } catch (recordErr) {
      return {
        ok: false,
        reason: `Not authorized (${auth.authorization_status}); also failed to record BLOCKED: ${recordErr.message}`,
        execution: null,
        auth
      };
    }
    return {
      ok: false,
      reason: `Not authorized: ${auth.authorization_status} via ${auth.source}`,
      execution,
      auth
    };
  }

  try {
    const result = await sideEffect(auth);
    const summary =
      result_summary ||
      (typeof result === "string" ? result : result != null ? "ok" : null);
    const execution = recordExecution(
      {
        receipt_id,
        executor_id,
        executor_type,
        status: "EXECUTED",
        result_summary: summary
      },
      { auditPath }
    );
    return { ok: true, result, execution, auth };
  } catch (err) {
    const execution = recordExecution(
      {
        receipt_id,
        executor_id,
        executor_type,
        status: "FAILED",
        result_summary: result_summary || null,
        error: err.message || String(err)
      },
      { auditPath }
    );
    return {
      ok: false,
      reason: err.message || String(err),
      execution,
      auth,
      failed: true
    };
  }
}
