import assert from "node:assert/strict";
import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, "..");
const require = createRequire(import.meta.url);
const { evaluateFiles } = require(path.join(repoRoot, "src"));
const { createApp } = require(path.join(repoRoot, "src", "server.js"));
const { readGatesConfig } = require(path.join(repoRoot, "src", "gates.js"));

const policyPath = path.join(repoRoot, "examples", "policy.yaml");
const dashboardPath = path.join(repoRoot, "public", "index.html");
const appPath = path.join(repoRoot, "public", "app.js");
const gatesPath = path.join(repoRoot, "config", "gates.json");
const notificationsPath = path.join(repoRoot, "config", "notifications.json");
const fixedTime = "2026-01-01T00:00:00.000Z";

const cases = [
  {
    name: "high-risk action triggers approval",
    intent: path.join(repoRoot, "examples", "intent-high-risk.json"),
    expectedDecision: "REQUIRE_APPROVAL",
    expectedPolicyId: "rule_01"
  },
  {
    name: "PII action triggers approval",
    intent: path.join(repoRoot, "examples", "intent.json"),
    expectedDecision: "REQUIRE_APPROVAL",
    expectedPolicyId: "email_gate"
  },
  {
    name: "safe action is allowed",
    intent: path.join(repoRoot, "examples", "intent-safe.json"),
    expectedDecision: "ALLOW",
    expectedPolicyId: null
  }
];

for (const testCase of cases) {
  const { decision, receipt } = evaluateFiles(testCase.intent, policyPath, {
    evaluatedAt: fixedTime
  });

  assert.equal(decision.decision, testCase.expectedDecision, testCase.name);
  assert.equal(decision.policy_id, testCase.expectedPolicyId, testCase.name);
  assert.equal(receipt.decision, testCase.expectedDecision, testCase.name);
  assert.equal(receipt.evaluated_at, fixedTime, `${testCase.name} receipt timestamp`);

  console.log(`passed: ${testCase.name}`);
}

const { receipt } = evaluateFiles(path.join(repoRoot, "examples", "intent-safe.json"), policyPath, {
  evaluatedAt: fixedTime
});

assert.match(receipt.receipt_id, /^rcpt_/);
assert.equal("receipt_hash" in receipt, false);
assert.equal("policy_hash" in receipt, false);
assert.equal("intent_hash" in receipt, false);
assert.equal("execution_ref" in receipt, false);
assert.equal(receipt.control_mode, "ALLOW_OR_ESCALATE");
assert.equal(receipt.blocking, false);
console.log("passed: receipt includes plain receipt ids and no-block control mode");

const gatesConfig = readGatesConfig(gatesPath);
assert.equal(gatesConfig.intent_schema.required_fields.includes("declared_intent"), true);
assert.equal(gatesConfig.intent_schema.required_fields.includes("intent_version"), true);
assert.equal(gatesConfig.intent_schema.required_fields.includes("log_id"), true);
console.log("passed: gates config includes required request fields");

const folderIntent = {
  intent_id: "INT-FOLDER-001",
  actor: "agent_33",
  declared_intent: "delete shared folder export",
  action: "manage_folder",
  target: "shared_folder",
  risk: "medium",
  timestamp: 1730000400,
  intent_version: "1",
  log_id: "LOG-FOLDER-001",
  data_classes: []
};

const { receipt: folderReceipt } = evaluateFiles(path.join(repoRoot, "examples", "intent-safe.json"), policyPath, {
  evaluatedAt: fixedTime,
  gatesPath
});
assert.equal(folderReceipt.original_intent.intent_version, "1");
assert.equal(folderReceipt.original_intent.intent_id, "INT-1730000300");
assert.equal(folderReceipt.original_intent.log_id, "log_INT-1730000300");
assert.equal(folderReceipt.original_intent.declared_intent, "summarize_report");

const { evaluateIntent, readPolicy } = require(path.join(repoRoot, "src"));
const { receipt: folderGateReceipt } = evaluateIntent(folderIntent, readPolicy(policyPath), {
  evaluatedAt: fixedTime,
  gatesConfig
});
assert.equal(folderGateReceipt.decision, "REQUIRE_APPROVAL");
assert.equal(folderGateReceipt.policy_id, "folder_gate");
console.log("passed: folder gate keywords trigger escalation");

const emailAllowIntent = {
  intent_id: "INT-EMAIL-ALLOW-001",
  actor: "agent_44",
  declared_intent: "read internal draft email summary",
  action: "send_email",
  target: "internal_workspace",
  risk: "low",
  timestamp: 1730000500,
  intent_version: "1",
  log_id: "LOG-EMAIL-ALLOW-001",
  data_classes: []
};

