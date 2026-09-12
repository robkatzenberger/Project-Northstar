const fs = require("node:fs");
const path = require("node:path");
const { appendAuditLog } = require("./audit");
const { evaluateRules } = require("./engine");
const { evaluateGates, readGatesConfig } = require("./gates");
const { readPolicy } = require("./policy");
const { buildReceipt } = require("./receipt");

const { version } = require(path.join(__dirname, "..", "package.json"));
const defaultGatesPath = path.join(__dirname, "..", "config", "gates.json");

function readJSON(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function evaluateFiles(intentPath, policyPath, options = {}) {
  const intent = normalizeIntent(readJSON(intentPath));
  const policy = readPolicy(policyPath);
  const gatesConfig = readGatesConfig(options.gatesPath || defaultGatesPath);
  return evaluateIntent(intent, policy, { ...options, gatesConfig });
}

function evaluateIntent(intent, policy, options = {}) {
  const normalizedIntent = normalizeIntent(intent);
  const gatesDecision = options.gatesConfig ? evaluateGates(normalizedIntent, options.gatesConfig) : null;
  const policyDecision = evaluateRules(normalizedIntent, policy);
  const decision = combineDecisions(gatesDecision, policyDecision);
  const receipt = buildReceipt({
    intent: normalizedIntent,
    policy,
    decision,
    evaluatedAt: options.evaluatedAt || new Date().toISOString(),
    apexVersion: version
  });

  if (options.logPath) {
    appendAuditLog(options.logPath, receipt);
  }

  return { decision, receipt };
}

function combineDecisions(gatesDecision, policyDecision) {
  if (gatesDecision && gatesDecision.decision === "REQUIRE_APPROVAL") {
    return gatesDecision;
  }

  if (policyDecision && policyDecision.decision === "REQUIRE_APPROVAL") {
    return policyDecision;
  }

  return gatesDecision || policyDecision;
}

function normalizeIntent(intent) {
  const normalized = { ...intent };

  if (typeof normalized.data_classes === "string") {
    normalized.data_classes = normalized.data_classes
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean);
  }

  if (!Array.isArray(normalized.data_classes)) {
    normalized.data_classes = [];
  }

  if (normalized.timestamp === undefined || normalized.timestamp === null || normalized.timestamp === "") {
    normalized.timestamp = Math.floor(Date.now() / 1000);
  } else if (/^-?\d+$/.test(String(normalized.timestamp))) {
    normalized.timestamp = Number(normalized.timestamp);
  }

  if (!normalized.intent_id) {
    normalized.intent_id = `INT-${normalized.timestamp}`;
  }

  if (!normalized.declared_intent) {
    normalized.declared_intent = normalized.action || "";
  }

  if (!normalized.intent_version) {
    normalized.intent_version = "1";
  }

  if (!normalized.log_id) {
    normalized.log_id = normalized.intent_id ? `log_${normalized.intent_id}` : "log_local";
  }

  for (const field of ["intent_id", "actor", "declared_intent", "action", "target", "risk", "intent_version", "log_id"]) {
    if (!normalized[field] && normalized[field] !== 0) {
      throw new Error(`Missing required field: ${field}`);
    }
  }

  return normalized;
}

module.exports = {
  evaluateFiles,
  evaluateIntent,
  normalizeIntent,
  readJSON,
  readPolicy
};
