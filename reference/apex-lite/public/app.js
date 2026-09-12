const state = {
  counts: { total: 0, allow: 0, esc: 0, human: 0 },
  feedItems: 0,
  escItems: [],
  barHistory: [],
  paused: false,
  evalTimestamps: []
};

function ts() {
  return new Date().toTimeString().slice(0, 8);
}

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function updateClock() {
  document.getElementById("clock").textContent = ts();
}

function handlePauseRequest() {
  document.getElementById("confirm-bar").classList.add("show");
  document.getElementById("btn-stop").style.display = "none";
}

function cancelPause() {
  document.getElementById("confirm-bar").classList.remove("show");
  document.getElementById("btn-stop").style.display = "";
}

function confirmPause() {
  state.paused = true;
  document.getElementById("confirm-bar").classList.remove("show");
  document.getElementById("btn-resume").style.display = "";
  setBadgePaused();
  updateEvaluateState();
  addLog("warn", "manual intake paused by operator");
}

function handleResume() {
  state.paused = false;
  document.getElementById("btn-resume").style.display = "none";
  document.getElementById("btn-stop").style.display = "";
  setBadgeLive();
  updateEvaluateState();
  addLog("ok", "manual intake resumed by operator");
}

function setBadgeLive() {
  const badge = document.getElementById("status-badge");
  badge.className = "badge-live";
  badge.innerHTML = '<span class="pulse"></span> live';
}

function setBadgePaused() {
  const badge = document.getElementById("status-badge");
  badge.className = "badge-paused";
  badge.textContent = "paused";
}

function addLog(type, message) {
  const log = document.getElementById("sys-log");
  const line = document.createElement("div");
  const className = type === "ok" ? "log-ok" : type === "warn" ? "log-warn" : "log-human";
  line.className = "log-line";
  line.innerHTML = '<span class="log-ts">' + ts() + '</span><span class="log-msg ' + className + '">' + escapeHtml(message) + "</span>";
  log.insertBefore(line, log.firstChild);
  if (log.children.length > 60) {
    log.removeChild(log.lastChild);
  }
}

function updateMetrics() {
  const total = state.counts.total || 1;
  document.getElementById("m-total").textContent = state.counts.total;
  document.getElementById("m-allow").textContent = state.counts.allow;
  document.getElementById("m-esc").textContent = state.counts.esc;
  document.getElementById("m-human").textContent = state.counts.human;
  document.getElementById("m-allow-pct").textContent = Math.round((state.counts.allow / total) * 100) + "% of events";
  document.getElementById("m-esc-pct").textContent = Math.round((state.counts.esc / total) * 100) + "% to review";
}

function renderBars() {
  const chart = document.getElementById("bar-chart");
  chart.innerHTML = "";
  const history = state.barHistory.slice(-20);
  while (history.length < 20) {
    history.unshift("allow");
  }

  history.forEach((item) => {
    const bar = document.createElement("div");
    bar.className = "chart-bar " + (item === "allow" ? "cb-allow" : item === "escalate" ? "cb-esc" : "cb-human");
    bar.style.height = item === "allow" ? "14px" : item === "escalate" ? "26px" : "18px";
    chart.appendChild(bar);
  });
}

function updateRate() {
  const cutoff = Date.now() - 60000;
  state.evalTimestamps = state.evalTimestamps.filter((value) => value >= cutoff);
  document.getElementById("pg-rate").textContent = state.evalTimestamps.length + " evals/min";
}

function updateEvaluateState(message) {
  const button = document.getElementById("evaluate-btn");
  button.disabled = state.paused;
  document.getElementById("form-status").textContent = message || (state.paused ? "evaluation paused by operator" : "ready to evaluate");
  document.getElementById("btn-stop").style.display = state.paused ? "none" : "";
}