const { receipt: emailAllowReceipt } = evaluateIntent(emailAllowIntent, readPolicy(policyPath), {
  evaluatedAt: fixedTime,
  gatesConfig
});
assert.equal(emailAllowReceipt.decision, "ALLOW");
assert.equal(emailAllowReceipt.policy_id, "email_gate");
console.log("passed: email gate keywords can allow low-risk requests");

const gatedAllowButPolicyEscalateIntent = {
  intent_id: "INT-GATE-POLICY-001",
  actor: "agent_55",
  declared_intent: "send internal draft email summary with pii",
  action: "send_email",
  target: "internal_workspace",
  risk: "low",
  timestamp: 1730000550,
  intent_version: "1",
  log_id: "LOG-GATE-POLICY-001",
  data_classes: ["PII"]
};

const allowFirstGatesConfig = {
  ...gatesConfig,
  gates: gatesConfig.gates.map((gate) => {
    if (gate.id !== "email_gate") {
      return gate;
    }

    return {
      ...gate,
      escalate_keywords: gate.escalate_keywords.filter((keyword) => keyword !== "pii")
    };
  })
};

const { receipt: gatedAllowPolicyEscalateReceipt } = evaluateIntent(gatedAllowButPolicyEscalateIntent, readPolicy(policyPath), {
  evaluatedAt: fixedTime,
  gatesConfig: allowFirstGatesConfig
});
assert.equal(gatedAllowPolicyEscalateReceipt.decision, "REQUIRE_APPROVAL");
assert.equal(gatedAllowPolicyEscalateReceipt.policy_id, "rule_02");
console.log("passed: yaml approval rules override gate allow matches");

const folderSubstringIntent = {
  intent_id: "INT-FOLDER-SUBSTRING-001",
  actor: "agent_56",
  declared_intent: "review thread notes for folder change",
  action: "manage_folder",
  target: "shared_drive",
  risk: "low",
  timestamp: 1730000560,
  intent_version: "1",
  log_id: "LOG-FOLDER-SUBSTRING-001",
  data_classes: []
};

const { receipt: folderSubstringReceipt } = evaluateIntent(folderSubstringIntent, readPolicy(policyPath), {
  evaluatedAt: fixedTime,
  gatesConfig
});
assert.equal(folderSubstringReceipt.decision, "ALLOW");
assert.equal(folderSubstringReceipt.policy_id, null);
console.log("passed: gate keywords do not match arbitrary substrings");

const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "apex-lite-"));
const logPath = path.join(tempDir, "audit.jsonl");

evaluateFiles(path.join(repoRoot, "examples", "intent-safe.json"), policyPath, {
  evaluatedAt: fixedTime,
  logPath
});

const lines = fs.readFileSync(logPath, "utf8").trim().split("\n");
assert.equal(lines.length, 1);

const loggedReceipt = JSON.parse(lines[0]);
assert.equal(loggedReceipt.receipt_type, "apex-lite.receipt");
assert.equal(loggedReceipt.decision, "ALLOW");
console.log("passed: audit log appends one JSON line per evaluation");

const auditPath = path.join(tempDir, "server-audit.jsonl");
const notificationConfigPath = path.join(tempDir, "notifications.json");
fs.writeFileSync(notificationConfigPath, JSON.stringify({
  config_version: "1",
  sms: {
    enabled: true,
    provider: "log_only",
    from: "APEX-Lite",
    recipients: ["+15555550123"]
  }
}, null, 2));

const server = createApp({ policyPath, auditPath, dashboardPath, appPath, gatesPath, notificationsPath: notificationConfigPath });
await new Promise((resolve) => server.listen(0, resolve));
const port = server.address().port;

