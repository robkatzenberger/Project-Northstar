#!/usr/bin/env node
/**
 * Glass CLI — local trust-layer checkpoint
 *
 * Usage:
 *   glass evaluate <intent.json> <policy.yaml> [--switchboard config/switchboard.json] [--log var/audit.jsonl]
 *   glass switchboard <agent_id> [--switchboard config/switchboard.json]
 *   glass approve <receipt_id> --operator <id> --log var/audit.jsonl [--note "..."]
 *   glass reject  <receipt_id> --operator <id> --log var/audit.jsonl [--note "..."]
 *   glass execute <receipt_id> --executor <id> --status EXECUTED|BLOCKED|FAILED --log var/audit.jsonl
 *   glass chain <receipt_id> --log var/audit.jsonl
 *   glass incident <receipt_id> --log var/audit.jsonl --why "..." [--observed action]
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  createPrismSignal,
  toEvaluationIntent,
  evaluateIntent,
  resolveEscalation,
  recordExecution,
  readPolicyFile,
  readAudit,
  reportFromAuditFile,
  buildChain,
  loadSwitchboard,
  lookupPrincipal,
  resolveAuthorizationFromAudit
} from "../src/index.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const defaultSwitchboard = path.resolve(__dirname, "../config/switchboard.json");

function print(obj) {
  console.log(JSON.stringify(obj, null, 2));
}

function die(msg, code = 1) {
  console.error(msg);
  process.exit(code);
}

function parseArgs(argv) {
  const args = { _: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--log") args.log = argv[++i];
    else if (a === "--operator") args.operator = argv[++i];
    else if (a === "--executor") args.executor = argv[++i];
    else if (a === "--status") args.status = argv[++i];
    else if (a === "--note") args.note = argv[++i];
    else if (a === "--why") args.why = argv[++i];
    else if (a === "--observed") args.observed = argv[++i];
    else if (a === "--severity") args.severity = argv[++i];
    else if (a === "--summary") args.summary = argv[++i];
    else if (a === "--switchboard") args.switchboard = argv[++i];
    else if (a === "--no-switchboard") args.noSwitchboard = true;
    else if (a.startsWith("--")) die(`Unknown flag: ${a}`);
    else args._.push(a);
  }
  return args;
}

function resolveSwitchboard(args) {
  if (args.noSwitchboard) return null;
  const p = path.resolve(args.switchboard || defaultSwitchboard);
  if (!fs.existsSync(p)) {
    if (args.switchboard) die(`Switchboard config not found: ${p}`);
    return null;
  }
  return loadSwitchboard(p);
}

function loadIntent(filePath) {
  const raw = JSON.parse(fs.readFileSync(filePath, "utf8"));

  // Full Prism signal already minted
  if (raw.prism_id && raw.intent_summary && raw.prism_version) {
    return toEvaluationIntent(raw);
  }

  // Example-style input: agent + intent_summary (+ optional glass fields)
  if (raw.agent && raw.intent_summary) {
    return toEvaluationIntent(createPrismSignal(raw));
  }

  // Flat APEX-style intent
  return {
    intent_id: raw.intent_id || raw.prism_id || `intent_${Date.now()}`,
    prism_id: raw.prism_id || raw.intent_id || null,
    actor: raw.actor || raw.agent,
    actor_type: raw.actor_type || "machine",
    declared_intent: raw.declared_intent || raw.intent_summary || `${raw.action} → ${raw.target}`,
    intent_summary: raw.intent_summary || raw.declared_intent,
    action: raw.action ?? null,
    target: raw.target ?? null,
    risk: raw.risk || "low",
    data_classes: raw.data_classes || [],
    context: raw.context || {},
    timestamp: raw.timestamp || new Date().toISOString()
  };
}

function findDecision(auditPath, receiptId) {
  const records = readAudit(auditPath);
  const decision = records.find(
    (r) =>
      (r.record_type === "tlpx.decision" || r.record_type === "glass.decision") &&
      r.receipt_id === receiptId
  );
  if (!decision) die(`No tlpx.decision for receipt_id=${receiptId}`);
  return { records, decision };
}

function findLatestOperator(records, receiptId) {
  return records
    .filter(
      (r) =>
        (r.record_type === "tlpx.operator_action" ||
          r.record_type === "glass.operator_action") &&
        r.receipt_id === receiptId
    )
    .at(-1);
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = args._[0];

  if (!cmd || cmd === "help" || cmd === "--help") {
    console.log(`Glass / TL-PX CLI

  evaluate <intent.json> <policy.yaml> --log PATH [--switchboard PATH] [--no-switchboard]
  switchboard <agent_id> [--switchboard PATH]
  approve  <receipt_id> --operator ID --log PATH [--note TEXT]
  reject   <receipt_id> --operator ID --log PATH [--note TEXT]
  execute  <receipt_id> --executor ID --status EXECUTED|BLOCKED|FAILED --log PATH [--summary TEXT]
  auth     <receipt_id> --log PATH
  chain    <receipt_id> --log PATH
  incident <receipt_id> --log PATH --why TEXT [--observed ACTION] [--severity low|medium|high]

Air-gapped: --log (audit JSONL) is required for evaluate/approve/execute.
Default switchboard: config/switchboard.json (if present)
`);
    process.exit(0);
  }

  if (cmd === "switchboard") {
    const agentId = args._[1];
    if (!agentId) die("Usage: glass switchboard <agent_id> [--switchboard PATH]");
    const sb = resolveSwitchboard({ ...args, noSwitchboard: false, switchboard: args.switchboard || defaultSwitchboard });
    if (!sb) die("No switchboard config loaded");
    const { status, principal } = lookupPrincipal(sb, agentId);
    print({
      switchboard_id: sb.switchboard_id,
      lookup: status,
      principal,
      thresholds: sb.thresholds,
      unknown_agent_policy: sb.unknown_agent_policy
    });
    return;
  }

  if (cmd === "evaluate") {
    const intentPath = args._[1];
    const policyPath = args._[2];
    if (!intentPath || !policyPath || !args.log) {
      die("Usage: glass evaluate <intent.json> <policy.yaml> --log PATH [--switchboard PATH]");
    }
    const intent = loadIntent(path.resolve(intentPath));
    const policy = readPolicyFile(path.resolve(policyPath));
    const switchboard = resolveSwitchboard(args);
    const decision = evaluateIntent(intent, policy, {
      auditPath: path.resolve(args.log),
      switchboard
    });
    print(decision);
    return;
  }

  if (cmd === "auth") {
    const receiptId = args._[1];
    if (!receiptId || !args.log) die("Usage: glass auth <receipt_id> --log PATH");
    print(resolveAuthorizationFromAudit(path.resolve(args.log), receiptId));
    return;
  }

  if (cmd === "approve" || cmd === "reject") {
    const receiptId = args._[1];
    if (!receiptId || !args.operator || !args.log) {
      die(`Usage: glass ${cmd} <receipt_id> --operator ID --log PATH`);
    }
    const { decision } = findDecision(path.resolve(args.log), receiptId);
    const action = resolveEscalation(
      decision,
      {
        operator_id: args.operator,
        outcome: cmd === "approve" ? "APPROVE" : "REJECT",
        note: args.note
      },
      { auditPath: path.resolve(args.log) }
    );
    print(action);
    return;
  }

  if (cmd === "execute") {
    const receiptId = args._[1];
    if (!receiptId || !args.executor || !args.status || !args.log) {
      die("Usage: glass execute <receipt_id> --executor ID --status STATUS --log PATH");
    }
    const logPath = path.resolve(args.log);
    // Chain-verified: ignores any client-forged status; audit is source of truth
    const execution = recordExecution(
      {
        receipt_id: receiptId,
        executor_id: args.executor,
        executor_type: "machine",
        status: args.status,
        result_summary: args.summary || null
      },
      { auditPath: logPath }
    );
    print(execution);
    return;
  }

  if (cmd === "chain") {
    const receiptId = args._[1];
    if (!receiptId || !args.log) die("Usage: glass chain <receipt_id> --log PATH");
    const records = readAudit(path.resolve(args.log));
    print(buildChain(records, receiptId));
    return;
  }

  if (cmd === "incident") {
    const receiptId = args._[1];
    if (!receiptId || !args.log || !args.why) {
      die("Usage: glass incident <receipt_id> --log PATH --why TEXT [--observed ACTION]");
    }
    const { chain, report } = reportFromAuditFile(
      path.resolve(args.log),
      receiptId,
      {
        what_went_wrong: args.why,
        observed_action: args.observed,
        severity: args.severity || "high"
      },
      { persist: true }
    );
    print({ chain, report });
    return;
  }

  die(`Unknown command: ${cmd}`);
}

main().catch((err) => {
  console.error(err.message || err);
  process.exit(1);
});