function verdictClass(decision) {
  if (decision === "SIMULATED" || decision === "SENT") {
    return "v-human";
  }

  if (decision === "APPROVE") {
    return "v-allow";
  }

  if (decision === "ESCALATE") {
    return "v-human";
  }

  return decision === "ALLOW" ? "v-allow" : "v-esc";
}

function receiptDecisionLabel(decision) {
  if (decision === "SIMULATED" || decision === "SENT") {
    return "sms";
  }

  if (decision === "APPROVE") {
    return "approve";
  }

  if (decision === "ESCALATE") {
    return "human";
  }

  return decision === "ALLOW" ? "allow" : "escalate";
}

function entryTime(entry) {
  return entry.evaluated_at || entry.action_at || entry.notification_at || "";
}

function entryIntentId(entry) {
  if (entry.intent_id) {
    return entry.intent_id;
  }

  return ((entry.original_intent || {}).intent_id) || "no intent id";
}

function entrySummary(entry) {
  if (entry.receipt_type === "apex-lite.operator_action") {
    const operator = entry.operator || "local_operator";
    return entry.action === "APPROVE" ? operator + " approved escalated intent" : operator + " rewarded transparent escalation with human review";
  }

  if (entry.receipt_type === "apex-lite.notification") {
    return "SMS notification " + String(entry.status || "unknown").toLowerCase() + " for escalation";
  }

  return entry.reason || "No reason recorded";
}

function renderReceipt(receipt) {
  const badge = document.getElementById("receipt-badge");
  badge.className = "verdict " + verdictClass(receipt.decision);
  badge.textContent = receiptDecisionLabel(receipt.decision);
  document.getElementById("receipt-meta").innerHTML =
    '<div class="meta-row"><span class="meta-key">decision</span><span class="meta-value">' + escapeHtml(receipt.decision) + '</span></div>' +
    '<div class="meta-row"><span class="meta-key">reason</span><span class="meta-value">' + escapeHtml(receipt.reason) + '</span></div>' +
    '<div class="meta-row"><span class="meta-key">receipt_id</span><span class="meta-value">' + escapeHtml(receipt.receipt_id || "none") + '</span></div>' +
    '<div class="meta-row"><span class="meta-key">log_id</span><span class="meta-value">' + escapeHtml(receipt.original_intent.log_id || "none") + '</span></div>' +
    '<div class="meta-row"><span class="meta-key">version</span><span class="meta-value">' + escapeHtml(receipt.original_intent.intent_version || "none") + '</span></div>' +
    '<div class="meta-row"><span class="meta-key">control_mode</span><span class="meta-value">' + escapeHtml(receipt.control_mode) + '</span></div>' +
    '<div class="meta-row"><span class="meta-key">blocking</span><span class="meta-value">' + escapeHtml(String(receipt.blocking)) + '</span></div>' +
    '<div class="meta-row"><span class="meta-key">policy_id</span><span class="meta-value">' + escapeHtml(receipt.policy_id || "none") + '</span></div>' +
    '<div class="meta-row"><span class="meta-key">reward_signal</span><span class="meta-value">' + escapeHtml(receipt.reward_signal || "none") + '</span></div>' +
    '<div class="meta-row"><span class="meta-key">evaluated_at</span><span class="meta-value">' + escapeHtml(receipt.evaluated_at) + "</span></div>";
  document.getElementById("receipt-json").textContent = JSON.stringify(receipt, null, 2);
}

