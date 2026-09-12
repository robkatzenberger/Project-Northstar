const fs = require("node:fs");
const https = require("node:https");
const querystring = require("node:querystring");
const { appendAuditLog } = require("./audit");

function readNotificationConfig(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

async function notifyEscalation(receipt, notificationConfig, auditPath) {
  const smsConfig = (notificationConfig && notificationConfig.sms) || {};
  const entry = {
    receipt_type: "apex-lite.notification",
    channel: "sms",
    notification_at: new Date().toISOString(),
    receipt_id: receipt.receipt_id,
    intent_id: receipt.original_intent.intent_id || null,
    policy_id: receipt.policy_id || null,
    status: "SKIPPED",
    provider: smsConfig.provider || "log_only",
    recipients: smsConfig.recipients || [],
    message: buildSmsMessage(receipt),
    reason: "SMS notifications are disabled"
  };

  if (!smsConfig.enabled) {
    appendAuditLog(auditPath, entry);
    return entry;
  }

  if (entry.provider === "log_only") {
    entry.status = "SIMULATED";
    entry.reason = "SMS notification recorded in log_only mode";
    appendAuditLog(auditPath, entry);
    return entry;
  }

  if (entry.provider === "twilio") {
    try {
      await sendTwilioSms(entry.message, smsConfig);
      entry.status = "SENT";
      entry.reason = "SMS notification sent through Twilio";
    } catch (error) {
      entry.status = "FAILED";
      entry.reason = error.message;
    }
    appendAuditLog(auditPath, entry);
    return entry;
  }

  entry.reason = `Unsupported SMS provider: ${entry.provider}`;
  appendAuditLog(auditPath, entry);
  return entry;
}

function buildSmsMessage(receipt) {
  return [
    "APEX-Lite escalation",
    `intent: ${receipt.original_intent.intent_id || "unknown"}`,
    `actor: ${receipt.original_intent.actor || "unknown"}`,
    `decision: ${receipt.decision}`,
    `reason: ${receipt.reason}`,
    `receipt: ${receipt.receipt_id}`
  ].join(" | ");
}

function sendTwilioSms(message, smsConfig) {
  return new Promise((resolve, reject) => {
    const twilio = smsConfig.twilio || {};
    const accountSid = process.env[twilio.account_sid_env];
    const authToken = process.env[twilio.auth_token_env];
    const fromNumber = process.env[twilio.from_number_env] || smsConfig.from;
    const toNumber = (smsConfig.recipients || [])[0];

    if (!accountSid || !authToken || !fromNumber || !toNumber) {
      reject(new Error("Twilio SMS requires account SID, auth token, from number, and one recipient."));
      return;
    }

    const body = querystring.stringify({
      From: fromNumber,
      To: toNumber,
      Body: message
    });

    const request = https.request(
      {
        hostname: "api.twilio.com",
        path: `/2010-04-01/Accounts/${accountSid}/Messages.json`,
        method: "POST",
        auth: `${accountSid}:${authToken}`,
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
          "Content-Length": Buffer.byteLength(body)
        }
      },
      (response) => {
        let payload = "";
        response.setEncoding("utf8");
        response.on("data", (chunk) => {
          payload += chunk;
        });
        response.on("end", () => {
          if (response.statusCode >= 200 && response.statusCode < 300) {
            resolve(payload);
            return;
          }

          reject(new Error(`Twilio SMS failed with status ${response.statusCode}`));
        });
      }
    );

    request.on("error", reject);
    request.write(body);
    request.end();
  });
}

module.exports = {
  notifyEscalation,
  readNotificationConfig
};
