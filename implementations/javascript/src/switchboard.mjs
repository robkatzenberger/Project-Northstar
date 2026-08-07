/**
 * Switchboard — agent/machine identity router.
 *
 * Responsibilities:
 * 1. Identify principal (agent / machine / human)
 * 2. Enforce whitelist access
 * 3. Attach credibility score in [0, 0.99]
 * 4. Route escalations to the right approvers
 *
 * Credibility is operator-managed evidence of trust — not an LLM judgment.
 * Policy remains deterministic; scores only change which deterministic branch runs.
 */

import fs from "node:fs";

export const CRED_MIN = 0;
export const CRED_MAX = 0.99;

/**
 * @typedef {object} Principal
 * @property {string} id
 * @property {"machine"|"human"|"agent"} type
 * @property {boolean} whitelisted
 * @property {number} credibility
 * @property {string[]} [allowed_actions]
 * @property {string[]} [approval_route]
 * @property {string} [label]
 */

/**
 * @typedef {object} SwitchboardConfig
 * @property {string} [switchboard_id]
 * @property {"DENY"|"REQUIRE_APPROVAL"} [unknown_agent_policy]
 * @property {{ force_escalate_below?: number, high_trust_at_or_above?: number, max_credibility?: number }} [thresholds]
 * @property {{ approval_route?: string[] }} [defaults]
 * @property {Principal[]} principals
 */

export function loadSwitchboard(filePath) {
  const raw = JSON.parse(fs.readFileSync(filePath, "utf8"));
  return normalizeConfig(raw);
}

export function normalizeConfig(raw) {
  if (!raw || !Array.isArray(raw.principals)) {
    throw new Error("switchboard: principals array required");
  }

  const maxCred = raw.thresholds?.max_credibility ?? CRED_MAX;
  if (maxCred > CRED_MAX) {
    throw new Error(`switchboard: max_credibility cannot exceed ${CRED_MAX}`);
  }

  const principals = raw.principals.map((p) => validatePrincipal(p, maxCred));
  const byId = new Map(principals.map((p) => [p.id, p]));

  const operators = raw.operators
    ? {
        enforce: raw.operators.enforce !== false,
        allowlist: Array.isArray(raw.operators.allowlist)
          ? raw.operators.allowlist.map(String)
          : []
      }
    : null;

  return {
    switchboard_id: raw.switchboard_id || "switchboard",
    unknown_agent_policy: raw.unknown_agent_policy === "REQUIRE_APPROVAL" ? "REQUIRE_APPROVAL" : "DENY",
    thresholds: {
      force_escalate_below: num(raw.thresholds?.force_escalate_below, 0.4),
      high_trust_at_or_above: num(raw.thresholds?.high_trust_at_or_above, 0.85),
      max_credibility: maxCred
    },
    defaults: {
      approval_route: Array.isArray(raw.defaults?.approval_route)
        ? raw.defaults.approval_route
        : []
    },
    operators,
    principals,
    byId
  };
}

function num(v, fallback) {
  const n = Number(v);
  return Number.isFinite(n) ? n : fallback;
}

function validatePrincipal(p, maxCred) {
  if (!p?.id || typeof p.id !== "string") {
    throw new Error("switchboard: principal.id required");
  }
  const credibility = Number(p.credibility);
  if (!Number.isFinite(credibility) || credibility < CRED_MIN || credibility > maxCred) {
    throw new Error(
      `switchboard: principal ${p.id} credibility must be in [${CRED_MIN}, ${maxCred}] (got ${p.credibility})`
    );
  }
  if (credibility > CRED_MAX) {
    throw new Error(`switchboard: credibility cannot exceed ${CRED_MAX}`);
  }

  const type = p.type === "human" ? "human" : p.type === "agent" ? "agent" : "machine";

  return {
    id: p.id,
    type,
    label: p.label || p.id,
    whitelisted: p.whitelisted === true,
    credibility,
    allowed_actions: Array.isArray(p.allowed_actions) ? p.allowed_actions : null,
    approval_route: Array.isArray(p.approval_route) ? p.approval_route : []
  };
}

/**
 * Resolve principal by actor id.
 * @returns {{ status: "known"|"unknown", principal: Principal|null }}
 */
export function lookupPrincipal(switchboard, actorId) {
  if (!actorId) return { status: "unknown", principal: null };
  const principal = switchboard.byId.get(actorId) || null;
  return principal ? { status: "known", principal } : { status: "unknown", principal: null };
}

/**
 * Route an evaluation intent through the switchboard.
 * Returns enrichment fields + optional hard gate outcome.
 *
 * @returns {object} switchboard_context attached to evaluation
 */
