/**
 * Minimal deterministic policy loader + evaluator.
 * Compatible with APEX-Lite style rules YAML used in ~/APEX-Lite.
 */

import fs from "node:fs";

function parseScalar(value) {
  if (value === "true") return true;
  if (value === "false") return false;
  if (/^-?\d+$/.test(value)) return Number(value);
  const quoted = value.match(/^"(.*)"$/);
  if (quoted) return quoted[1];
  return value;
}

export function readPolicyFile(filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  return parsePolicyText(text);
}

export function parsePolicyText(text) {
  const lines = text.split(/\r?\n/);
  const rules = [];
  let currentRule = null;

  for (const rawLine of lines) {
    const line = rawLine.trimEnd();
    if (!line.trim() || line.trimStart().startsWith("#") || line.trim() === "rules:") {
      continue;
    }

    const ruleMatch = line.match(/^\s*-\s+id:\s+(.+)$/);
    if (ruleMatch) {
      currentRule = { id: parseScalar(ruleMatch[1].trim()) };
      rules.push(currentRule);
      continue;
    }

    const fieldMatch = line.match(/^\s+([A-Za-z_]+):\s+(.+)$/);
    if (fieldMatch && currentRule) {
      currentRule[fieldMatch[1]] = parseScalar(fieldMatch[2].trim());
    }
  }

  return {
    policy_pack_id: "default",
    rules
  };
}

function evaluateCondition(expression, intent) {
  const normalized = expression
    .replace(/\band\b/g, "&&")
    .replace(/\bor\b/g, "||")
    .replace(/"([^"]+)"\s+in\s+([A-Za-z_][A-Za-z0-9_]*)/g, 'includes($2, "$1")');

  const evaluator = new Function(
    "intent",
    "includes",
    `with (intent) { return (${normalized}); }`
  );

  return Boolean(
    evaluator(intent, (value, item) => Array.isArray(value) && value.includes(item))
  );
}

/**
 * @returns {{ decision: "ALLOW"|"REQUIRE_APPROVAL", reason: string, policy_id: string|null }}
 */
export function evaluateRules(intent, policy) {
  for (const rule of policy.rules) {
    if (!rule.if || !evaluateCondition(rule.if, intent)) {
      continue;
    }

    // deny: true maps to REQUIRE_APPROVAL (APEX-Lite ALLOW_OR_ESCALATE philosophy)
    if (rule.deny === true || rule.require) {
      return {
        decision: "REQUIRE_APPROVAL",
        reason: rule.description || (rule.require ? `Policy requires ${rule.require}` : "Policy requires approval"),
        policy_id: rule.id
      };
    }
  }

  return {
    decision: "ALLOW",
    reason: "No approval rules matched",
    policy_id: null
  };
}