function addFeedItem(receipt) {
  const feed = document.getElementById("feed");
  const intent = receipt.original_intent || {};
  const row = document.createElement("div");
  row.className = "txn";
  row.innerHTML =
    '<span class="txn-time">' + escapeHtml((receipt.evaluated_at || "").slice(11, 19) || ts()) + "</span>" +
    '<div class="txn-col"><div class="txn-row1">' + escapeHtml(intent.actor || "unknown") + '<span class="actor-badge ab-agent">agent</span></div><div class="txn-row2">' + escapeHtml(intent.intent_id || "no intent id") + "</div></div>" +
    '<div class="txn-col"><div class="txn-row1">' + escapeHtml(intent.action || "unknown action") + '</div><div class="txn-row2">risk: ' + escapeHtml(intent.risk || "unknown") + "</div></div>" +
    '<div class="txn-col"><div class="txn-row1">' + escapeHtml(intent.target || "unknown target") + '</div><div class="txn-row2">' + escapeHtml(receipt.reason) + "</div></div>" +
    '<div class="txn-verdict"><span class="verdict ' + verdictClass(receipt.decision) + '">' + escapeHtml(receiptDecisionLabel(receipt.decision)) + "</span></div>";
  feed.insertBefore(row, feed.firstChild);
  state.feedItems += 1;
  if (feed.children.length > 50) {
    feed.removeChild(feed.lastChild);
  }
  document.getElementById("feed-count").textContent = state.feedItems + " events";
}

function addEscalation(receipt) {
  state.escItems.unshift({ id: receipt.receipt_id || receipt.original_intent.intent_id, receipt });
  renderEscalations();
}

function renderEscalations() {
  const queue = document.getElementById("esc-queue");
  queue.innerHTML = "";
  if (!state.escItems.length) {
    queue.innerHTML = '<div class="empty">No pending escalations.</div>';
  } else {
    state.escItems.forEach((entry) => {
      const intent = entry.receipt.original_intent || {};
      const item = document.createElement("div");
      item.className = "esc-item";
      item.innerHTML =
        '<div class="esc-header"><span class="esc-agent">' + escapeHtml(intent.actor || "unknown") + ' <span style="color:var(--muted);font-size:10px">' + escapeHtml(intent.intent_id || "no intent id") + '</span></span><span class="esc-timer">' + escapeHtml((entry.receipt.evaluated_at || "").replace("T", " ").replace("Z", "")) + "</span></div>" +
        '<div class="esc-meta">intent: ' + escapeHtml(intent.action || "unknown") + "</div>" +
        '<div class="esc-parties">target: ' + escapeHtml(intent.target || "unknown") + ' | risk: ' + escapeHtml(intent.risk || "unknown") + "</div>" +
        '<div class="esc-reason">reason: ' + escapeHtml(entry.receipt.reason) + ' | escalation is rewarded, not blocked</div>' +
        '<div class="esc-actions"><button class="esc-btn esc-approve" type="button" data-action="approve" data-id="' + escapeHtml(entry.id) + '">approve</button><button class="esc-btn esc-human" type="button" data-action="escalate" data-id="' + escapeHtml(entry.id) + '">escalate</button></div>';
      queue.appendChild(item);
    });
  }
  document.getElementById("esc-count").textContent = state.escItems.length + " pending";
}

async function resolveEscalation(id, action) {
  const entry = state.escItems.find((item) => item.id === id);
  if (!entry) {
    return;
  }

  const label = action === "approve" ? "approved for execution" : "rewarded transparent escalation with human review";

  try {
    const response = await fetch("/api/operator-action", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        action,
        receipt: entry.receipt,
        operator: currentOperator()
      })
    });
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || "Operator action failed");
    }

    await refreshAudit();
  } catch (error) {
    addLog("warn", error.message);
    return;
  }

  state.escItems = state.escItems.filter((item) => item.id !== id);
  state.counts.human += 1;
  state.barHistory.push("human");
  updateMetrics();
  renderBars();
  renderEscalations();
  addLog(action === "approve" ? "ok" : "human", currentOperator() + " resolved " + entry.receipt.original_intent.intent_id + ": " + label);
}

