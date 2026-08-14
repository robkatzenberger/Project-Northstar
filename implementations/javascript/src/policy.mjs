/**
 * Deterministic policy loader, compiler, and evaluator.
 *
 * Stages: parse → validate → compile → evaluate.
 * The entire pack is compiled before any decision. Invalid policy
 * never authorizes. Compile/parse errors are not treated as a non-match.
 *
 * Expression language (no eval / new Function):
 *   field == "string" | field != "string" | field == true|false|number
 *   "item" in field
 *   and / or  (and binds tighter than or)
 *   parentheses
 */

import fs from "node:fs";

export const POLICY_COMPILED = Symbol("northstar.policy.compiled");

export const POLICY_RULE_KEYS = Object.freeze([
  "id",
  "description",
  "if",
  "require",
  "deny"
]);

export const POLICY_PACK_KEYS = Object.freeze(["policy_pack_id", "rules"]);

export const POLICY_REQUIRE_VALUES = Object.freeze(["human_approval"]);

/** Intent fields a compiled expression may name. */
export const POLICY_EXPRESSION_FIELDS = Object.freeze([
  "action",
  "target",
  "risk",
  "data_classes",
  "actor",
  "actor_type",
  "whitelisted",
  "credibility",
  "credibility_band",
  "low_credibility",
  "high_trust"
]);

/** Fields that may appear on the right of `in`. */
export const POLICY_ARRAY_FIELDS = Object.freeze(["data_classes"]);

const RULE_KEY_SET = new Set(POLICY_RULE_KEYS);
const PACK_KEY_SET = new Set(POLICY_PACK_KEYS);
const FIELD_SET = new Set(POLICY_EXPRESSION_FIELDS);
const ARRAY_FIELD_SET = new Set(POLICY_ARRAY_FIELDS);
const REQUIRE_SET = new Set(POLICY_REQUIRE_VALUES);
const YAML_RULE_FIELDS = new Set(["description", "if", "require", "deny"]);

function parseScalar(value) {
  if (value === "true") return true;
  if (value === "false") return false;
  if (/^-?\d+$/.test(value)) return Number(value);
  const quoted = value.match(/^"(.*)"$/);
  if (quoted) return quoted[1];
  return value;
}

function policyError(message) {
  return new Error(`policy: ${message}`);
}

export function readPolicyFile(filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  return compilePolicy(parsePolicyText(text));
}

/**
 * Parse a restricted YAML subset into an uncompiled pack.
 * Unmatched or unknown structure is rejected.
 */
export function parsePolicyText(text) {
  if (typeof text !== "string") {
    throw policyError("policy text must be a string");
  }

  const lines = text.split(/\r?\n/);
  const rules = [];
  let currentRule = null;
  let seenRulesKey = false;
  let policyPackId = "default";

  for (let i = 0; i < lines.length; i++) {
    const rawLine = lines[i];
    const trimmed = rawLine.trim();
    const lineNo = i + 1;

    if (!trimmed || trimmed.startsWith("#")) continue;

    if (/^policy_pack_id:\s+\S/.test(trimmed) && !seenRulesKey && !currentRule) {
      policyPackId = parseScalar(trimmed.slice(trimmed.indexOf(":") + 1).trim());
      if (typeof policyPackId !== "string" || !policyPackId) {
        throw policyError(`invalid policy_pack_id at line ${lineNo}`);
      }
      continue;
    }

    if (trimmed === "rules:") {
      if (seenRulesKey) {
        throw policyError(`duplicate rules: key at line ${lineNo}`);
      }
      seenRulesKey = true;
      currentRule = null;
      continue;
    }

    const ruleMatch = rawLine.match(/^(\s*)-\s+id:\s*(.*)$/);
    if (ruleMatch) {
      if (!seenRulesKey) {
        throw policyError(`rule before rules: key at line ${lineNo}`);
      }
      const id = parseScalar(String(ruleMatch[2] ?? "").trim());
      if (typeof id !== "string" || !id) {
        throw policyError(`missing rule id at line ${lineNo}`);
      }
      currentRule = { id };
      rules.push(currentRule);
      continue;
    }

    const fieldMatch = rawLine.match(/^\s+([A-Za-z_]+):\s*(.*)$/);
    if (fieldMatch && currentRule) {
      const key = fieldMatch[1];
      const rawValue = fieldMatch[2].trim();
      if (!YAML_RULE_FIELDS.has(key)) {
        throw policyError(`unknown field '${key}' at line ${lineNo}`);
      }
      if (Object.prototype.hasOwnProperty.call(currentRule, key)) {
        throw policyError(`duplicate field '${key}' on rule ${currentRule.id} at line ${lineNo}`);
      }
      if (!rawValue) {
        throw policyError(`empty field '${key}' at line ${lineNo}`);
      }
      currentRule[key] = parseScalar(rawValue);
      continue;
    }

    throw policyError(`unsupported structure at line ${lineNo}`);
  }

  if (!seenRulesKey) {
    throw policyError("missing rules: key");
  }

  return {
    policy_pack_id: policyPackId,
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
      if (j >= s.length) throw policyError("unterminated string");
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
      if (Number.isNaN(num)) throw policyError(`bad number near ${s.slice(i)}`);
      tokens.push({ type: "NUMBER", value: num });
      i = j;
      continue;
    }
    throw policyError(`illegal character '${c}' at ${i}`);
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
      throw policyError(`expected ${type || "token"}, got ${t?.type || "EOF"}`);
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
        throw policyError(`expected == or != after ${field}`);
      }
      take(op);
      const v = peek();
      if (!v || !["STRING", "NUMBER", "BOOL"].includes(v.type)) {
        throw policyError(`expected value after ${field}`);
      }
      pos++;
      return {
        type: op === "EQ" ? "eq" : "ne",
        field,
        value: v.value
      };
    }
    throw policyError(`unexpected token ${peek()?.type || "EOF"}`);
  }

  const ast = parseOr();
  if (pos < tokens.length) {
    throw policyError(`trailing token ${tokens[pos].type}`);
  }
  return ast;
}

