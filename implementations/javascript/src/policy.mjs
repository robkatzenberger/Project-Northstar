/**
 * Minimal deterministic policy loader + safe expression evaluator.
 * Compatible with APEX-Lite style rules YAML used in ~/APEX-Lite.
 *
 * Expression language (no eval / new Function):
 *   field == "string" | field != "string" | field == true|false|number
 *   "item" in field
 *   and / or  (and binds tighter than or)
 *   parentheses
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

// ─── Safe expression evaluator ─────────────────────────────────────────

/**
 * Tokenize policy expressions. Throws on illegal characters.
 */
export function tokenize(expr) {
  const tokens = [];
  let i = 0;
  const s = expr;
  while (i < s.length) {
    const c = s[i];
    if (/\s/.test(c)) {
      i++;
      continue;
    }
    if (c === "(" || c === ")") {
      tokens.push({ type: c });
      i++;
      continue;
    }
    if (s.startsWith("==", i)) {
      tokens.push({ type: "EQ" });
      i += 2;
      continue;
    }
    if (s.startsWith("!=", i)) {
      tokens.push({ type: "NE" });
      i += 2;
      continue;
    }
    if (c === '"') {
      let j = i + 1;
      let out = "";
      while (j < s.length && s[j] !== '"') {
        if (s[j] === "\\" && j + 1 < s.length) {
          out += s[j + 1];
          j += 2;
          continue;
        }
        out += s[j];
        j++;
      }
      if (j >= s.length) throw new Error("policy expr: unterminated string");
      tokens.push({ type: "STRING", value: out });
      i = j + 1;
      continue;
    }
    if (/[A-Za-z_]/.test(c)) {
      let j = i + 1;
      while (j < s.length && /[A-Za-z0-9_]/.test(s[j])) j++;
      const word = s.slice(i, j);
      if (word === "and") tokens.push({ type: "AND" });
      else if (word === "or") tokens.push({ type: "OR" });
      else if (word === "in") tokens.push({ type: "IN" });
      else if (word === "true") tokens.push({ type: "BOOL", value: true });
      else if (word === "false") tokens.push({ type: "BOOL", value: false });
      else tokens.push({ type: "IDENT", value: word });
      i = j;
      continue;
    }
    if (/[0-9-]/.test(c)) {
      let j = i + 1;
      while (j < s.length && /[0-9.]/.test(s[j])) j++;
      const num = Number(s.slice(i, j));
      if (Number.isNaN(num)) throw new Error(`policy expr: bad number near ${s.slice(i)}`);
      tokens.push({ type: "NUMBER", value: num });
      i = j;
      continue;
    }
    throw new Error(`policy expr: illegal character '${c}' at ${i}`);
  }
  return tokens;
}

/**
 * Parse tokens into AST.
 * or-expr := and-expr (OR and-expr)*
 * and-expr := cmp (AND cmp)*
 * cmp := STRING IN IDENT | IDENT EQ|NE value | ( or-expr )
 * value := STRING | NUMBER | BOOL
 */
export function parseExpr(tokens) {
  let pos = 0;
  const peek = () => tokens[pos];
  const take = (type) => {
    const t = tokens[pos];
    if (!t || (type && t.type !== type)) {
      throw new Error(`policy expr: expected ${type || "token"}, got ${t?.type || "EOF"}`);
    }
    pos++;
    return t;
  };

  function parseOr() {
    let left = parseAnd();
    while (peek()?.type === "OR") {
      take("OR");
      left = { type: "or", left, right: parseAnd() };
    }
    return left;
  }

  function parseAnd() {
    let left = parseCmp();
    while (peek()?.type === "AND") {
      take("AND");
      left = { type: "and", left, right: parseCmp() };
    }
    return left;
  }

  function parseCmp() {
    if (peek()?.type === "(") {
      take("(");
      const inner = parseOr();
      take(")");
      return inner;
    }
    // "x" in field
    if (peek()?.type === "STRING" && tokens[pos + 1]?.type === "IN") {
      const str = take("STRING").value;
      take("IN");
      const field = take("IDENT").value;
      return { type: "in", item: str, field };
    }
    // field == / != value
    if (peek()?.type === "IDENT") {
      const field = take("IDENT").value;
      const op = peek()?.type;
      if (op !== "EQ" && op !== "NE") {
        throw new Error(`policy expr: expected == or != after ${field}`);
      }
      take(op);
      const v = peek();
      if (!v || !["STRING", "NUMBER", "BOOL"].includes(v.type)) {
        throw new Error(`policy expr: expected value after ${field}`);
      }
      pos++;
      return {
        type: op === "EQ" ? "eq" : "ne",
        field,
        value: v.value
      };
    }
    throw new Error(`policy expr: unexpected token ${peek()?.type || "EOF"}`);
  }

  const ast = parseOr();
  if (pos < tokens.length) {
    throw new Error(`policy expr: trailing token ${tokens[pos].type}`);
  }
  return ast;
}

export function evalAst(ast, intent) {
  switch (ast.type) {
    case "or":
      return evalAst(ast.left, intent) || evalAst(ast.right, intent);
    case "and":
      return evalAst(ast.left, intent) && evalAst(ast.right, intent);
    case "eq":
      return intent[ast.field] === ast.value;
    case "ne":
      return intent[ast.field] !== ast.value;
    case "in": {
      const arr = intent[ast.field];
      return Array.isArray(arr) && arr.includes(ast.item);
    }
    default:
      throw new Error(`policy expr: unknown node ${ast.type}`);
  }
}

/**
 * Evaluate a tiny policy expression against intent fields (safe, no Function).
 */
export function evaluateCondition(expression, intent) {
  if (!expression || typeof expression !== "string") return false;
  try {
    const tokens = tokenize(expression);
    const ast = parseExpr(tokens);
    return Boolean(evalAst(ast, intent));
  } catch {
    // Malformed rule does not match (fail closed for that rule)
    return false;
  }
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
        reason:
          rule.description ||
          (rule.require ? `Policy requires ${rule.require}` : "Policy requires approval"),
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
