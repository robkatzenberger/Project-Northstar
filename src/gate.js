import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createSwitchboard } from './switchboard.js';

const catalog = JSON.parse(readFileSync(new URL('../config/action-catalog.json', import.meta.url), 'utf8'));
const actions = new Set(catalog.actions.map(({ id }) => id));
const decisions = new Set(['APPROVE', 'ESCALATE']);
const legacyDecisions = new Set(['ALLOW', 'DENY', 'REQUIRE_APPROVAL']);
const targetPattern = /^[A-Za-z0-9][A-Za-z0-9._:/-]{0,255}$/;
const maximumBytes = 32 * 1024;
const maximumRules = 200;

function invalidRules(message, line) {
  const error = new TypeError(message);
  error.code = 'INVALID_RULES';
  if (line !== undefined) error.line = line;
  return error;
}

function hasFields(value, fields) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) return false;
  const descriptors = Object.getOwnPropertyDescriptors(value);
  return Reflect.ownKeys(descriptors).length === fields.length
    && fields.every((field) => Object.hasOwn(descriptors, field)
      && Object.hasOwn(descriptors[field], 'value') && descriptors[field].enumerable);
}

/**
 * Compile the whole operator-entered policy before it can be used. Only full
 * line comments, ASCII spaces/tabs, and LF/CRLF line endings are syntax.
 * Targets are opaque, case-sensitive identifiers: no expansion or normalization.
 */
function parseDecisionRules(text, vocabulary, decisionMessage) {
  if (typeof text !== 'string' || !text.isWellFormed()) {
    throw invalidRules('Rules must be well-formed Unicode text.');
  }
  if (Buffer.byteLength(text, 'utf8') > maximumBytes) {
    throw invalidRules('Rules must contain at most 32 KiB of UTF-8 text.');
  }
  const rules = [];
  const seen = new Set();
  const lines = text.split('\n');
  for (const [index, source] of lines.entries()) {
    const line = index + 1;
    const content = (index < lines.length - 1 ? source.replace(/\r$/, '') : source)
      .replace(/^[ \t]+|[ \t]+$/g, '');
    if (content.includes('\r')) throw invalidRules('Use LF or CRLF line endings.', line);
    if (content === '' || content.startsWith('#')) continue;
    const fields = content.split(/[ \t]+/);
    if (fields.length !== 3) {
      throw invalidRules('Expected DECISION action target, with no extra fields.', line);
    }
    const [decision, action, target] = fields;
    if (!vocabulary.has(decision)) {
      throw invalidRules(decisionMessage, line);
    }
    if (!actions.has(action)) throw invalidRules('Action is not in the Apex action catalog.', line);
    if (target !== target.trim() || !targetPattern.test(target)) {
      throw invalidRules('Target must be one exact identifier, without wildcards.', line);
    }
    const key = JSON.stringify(fields);
    if (seen.has(key)) throw invalidRules('Duplicate rule.', line);
    if (rules.length === maximumRules) throw invalidRules('At most 200 rules are permitted.', line);
    seen.add(key);
    rules.push(Object.freeze({ line, decision, action, target }));
  }
  return Object.freeze(rules);
}

export function parseRules(text) {
  return parseDecisionRules(text, decisions, 'Decision must be APPROVE or ESCALATE.');
}

// Historical journal validation only. Legacy decisions cannot activate a gate.
export function parseLegacyRules(text) {
  return parseDecisionRules(text, legacyDecisions, 'Legacy decision must be ALLOW, DENY, or REQUIRE_APPROVAL.');
}

/**
 * v0.0.2 auditor assessment only. APPROVE means authenticated scope plus a matching rule;
 * it does not validate material action fields, issue execution permission, or
 * execute anything. ESCALATE awaits a human decision; the gate never issues DENY.
 * Switchboard failures stay outside that decision path as BLOCKED or ERROR.
 * The host must replace this immutable instance to activate changed rules.
 */
export function createGate(options) {
  if (!hasFields(options, ['registry', 'authenticate', 'policy'])) {
    throw new TypeError('Gate requires registry, authenticate, and policy.');
  }
  const { registry, authenticate, policy: inputPolicy } = options;
  if (!hasFields(inputPolicy, ['version', 'text'])
    || !Number.isSafeInteger(inputPolicy.version) || inputPolicy.version < 1) {
    throw invalidRules('Policy requires a positive integer version and text.');
  }
  const rules = parseRules(inputPolicy.text);
  const policy = Object.freeze({
    version: inputPolicy.version,
    hash: createHash('sha256').update(inputPolicy.text, 'utf8').digest('hex'),
  });
  const switchboard = createSwitchboard(registry, { authenticate });

  function evaluate(request) {
    const screening = switchboard.screen(request);
    const result = (status, code, reason, stage, matchedRules = []) => Object.freeze({
      version: '0.0.2', decisionModel: 'approve_escalate_v1', status, code, reason, stage, policy,
      switchboard: screening, matchedRules: Object.freeze(matchedRules), execution: 'NOT_EXECUTED',
    });
    if (screening.status !== 'PASS') {
      return result(screening.status === 'DENY' ? 'BLOCKED' : 'ERROR',
        screening.code, screening.reason, 'SWITCHBOARD');
    }
    const { action, target } = screening.intent;
    if (!actions.has(action)) {
      return result('ERROR', 'UNKNOWN_ACTION', 'Action is not in the Apex action catalog.', 'GATE');
    }
    const matchedRules = rules.filter((rule) => rule.action === action && rule.target === target);
    if (matchedRules.length === 0) {
      return result('ESCALATE', 'NO_MATCHING_RULE',
        'No rule covers this exact action and target. A human decision is required.', 'GATE');
    }
    if (matchedRules.some((rule) => rule.decision === 'ESCALATE')) {
      return result('ESCALATE', 'HUMAN_VERIFICATION_REQUIRED',
        'A matching rule requires a human decision. The assessment remains unresolved.', 'GATE', matchedRules);
    }
    return result('APPROVE', 'RULE_APPROVE',
      'Auditor assessment satisfies authenticated scope and a matching rule. This is not execution permission.', 'GATE', matchedRules);
  }

  return Object.freeze({ evaluate });
}