try {
  const evaluateResponse = await requestJSON(port, "/api/evaluate", {
    method: "POST",
    body: {
      intent: {
        intent_id: "INT-API-001",
        actor: "agent_42",
        declared_intent: "send external customer email with pii attachment",
        action: "send_email",
        target: "external_user",
        risk: "medium",
        intent_version: "1",
        log_id: "LOG-API-001",
        data_classes: ["PII"],
        timestamp: 1730000000
      }
    }
  });

  assert.equal(evaluateResponse.statusCode, 200);
  assert.equal(evaluateResponse.body.decision, "REQUIRE_APPROVAL");
  assert.equal(evaluateResponse.body.policy_id, "email_gate");
  assert.equal(evaluateResponse.body.reward_signal, "TRANSPARENCY_REWARDED");
  assert.equal(evaluateResponse.body.blocking, false);
  assert.equal(evaluateResponse.body.original_intent.log_id, "LOG-API-001");
  assert.equal(evaluateResponse.body.notification.status, "SIMULATED");
  assert.match(evaluateResponse.body.receipt_id, /^rcpt_/);
  assert.equal("receipt_hash" in evaluateResponse.body, false);
  console.log("passed: api evaluate returns a real receipt with rewarded escalation");

  const auditResponse = await requestJSON(port, "/api/audit");
  assert.equal(auditResponse.statusCode, 200);
  assert.equal(auditResponse.body.entries.length, 2);
  assert.equal(auditResponse.body.entries[1].original_intent.intent_id, "INT-API-001");
  assert.equal(auditResponse.body.entries[0].receipt_type, "apex-lite.notification");
  assert.equal(auditResponse.body.entries[0].status, "SIMULATED");
  console.log("passed: api audit returns appended receipts and sms notification events");

  const operatorResponse = await requestJSON(port, "/api/operator-action", {
    method: "POST",
    body: {
      action: "approve",
      receipt: evaluateResponse.body,
      operator: "ops_local"
    }
  });

  assert.equal(operatorResponse.statusCode, 200);
  assert.equal(operatorResponse.body.action, "APPROVE");
  assert.equal(operatorResponse.body.intent_id, "INT-API-001");
  assert.equal(operatorResponse.body.operator, "ops_local");
  assert.equal(operatorResponse.body.receipt_id, evaluateResponse.body.receipt_id);
  assert.equal(operatorResponse.body.outcome, "ALLOW");
  console.log("passed: operator actions are recorded as audit events");

  const auditAfterAction = await requestJSON(port, "/api/audit");
  assert.equal(auditAfterAction.statusCode, 200);
  assert.equal(auditAfterAction.body.entries.length, 3);
  assert.equal(auditAfterAction.body.entries[0].receipt_type, "apex-lite.operator_action");
  assert.equal(auditAfterAction.body.entries[0].operator, "ops_local");
  assert.equal(auditAfterAction.body.entries[0].outcome, "ALLOW");
  console.log("passed: audit api returns operator action events");

  const duplicateOperatorResponse = await requestJSON(port, "/api/operator-action", {
    method: "POST",
    body: {
      action: "approve",
      receipt: evaluateResponse.body,
      operator: "ops_local"
    }
  });

  assert.equal(duplicateOperatorResponse.statusCode, 400);
  assert.match(duplicateOperatorResponse.body.error, /already has a recorded operator outcome/i);
  console.log("passed: duplicate operator outcomes are rejected");

  const forgedOperatorResponse = await requestJSON(port, "/api/operator-action", {
    method: "POST",
    body: {
      action: "approve",
      receipt: {
        ...evaluateResponse.body,
        receipt_id: "rcpt_fake_receipt"
      }
    }
  });

  assert.equal(forgedOperatorResponse.statusCode, 400);
  assert.match(forgedOperatorResponse.body.error, /recorded evaluation receipt/i);
  console.log("passed: forged operator actions are rejected");

  const allowEvaluateResponse = await requestJSON(port, "/api/evaluate", {
    method: "POST",
    body: {
      intent: {
        intent_id: "INT-API-ALLOW",
        actor: "agent_12",
        declared_intent: "read internal draft email summary",
        action: "summarize_report",
        target: "internal_workspace",
        risk: "low",
        intent_version: "1",
        log_id: "LOG-API-ALLOW",
        data_classes: [],
        timestamp: 1730000300
      }
    }
  });

  assert.equal(allowEvaluateResponse.statusCode, 200);
  assert.equal(allowEvaluateResponse.body.decision, "ALLOW");

  const invalidAllowAction = await requestJSON(port, "/api/operator-action", {
    method: "POST",
    body: {
      action: "approve",
      receipt: allowEvaluateResponse.body
    }
  });

  assert.equal(invalidAllowAction.statusCode, 400);
  assert.match(invalidAllowAction.body.error, /only valid for escalated evaluations/i);
  console.log("passed: non-escalated receipts cannot be resolved as operator actions");

  const freshEscalationResponse = await requestJSON(port, "/api/evaluate", {
    method: "POST",
    body: {
      intent: {
        intent_id: "INT-API-BAD-OP",
        actor: "agent_99",
        declared_intent: "send external customer email with pii attachment",
        action: "send_email",
        target: "external_user",
        risk: "medium",
        intent_version: "1",
        log_id: "LOG-BAD-OP",
        data_classes: ["PII"],
        timestamp: 1730000700
      }
    }
  });

  assert.equal(freshEscalationResponse.statusCode, 200);
  assert.equal(freshEscalationResponse.body.decision, "REQUIRE_APPROVAL");

  const invalidOperatorName = await requestJSON(port, "/api/operator-action", {
    method: "POST",
    body: {
      action: "approve",
      receipt: freshEscalationResponse.body,
      operator: "bad operator!"
    }
  });

  assert.equal(invalidOperatorName.statusCode, 400);
  assert.match(invalidOperatorName.body.error, /Operator must use 1-64 characters/i);
  console.log("passed: invalid operator identifiers are rejected");

  const dashboardResponse = await requestText(port, "/");
  assert.equal(dashboardResponse.statusCode, 200);
  assert.match(dashboardResponse.body, /intent input/i);
  assert.match(dashboardResponse.body, /declared_intent/i);
  assert.match(dashboardResponse.body, /log_id/i);
  assert.match(dashboardResponse.body, /Policy Gate Notice/i);
  assert.doesNotMatch(dashboardResponse.body, /Math\.random/);
  console.log("passed: dashboard is served without simulation logic");

  const appResponse = await requestText(port, "/app.js");
  assert.equal(appResponse.statusCode, 200);
  assert.match(appResponse.body, /fetch\(\"\/api\/evaluate\"/);
  assert.match(appResponse.body, /transparency rewarded/);
  assert.match(appResponse.body, /SMS notification/);
  assert.doesNotMatch(appResponse.body, /DENY/);
  console.log("passed: dashboard app is served");
} finally {
  server.close();
}

const failedNotificationAuditPath = path.join(tempDir, "failed-notification-audit.jsonl");
const failingNotificationConfigPath = path.join(tempDir, "failing-notifications.json");
fs.writeFileSync(failingNotificationConfigPath, JSON.stringify({
  config_version: "1",
  sms: {
    enabled: true,
    provider: "twilio",
    recipients: ["+15555550123"],
    twilio: {
      account_sid_env: "MISSING_TWILIO_SID",
      auth_token_env: "MISSING_TWILIO_TOKEN",
      from_number_env: "MISSING_TWILIO_FROM"
    }
  }
}, null, 2));

const failedNotificationServer = createApp({
  policyPath,
  auditPath: failedNotificationAuditPath,
  dashboardPath,
  appPath,
  gatesPath,
  notificationsPath: failingNotificationConfigPath
});

await new Promise((resolve) => failedNotificationServer.listen(0, resolve));
const failedNotificationPort = failedNotificationServer.address().port;

try {
  const failedNotificationResponse = await requestJSON(failedNotificationPort, "/api/evaluate", {
    method: "POST",
    body: {
      intent: {
        intent_id: "INT-API-FAIL-SMS",
        actor: "agent_42",
        declared_intent: "send external customer email with pii attachment",
        action: "send_email",
        target: "external_user",
        risk: "medium",
        intent_version: "1",
        log_id: "LOG-FAIL-SMS",
        data_classes: ["PII"],
        timestamp: 1730000600
      }
    }
  });

  assert.equal(failedNotificationResponse.statusCode, 200);
  assert.equal(failedNotificationResponse.body.decision, "REQUIRE_APPROVAL");
  assert.equal(failedNotificationResponse.body.notification.status, "FAILED");
  console.log("passed: notification delivery failure does not break evaluation");
} finally {
  failedNotificationServer.close();
}

console.log("All APEX Lite tests passed.");

function requestJSON(port, requestPath, options = {}) {
  return new Promise((resolve, reject) => {
    const request = http.request(
      {
        host: "127.0.0.1",
        port,
        path: requestPath,
        method: options.method || "GET",
        headers: {
          "Content-Type": "application/json"
        }
      },
      (response) => {
        let body = "";
        response.setEncoding("utf8");
        response.on("data", (chunk) => {
          body += chunk;
        });
        response.on("end", () => {
          resolve({
            statusCode: response.statusCode,
            body: body ? JSON.parse(body) : null
          });
        });
      }
    );

    request.on("error", reject);

    if (options.body) {
      request.write(JSON.stringify(options.body));
    }

    request.end();
  });
}

function requestText(port, requestPath) {
  return new Promise((resolve, reject) => {
    const request = http.request(
      {
        host: "127.0.0.1",
        port,
        path: requestPath,
        method: "GET"
      },
      (response) => {
        let body = "";
        response.setEncoding("utf8");
        response.on("data", (chunk) => {
          body += chunk;
        });
        response.on("end", () => {
          resolve({
            statusCode: response.statusCode,
            body
          });
        });
      }
    );

    request.on("error", reject);
    request.end();
  });
}