function renderAudit(entries) {
  const list = document.getElementById("audit-list");
  if (!entries.length) {
    list.innerHTML = '<div class="empty">Audit log is empty.</div>';
    return;
  }

  list.innerHTML = "";
  entries.forEach((entry) => {
    const item = document.createElement("div");
    item.className = "audit-item";
    item.innerHTML =
      '<div class="audit-top"><span class="verdict ' + verdictClass(entry.decision || entry.action || entry.status) + '">' + escapeHtml(receiptDecisionLabel(entry.decision || entry.action || entry.status)) + "</span><span>" + escapeHtml(entryTime(entry)) + "</span></div>" +
      '<div class="txn-row2">' + escapeHtml(entryIntentId(entry)) + " - " + escapeHtml(entrySummary(entry)) + "</div>" +
      '<div class="audit-hashes">receipt: ' + escapeHtml(entry.receipt_id || "n/a") + "<br>policy: " + escapeHtml(entry.policy_id || "none") + "</div>";
    list.appendChild(item);
  });
}

async function refreshAudit() {
  try {
    const response = await fetch("/api/audit");
    if (!response.ok) {
      throw new Error("Audit fetch failed");
    }
    const data = await response.json();
    renderAudit(data.entries || []);
  } catch (error) {
    document.getElementById("api-status").textContent = "DEGRADED";
    addLog("warn", "unable to refresh audit log");
  }
}

function formIntent() {
  const form = document.getElementById("intent-form");
  return {
    intent_id: form.intent_id.value.trim(),
    actor: form.actor.value.trim(),
    declared_intent: form.declared_intent.value.trim(),
    action: form.action.value.trim(),
    target: form.target.value.trim(),
    risk: form.risk.value.trim(),
    intent_version: form.intent_version.value.trim(),
    log_id: form.log_id.value.trim(),
    data_classes: form.data_classes.value.split(",").map((item) => item.trim()).filter(Boolean),
    timestamp: form.timestamp.value.trim()
  };
}

function currentOperator() {
  return document.getElementById("operator").value.trim() || "local_operator";
}

async function evaluateIntent(event) {
  event.preventDefault();
  if (state.paused) {
    return;
  }

  updateEvaluateState("evaluating intent...");
  document.getElementById("api-status").textContent = "ONLINE";

  try {
    const response = await fetch("/api/evaluate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ intent: formIntent() })
    });
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || "Evaluation failed");
    }

    state.counts.total += 1;
    if (payload.decision === "ALLOW") {
      state.counts.allow += 1;
      state.barHistory.push("allow");
    } else {
      state.counts.esc += 1;
      state.barHistory.push("escalate");
      addEscalation(payload);
    }
    state.evalTimestamps.push(Date.now());

    renderReceipt(payload);
    addFeedItem(payload);
    updateMetrics();
    renderBars();
    updateRate();
    await refreshAudit();
    addLog(payload.decision === "ALLOW" ? "ok" : "human", payload.original_intent.intent_id + ": " + payload.decision + " (" + payload.reason + ")" + (payload.decision === "ALLOW" ? "" : " | transparency rewarded"));
    updateEvaluateState("evaluation complete");
  } catch (error) {
    document.getElementById("api-status").textContent = "DEGRADED";
    addLog("warn", error.message);
    updateEvaluateState(error.message);
  }
}

document.getElementById("btn-stop").addEventListener("click", handlePauseRequest);
document.getElementById("btn-resume").addEventListener("click", handleResume);
document.getElementById("confirm-yes").addEventListener("click", confirmPause);
document.getElementById("confirm-no").addEventListener("click", cancelPause);
document.getElementById("refresh-audit").addEventListener("click", refreshAudit);
document.getElementById("intent-form").addEventListener("submit", evaluateIntent);
document.getElementById("esc-queue").addEventListener("click", (event) => {
  const button = event.target.closest("button[data-id]");
  if (!button) {
    return;
  }
  resolveEscalation(button.dataset.id, button.dataset.action);
});

updateClock();
updateMetrics();
renderBars();
renderEscalations();
refreshAudit();
addLog("ok", "apex policy gate online");
addLog("ok", "dashboard connected to local evaluation api");
addLog("human", "operator console ready");
updateEvaluateState();
setInterval(updateClock, 1000);
setInterval(updateRate, 5000);

