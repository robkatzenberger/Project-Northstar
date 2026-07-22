/**
 * Prism-compatible intent signal (v0.1) plus optional Glass enrichment.
 * Prism describes intent — it does not judge.
 */

import { uuid, nowIso } from "./ids.mjs";

export const PRISM_VERSION = "prism_v0.1";

/**
 * @param {object} input
 * @param {string} input.agent - agent or human principal id
 * @param {string} input.intent_summary - human-readable summary
 * @param {"human"|"machine"} [input.actor_type]
 * @param {string} [input.action]
 * @param {string} [input.target]
 * @param {"low"|"medium"|"high"} [input.risk]
 * @param {string[]} [input.data_classes]
 * @param {object} [input.context] - non-sensitive operator context only
 */
export function createPrismSignal(input) {
  if (!input || typeof input !== "object") {
    throw new Error("createPrismSignal: input object required");
  }
  if (!input.agent || !String(input.agent).trim()) {
    throw new Error("createPrismSignal: agent is required");
  }
  if (!input.intent_summary || !String(input.intent_summary).trim()) {
    throw new Error("createPrismSignal: intent_summary is required");
  }

  const actorType = input.actor_type || "machine";
  if (actorType !== "human" && actorType !== "machine") {
    throw new Error('createPrismSignal: actor_type must be "human" or "machine"');
  }

  /** @type {Record<string, unknown>} */
  const signal = {
    prism_id: uuid(),
    timestamp: nowIso(),
    agent: String(input.agent).trim(),
    intent_summary: String(input.intent_summary).trim(),
    prism_version: PRISM_VERSION,
    // Glass extensions (not part of minimal Prism v0.1 core)
    glass: {
      actor_type: actorType,
      action: input.action ?? null,
      target: input.target ?? null,
      risk: input.risk ?? "low",
      data_classes: Array.isArray(input.data_classes) ? [...input.data_classes] : [],
      context: input.context && typeof input.context === "object" ? { ...input.context } : {}
    }
  };

  return signal;
}

/** Flatten Prism + glass extensions into evaluation intent. */
export function toEvaluationIntent(signal) {
  const g = signal.glass || {};
  return {
    intent_id: signal.prism_id,
    prism_id: signal.prism_id,
    prism_version: signal.prism_version,
    actor: signal.agent,
    actor_type: g.actor_type || "machine",
    declared_intent: signal.intent_summary,
    intent_summary: signal.intent_summary,
    action: g.action,
    target: g.target,
    risk: g.risk || "low",
    data_classes: g.data_classes || [],
    context: g.context || {},
    timestamp: signal.timestamp
  };
}
