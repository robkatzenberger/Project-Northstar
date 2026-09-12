function evaluateRules(intent, policy) {
  for (const rule of policy.rules) {
    if (!rule.if || !evaluateExpression(rule.if, intent)) {
      continue;
    }

    if (rule.require || rule.deny === true) {
      return {
        decision: "REQUIRE_APPROVAL",
        reason: rule.description || "Policy requires approval",
        policy_id: rule.id || null
      };
    }
  }

  return {
    decision: "ALLOW",
    reason: "No approval rules matched",
    policy_id: null
  };
}

function evaluateExpression(expression, intent) {
  const orClauses = splitExpression(expression, " or ");
  return orClauses.some((clause) => {
    const andClauses = splitExpression(clause, " and ");
    return andClauses.every((part) => evaluateClause(part.trim(), intent));
  });
}

function splitExpression(expression, separator) {
  return expression
    .split(separator)
    .map((part) => part.trim())
    .filter(Boolean);
}

function evaluateClause(clause, intent) {
  const inMatch = clause.match(/^"([^"]+)"\s+in\s+([A-Za-z_][A-Za-z0-9_]*)$/);
  if (inMatch) {
    const value = intent[inMatch[2]];
    return Array.isArray(value) && value.includes(inMatch[1]);
  }

  const equalityMatch = clause.match(/^([A-Za-z_][A-Za-z0-9_]*)\s*(==|!=)\s*(.+)$/);
  if (equalityMatch) {
    const actual = intent[equalityMatch[1]];
    const expected = parseLiteral(equalityMatch[3].trim());
    return equalityMatch[2] === "==" ? actual === expected : actual !== expected;
  }

  throw new Error(`Unsupported policy expression: ${clause}`);
}

function parseLiteral(value) {
  if (value === "true") {
    return true;
  }

  if (value === "false") {
    return false;
  }

  if (/^-?\d+$/.test(value)) {
    return Number(value);
  }

  const quoted = value.match(/^"(.*)"$/);
  if (quoted) {
    return quoted[1];
  }

  throw new Error(`Unsupported literal in policy expression: ${value}`);
}

module.exports = {
  evaluateExpression,
  evaluateRules
};
