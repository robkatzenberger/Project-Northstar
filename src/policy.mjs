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

/**
 * Evaluate a tiny policy expression against intent fields.
 * Avoids `with` so missing fields are undefined (not ReferenceError).
 * Supports: ==, !=, &&, ||, "X" in fieldName, bare field identifiers.
 */
function evaluateCondition(expression, intent) {
  let expr = expression
    .replace(/\band\b/g, "&&")
    .replace(/\bor\b/g, "||");

  // "PII" in data_classes → __includes(__intent["data_classes"], "PII")
  expr = expr.replace(
    /"([^"]+)"\s+in\s+([A-Za-z_][A-Za-z0-9_]*)/g,
    '__includes(__intent["$2"], "$1")'
  );

  // Replace bare field identifiers without touching string literals.
  // Split on "..." segments; only rewrite odd/even outside quotes.
  const parts = expr.split(/("(?:\\.|[^"\\])*")/);
  expr = parts
    .map((part, i) => {
      // Even indices are outside quotes (split keeps delimiters on odd indices)
      if (i % 2 === 1) return part;
      return part.replace(/\b([A-Za-z_][A-Za-z0-9_]*)\b/g, (id) => {
        if (
          id === "true" ||
          id === "false" ||
          id === "__intent" ||
          id === "__includes"
        ) {
          return id;
        }
        return `__intent[${JSON.stringify(id)}]`;
      });
    })
    .join("");

  const evaluator = new Function(
    "__intent",
    "__includes",
    `return (${expr});`
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
