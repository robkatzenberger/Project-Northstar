const fs = require("node:fs");
const http = require("node:http");
const path = require("node:path");
const { readAuditLog } = require("./audit");
const { evaluateIntent, normalizeIntent, readPolicy } = require("./index");
const { readGatesConfig } = require("./gates");
const { notifyEscalation, readNotificationConfig } = require("./notifications");
const { recordOperatorAction } = require("./operator-action");

const DEFAULT_POLICY_PATH = path.join(__dirname, "..", "examples", "policy.yaml");
const DEFAULT_AUDIT_PATH = path.join(__dirname, "..", "var", "audit.jsonl");
const DEFAULT_DASHBOARD_PATH = path.join(__dirname, "..", "public", "index.html");
const DEFAULT_APP_PATH = path.join(__dirname, "..", "public", "app.js");
const DEFAULT_GATES_PATH = path.join(__dirname, "..", "config", "gates.json");
const DEFAULT_NOTIFICATIONS_PATH = path.join(__dirname, "..", "config", "notifications.json");

function createApp(options = {}) {
  const policyPath = options.policyPath || DEFAULT_POLICY_PATH;
  const auditPath = options.auditPath || DEFAULT_AUDIT_PATH;
  const dashboardPath = options.dashboardPath || DEFAULT_DASHBOARD_PATH;
  const appPath = options.appPath || DEFAULT_APP_PATH;
  const gatesPath = options.gatesPath || DEFAULT_GATES_PATH;
  const notificationsPath = options.notificationsPath || DEFAULT_NOTIFICATIONS_PATH;

  return http.createServer(async (request, response) => {
    try {
      if (request.method === "GET" && request.url === "/") {
        return sendFile(response, dashboardPath);
      }

      if (request.method === "GET" && request.url === "/app.js") {
        return sendFile(response, appPath, "application/javascript; charset=utf-8");
      }

      if (request.method === "GET" && request.url === "/api/audit") {
        return sendJSON(response, 200, { entries: readAuditEntries(auditPath, 20) });
      }

      if (request.method === "POST" && request.url === "/api/evaluate") {
        const payload = await readRequestJSON(request);
        const receipt = await evaluateRequest(payload, policyPath, auditPath, gatesPath, notificationsPath);
        return sendJSON(response, 200, receipt);
      }

      if (request.method === "POST" && request.url === "/api/operator-action") {
        const payload = await readRequestJSON(request);
        const entry = recordOperatorAction(payload, auditPath);
        return sendJSON(response, 200, entry);
      }

      sendJSON(response, 404, { error: "Not found" });
    } catch (error) {
      const statusCode = isClientError(error) ? 400 : 500;
      sendJSON(response, statusCode, { error: error.message });
    }
  });
}

async function evaluateRequest(payload, policyPath, auditPath, gatesPath, notificationsPath) {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    throw new Error("Request body must be a JSON object.");
  }

  const intent = normalizeIntent(payload.intent || payload);
  const policy = readPolicy(policyPath);
  const gatesConfig = readGatesConfig(gatesPath || DEFAULT_GATES_PATH);
  const { receipt } = evaluateIntent(intent, policy, { logPath: auditPath, gatesConfig });
  const notificationsConfig = readNotificationConfig(notificationsPath || DEFAULT_NOTIFICATIONS_PATH);

  if (receipt.decision === "REQUIRE_APPROVAL") {
    const notification = await notifyEscalation(receipt, notificationsConfig, auditPath);
    receipt.notification = {
      channel: notification.channel,
      status: notification.status,
      provider: notification.provider
    };
  }

  return receipt;
}

function readAuditEntries(filePath, limit) {
  return readAuditLog(filePath).slice(-limit).reverse();
}

function readRequestJSON(request) {
  return new Promise((resolve, reject) => {
    let body = "";

    request.on("data", (chunk) => {
      body += chunk;
      if (body.length > 1024 * 1024) {
        reject(new Error("Request body too large."));
        request.destroy();
      }
    });

    request.on("end", () => {
      if (!body.trim()) {
        reject(new Error("Request body is required."));
        return;
      }

      try {
        resolve(JSON.parse(body));
      } catch (error) {
        reject(new Error("Request body must be valid JSON."));
      }
    });

    request.on("error", reject);
  });
}

function sendFile(response, filePath, contentType = "text/html; charset=utf-8") {
  response.writeHead(200, { "Content-Type": contentType });
  response.end(fs.readFileSync(filePath, "utf8"));
}

function sendJSON(response, statusCode, value) {
  response.writeHead(statusCode, { "Content-Type": "application/json; charset=utf-8" });
  response.end(JSON.stringify(value, null, 2));
}

function isClientError(error) {
  return /Request body|Missing required field|valid evaluation receipt|recorded evaluation receipt|only valid for escalated evaluations|does not match the recorded evaluation|Expected the 'evaluate' command|Incorrect number of arguments|Only the --log flag is supported|Operator must use|Unsupported policy expression|Unsupported literal|Action must be 'approve' or 'escalate'|already has a recorded operator outcome/.test(error.message);
}

module.exports = {
  DEFAULT_AUDIT_PATH,
  DEFAULT_APP_PATH,
  DEFAULT_DASHBOARD_PATH,
  DEFAULT_GATES_PATH,
  DEFAULT_NOTIFICATIONS_PATH,
  DEFAULT_POLICY_PATH,
  createApp,
  evaluateRequest,
  normalizeIntent,
  readAuditEntries
};