export function routeThroughSwitchboard(switchboard, intent) {
  const actorId = intent.actor;
  const { status, principal } = lookupPrincipal(switchboard, actorId);
  const action = intent.action ?? null;
  const thr = switchboard.thresholds;

  /** @type {object} */
  const ctx = {
    switchboard_id: switchboard.switchboard_id,
    lookup: status,
    principal_id: principal?.id || actorId || null,
    principal_type: principal?.type || intent.actor_type || "machine",
    whitelisted: false,
    credibility: null,
    credibility_band: "unknown",
    action_permitted: false,
    approval_route: [...(switchboard.defaults.approval_route || [])],
    gate: null, // null | { decision, reason, policy_id }
    flags: []
  };

  // --- Unknown agent ---
  if (status === "unknown") {
    ctx.flags.push("UNKNOWN_PRINCIPAL");
    if (switchboard.unknown_agent_policy === "DENY") {
      ctx.gate = {
        decision: "DENY",
        reason: "Switchboard: principal not registered",
        policy_id: "switchboard.unknown_deny"
      };
    } else {
      ctx.gate = {
        decision: "REQUIRE_APPROVAL",
        reason: "Switchboard: unknown principal requires human approval",
        policy_id: "switchboard.unknown_escalate"
      };
      ctx.approval_route = [...(switchboard.defaults.approval_route || [])];
    }
    return ctx;
  }

  // --- Known principal ---
  ctx.whitelisted = principal.whitelisted === true;
  ctx.credibility = principal.credibility;
  ctx.credibility_band = bandFor(principal.credibility, thr);
  ctx.approval_route =
    principal.approval_route?.length > 0
      ? [...principal.approval_route]
      : [...(switchboard.defaults.approval_route || [])];

  // Party type for TL-PX: agent → machine (agents are non-human principals)
  ctx.actor_type_normalized =
    principal.type === "human" ? "human" : "machine";

  if (!ctx.whitelisted) {
    ctx.flags.push("NOT_WHITELISTED");
    ctx.gate = {
      decision: "DENY",
      reason: "Switchboard: principal not on whitelist",
      policy_id: "switchboard.not_whitelisted"
    };
    return ctx;
  }

  // Action allow-list (optional per principal)
  if (principal.allowed_actions !== null) {
    if (action && !principal.allowed_actions.includes(action)) {
      ctx.flags.push("ACTION_NOT_PERMITTED");
      ctx.action_permitted = false;
      ctx.gate = {
        decision: "DENY",
        reason: `Switchboard: action '${action}' not permitted for ${principal.id}`,
        policy_id: "switchboard.action_denied"
      };
      return ctx;
    }
    if (!action && principal.allowed_actions.length > 0) {
      // No action code — do not hard-deny; policy layer decides
      ctx.flags.push("ACTION_UNSPECIFIED");
      ctx.action_permitted = true;
    } else {
      ctx.action_permitted = true;
    }
  } else {
    ctx.action_permitted = true;
  }

  // Credibility routing flags (policy can also read numeric credibility)
  if (principal.credibility < thr.force_escalate_below) {
    ctx.flags.push("LOW_CREDIBILITY");
  }
  if (principal.credibility >= thr.high_trust_at_or_above) {
    ctx.flags.push("HIGH_TRUST");
  }

  return ctx;
}

function bandFor(score, thr) {
  if (score < thr.force_escalate_below) return "low";
  if (score >= thr.high_trust_at_or_above) return "high";
  return "medium";
}

/**
 * Merge switchboard context into evaluation intent fields for policy expressions.
 */
export function enrichIntentWithSwitchboard(intent, sbCtx) {
  return {
    ...intent,
    actor_type: sbCtx.actor_type_normalized || intent.actor_type,
    whitelisted: sbCtx.whitelisted,
    credibility: sbCtx.credibility,
    credibility_band: sbCtx.credibility_band,
    switchboard_flags: sbCtx.flags,
    // numeric helper for policy: low_credibility == true style via band
    low_credibility: sbCtx.credibility_band === "low",
    high_trust: sbCtx.credibility_band === "high"
  };
}

/**
 * Optional credibility update after outcomes (operator-managed learning signal).
 * Clamps to [0, 0.99]. Does not auto-write config — returns new score for caller to persist.
 *
 * @param {number} current
 * @param {"success"|"failure"|"escalation_honest"|"rejected"|"bypass_suspected"} event
 */
export function suggestCredibilityDelta(current, event) {
  const deltas = {
    success: +0.02,
    failure: -0.05,
    escalation_honest: +0.01,
    rejected: -0.03,
    bypass_suspected: -0.15
  };
  const delta = deltas[event] ?? 0;
  const next = Math.min(CRED_MAX, Math.max(CRED_MIN, round2(current + delta)));
  return { previous: current, delta, next, event };
}

function round2(n) {
  return Math.round(n * 100) / 100;
}