function walkAstFields(ast, visit) {
  switch (ast.type) {
    case "or":
    case "and":
      walkAstFields(ast.left, visit);
      walkAstFields(ast.right, visit);
      return;
    case "eq":
    case "ne":
      visit(ast.field, ast.type);
      return;
    case "in":
      visit(ast.field, "in");
      return;
    default:
      throw policyError(`unknown node ${ast.type}`);
  }
}

export function compileExpression(expression) {
  if (typeof expression !== "string" || !expression.trim()) {
    throw policyError("missing condition");
  }
  const tokens = tokenize(expression);
  if (tokens.length === 0) {
    throw policyError("empty condition");
  }
  const ast = parseExpr(tokens);
  walkAstFields(ast, (field, op) => {
    if (!FIELD_SET.has(field)) {
      throw policyError(`unknown field '${field}'`);
    }
    if (op === "in" && !ARRAY_FIELD_SET.has(field)) {
      throw policyError(`'in' requires an array field, got '${field}'`);
    }
  });
  return ast;
}

function compileRule(rule, index, ids) {
  if (!rule || typeof rule !== "object" || Array.isArray(rule)) {
    throw policyError(`rule ${index} must be an object`);
  }

  for (const key of Object.keys(rule)) {
    if (!RULE_KEY_SET.has(key)) {
      throw policyError(`unknown field '${key}' on rule ${rule.id ?? index}`);
    }
  }

  if (typeof rule.id !== "string" || !rule.id.trim()) {
    throw policyError(`rule ${index} missing id`);
  }
  if (ids.has(rule.id)) {
    throw policyError(`duplicate rule id '${rule.id}'`);
  }
  ids.add(rule.id);

  if (rule.description !== undefined && typeof rule.description !== "string") {
    throw policyError(`rule ${rule.id} description must be a string`);
  }

  const ast = compileExpression(rule.if);

  const hasRequire = rule.require !== undefined && rule.require !== null && rule.require !== false;
  const hasDeny = rule.deny !== undefined;

  if (hasRequire) {
    if (typeof rule.require !== "string" || !REQUIRE_SET.has(rule.require)) {
      throw policyError(`rule ${rule.id} invalid require '${rule.require}'`);
    }
  }
  if (hasDeny && rule.deny !== true) {
    throw policyError(`rule ${rule.id} invalid deny value`);
  }
  if (!hasRequire && rule.deny !== true) {
    throw policyError(`rule ${rule.id} has no effect`);
  }

  const compiled = {
    id: rule.id,
    description: rule.description,
    if: rule.if,
    require: hasRequire ? rule.require : undefined,
    deny: rule.deny === true ? true : undefined
  };
  Object.defineProperty(compiled, "ast", {
    value: ast,
    enumerable: false
  });
  return Object.freeze(compiled);
}

/**
 * Validate and compile an entire pack. Idempotent for already-compiled packs.
 * Throws; never returns a pack that may authorize.
 */
export function compilePolicy(policy) {
  if (!policy || typeof policy !== "object" || Array.isArray(policy)) {
    throw policyError("pack must be an object");
  }
  if (policy[POLICY_COMPILED]) return policy;

  for (const key of Object.keys(policy)) {
    if (!PACK_KEY_SET.has(key)) {
      throw policyError(`unknown pack field '${key}'`);
    }
  }

  if (!Array.isArray(policy.rules)) {
    throw policyError("rules must be an array");
  }
  if (policy.rules.length === 0) {
    throw policyError("empty policy pack");
  }

  const ids = new Set();
  const rules = policy.rules.map((rule, i) => compileRule(rule, i, ids));

  const compiled = {
    policy_pack_id:
      typeof policy.policy_pack_id === "string" && policy.policy_pack_id
        ? policy.policy_pack_id
        : "default",
    rules: Object.freeze(rules)
  };
  Object.defineProperty(compiled, POLICY_COMPILED, {
    value: true,
    enumerable: false
  });
  return Object.freeze(compiled);
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
      throw policyError(`unknown node ${ast.type}`);
  }
}

/**
 * Evaluate a tiny policy expression against intent fields (safe, no Function).
 * Compile errors throw. They are not treated as a non-match.
 */
export function evaluateCondition(expression, intent) {
  const ast = compileExpression(expression);
  return Boolean(evalAst(ast, intent));
}

/**
 * @returns {{ decision: "ALLOW"|"REQUIRE_APPROVAL", reason: string, policy_id: string|null }}
 */
export function evaluateRules(intent, policy) {
  const compiled = compilePolicy(policy);
  for (const rule of compiled.rules) {
    if (!evalAst(rule.ast, intent)) continue;
    return {
      decision: "REQUIRE_APPROVAL",
      reason:
        rule.description ||
        (rule.require ? `Policy requires ${rule.require}` : "Policy requires approval"),
      policy_id: rule.id
    };
  }

  return {
    decision: "ALLOW",
    reason: "No approval rules matched",
    policy_id: null
  };
}
