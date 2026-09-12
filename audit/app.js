const elements = {
  notice: document.querySelector("#notice"),
  countPending: document.querySelector("#count-pending"),
  countApproved: document.querySelector("#count-approved"),
  countDenied: document.querySelector("#count-denied"),
  countError: document.querySelector("#count-error"),
  filterStatus: document.querySelector("#filter-status"),
  filterAgent: document.querySelector("#filter-agent"),
  filterAction: document.querySelector("#filter-action"),
  filterRequest: document.querySelector("#filter-request"),
  filterSince: document.querySelector("#filter-since"),
  clearFilters: document.querySelector("#clear-filters"),
  moreFilters: document.querySelector("#more-filters"),
  moreFiltersToggle: document.querySelector("#more-filters-toggle"),
  filtersBody: document.querySelector("#filters-body"),
  filtersMobileToggle: document.querySelector("#filters-mobile-toggle"),
  activeFilterChips: document.querySelector("#active-filter-chips"),
  refreshButton: document.querySelector("#refresh-button"),
  loading: document.querySelector("#loading"),
  loadError: document.querySelector("#load-error"),
  loadErrorDetail: document.querySelector("#load-error-detail"),
  retryButton: document.querySelector("#retry-button"),
  empty: document.querySelector("#empty"),
  emptyTitle: document.querySelector("#empty-title"),
  requestList: document.querySelector("#request-list"),
  detail: document.querySelector("#detail"),
  detailClose: document.querySelector("#detail-close"),
  detailRequestId: document.querySelector("#detail-request-id"),
  detailStatusRow: document.querySelector("#detail-status-row"),
  detailAgent: document.querySelector("#detail-agent"),
  detailAction: document.querySelector("#detail-action"),
  detailTarget: document.querySelector("#detail-target"),
  detailAt: document.querySelector("#detail-at"),
  detailIntent: document.querySelector("#detail-intent"),
  detailDecision: document.querySelector("#detail-decision"),
  detailDecisionCopy: document.querySelector("#detail-decision-copy"),
  detailActions: document.querySelector("#detail-actions"),
  approveButton: document.querySelector("#approve-button"),
  denyButton: document.querySelector("#deny-button"),
  confirmDialog: document.querySelector("#confirm-dialog"),
  confirmForm: document.querySelector("#confirm-form"),
  confirmTitle: document.querySelector("#confirm-title"),
  confirmCopy: document.querySelector("#confirm-copy"),
  confirmRequestId: document.querySelector("#confirm-request-id"),
  confirmIntent: document.querySelector("#confirm-intent"),
  confirmOperator: document.querySelector("#confirm-operator"),
  confirmError: document.querySelector("#confirm-error"),
  confirmClose: document.querySelector("#confirm-close"),
  confirmCancel: document.querySelector("#confirm-cancel"),
  confirmSubmit: document.querySelector("#confirm-submit"),
};

const statusLabels = {
  PENDING: "Pending",
  APPROVED: "Approved",
  DENIED: "Denied",
  ERROR: "Error",
};

const statusChipClass = {
  PENDING: "chip-pending",
  APPROVED: "chip-approved",
  DENIED: "chip-denied",
  ERROR: "chip-error",
};

let pending = [];
let decisions = [];
let selectedId = null;
let pendingDecision = null;
let intentHashById = new Map();

function clearNotice() {
  elements.notice.hidden = true;
  elements.notice.textContent = "";
  elements.notice.classList.remove("error");
}

function setNotice(message, kind = "ok") {
  elements.notice.hidden = false;
  elements.notice.textContent = message;
  elements.notice.classList.toggle("error", kind === "error");
}

function formatWhen(value) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value || "—";
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

function chip(label, className) {
  const span = document.createElement("span");
  span.className = `chip ${className || ""}`.trim();
  span.textContent = label;
  return span;
}

function filtersActive() {
  return Boolean(
    elements.filterStatus.value.trim() ||
      elements.filterAgent.value.trim() ||
      elements.filterAction.value.trim() ||
      elements.filterRequest.value.trim() ||
      elements.filterSince.value,
  );
}

function queryString() {
  const params = new URLSearchParams();
  const status = elements.filterStatus.value.trim();
  const agentId = elements.filterAgent.value.trim();
  const action = elements.filterAction.value.trim();
  const requestId = elements.filterRequest.value.trim();
  const sinceLocal = elements.filterSince.value;
  if (status) params.set("status", status);
  if (agentId) params.set("agentId", agentId);
  if (action) params.set("action", action);
  if (requestId) params.set("requestId", requestId);
  if (sinceLocal) {
    const since = new Date(sinceLocal);
    if (!Number.isNaN(since.getTime())) params.set("since", since.toISOString());
  }
  const text = params.toString();
  return text ? `?${text}` : "";
}

function decisionFor(requestId) {
  return decisions.find((item) => item.requestId === requestId) || null;
}

function renderCounts(counts) {
  elements.countPending.textContent = counts?.pending ?? "—";
  elements.countApproved.textContent = counts?.approved ?? "—";
  elements.countDenied.textContent = counts?.denied ?? "—";
  elements.countError.textContent = counts?.error ?? "—";
}

function renderActiveFilterChips() {
  elements.activeFilterChips.replaceChildren();
  const status = elements.filterStatus.value.trim();
  if (!status) {
    elements.activeFilterChips.hidden = true;
    return;
  }

  const chipEl = document.createElement("span");
  chipEl.className = "filter-chip";
  chipEl.append(document.createTextNode(`Status: ${statusLabels[status] || status}`));

  const remove = document.createElement("button");
  remove.type = "button";
  remove.className = "filter-chip-remove";
  remove.setAttribute("aria-label", "Clear status filter");
  remove.textContent = "×";
  remove.addEventListener("click", () => {
    elements.filterStatus.value = "";
    load();
  });

  chipEl.append(remove);
  elements.activeFilterChips.append(chipEl);
  elements.activeFilterChips.hidden = false;
}

function renderList() {
  elements.requestList.replaceChildren();
  if (!pending.length) {
    elements.requestList.hidden = true;
    elements.empty.hidden = false;
    elements.emptyTitle.textContent = filtersActive()
      ? "No requests match these filters."
      : "Nothing waiting.";
    return;
  }
  elements.empty.hidden = true;
  elements.requestList.hidden = false;

  for (const record of pending) {
    const row = document.createElement("button");
    row.type = "button";
    row.className = "request-row";
    row.setAttribute("role", "option");
    row.setAttribute("aria-selected", String(record.requestId === selectedId));
    if (record.requestId === selectedId) row.classList.add("is-selected");

    const top = document.createElement("div");
    top.className = "request-top";
    const id = document.createElement("span");
    id.className = "request-id";
    id.textContent = record.requestId;
    const status = chip(statusLabels[record.status] || record.status, statusChipClass[record.status]);
    top.append(id, status);

    const meta = document.createElement("div");
    meta.className = "request-meta";
    meta.append(
      document.createTextNode(record.principalId),
      document.createTextNode(" · "),
      document.createTextNode(record.action),
      document.createTextNode(" · "),
      document.createTextNode(formatWhen(record.at)),
    );

    const intent = document.createElement("p");
    intent.className = "request-intent";
    intent.textContent = record.intentSummary;

    if (record.notScreened) {
      const chips = document.createElement("div");
      chips.className = "chip-row";
      chips.style.margin = "0";
      chips.append(chip("Not screened", "chip-muted"));
      row.append(top, meta, intent, chips);
    } else {
      row.append(top, meta, intent);
    }

    row.addEventListener("click", () => selectRequest(record.requestId));
    elements.requestList.append(row);
  }
}

async function ensureIntentHash(requestId) {
  if (intentHashById.has(requestId)) return intentHashById.get(requestId);
  const response = await fetch(`/api/audit/${encodeURIComponent(requestId)}`, { cache: "no-store" });
  const payload = await response.json();
  if (!response.ok) throw new Error(payload.error || "Could not load request detail.");
  intentHashById.set(requestId, payload.intentSummaryHash);
  if (payload.decision) {
    const index = decisions.findIndex((item) => item.requestId === requestId);
    if (index >= 0) decisions[index] = payload.decision;
    else decisions.push(payload.decision);
  }
  return payload.intentSummaryHash;
}

function renderDetail() {
  const record = pending.find((item) => item.requestId === selectedId);
  if (!record) {
    elements.detail.hidden = true;
    return;
  }

  elements.detail.hidden = false;
  elements.detailRequestId.textContent = record.requestId;
  elements.detailAgent.textContent = record.principalId;
  elements.detailAction.textContent = record.action;
  elements.detailTarget.textContent = record.target;
  elements.detailAt.textContent = formatWhen(record.at);
  elements.detailIntent.textContent = record.intentSummary;

  elements.detailStatusRow.replaceChildren(
    chip(statusLabels[record.status] || record.status, statusChipClass[record.status]),
  );
  if (record.notScreened) {
    elements.detailStatusRow.append(chip("Not screened", "chip-muted"));
  }

  const decision = decisionFor(record.requestId);
  if (decision) {
    elements.detailDecision.hidden = false;
    elements.detailDecisionCopy.textContent =
      `${decision.status} by ${decision.operatorId} at ${formatWhen(decision.at)}. Receipt ${decision.id}.`;
  } else {
    elements.detailDecision.hidden = true;
    elements.detailDecisionCopy.textContent = "";
  }

  const canDecide = record.status === "PENDING";
  elements.detailActions.hidden = !canDecide;
}

async function selectRequest(requestId) {
  selectedId = requestId;
  renderList();
  renderDetail();
  try {
    await ensureIntentHash(requestId);
    renderDetail();
  } catch (error) {
    setNotice(error.message, "error");
  }
}

async function load() {
  elements.loading.hidden = false;
  elements.loadError.hidden = true;
  elements.requestList.hidden = true;
  elements.empty.hidden = true;
  clearNotice();
  renderActiveFilterChips();
  try {
    const response = await fetch(`/api/audit${queryString()}`, { cache: "no-store" });
    const payload = await response.json();
    if (!response.ok) throw new Error(payload.error || "The request failed.");
    pending = payload.pending || [];
    decisions = payload.decisions || [];
    intentHashById = new Map();
    renderCounts(payload.counts);
    if (selectedId && !pending.some((item) => item.requestId === selectedId)) {
      selectedId = null;
      elements.detail.hidden = true;
    }
    renderList();
    renderDetail();
  } catch (error) {
    elements.loadErrorDetail.textContent = error.message;
    elements.loadError.hidden = false;
    elements.detail.hidden = true;
  } finally {
    elements.loading.hidden = true;
  }
}

function openConfirm(decision) {
  const record = pending.find((item) => item.requestId === selectedId);
  if (!record || record.status !== "PENDING") {
    setNotice("Only pending requests can be decided.", "error");
    return;
  }
  pendingDecision = decision;
  elements.confirmError.hidden = true;
  elements.confirmError.textContent = "";
  elements.confirmTitle.textContent = decision === "APPROVE" ? "Confirm approve" : "Confirm deny";
  elements.confirmCopy.textContent =
    decision === "APPROVE"
      ? "Append an APPROVED decision receipt. This does not mean the action ran."
      : "Append a DENIED decision receipt. This does not mean the action ran.";
  elements.confirmRequestId.textContent = record.requestId;
  elements.confirmIntent.textContent = record.intentSummary;
  elements.confirmSubmit.textContent = decision === "APPROVE" ? "Approve" : "Deny";
  elements.confirmSubmit.className =
    decision === "APPROVE" ? "button button-primary" : "button button-danger";
  elements.confirmDialog.showModal();
  requestAnimationFrame(() => elements.confirmOperator.focus());
}

elements.confirmForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const record = pending.find((item) => item.requestId === selectedId);
  if (!record || !pendingDecision) return;

  const operatorId = elements.confirmOperator.value.trim();
  if (!operatorId) {
    elements.confirmError.textContent = "Enter an operator id.";
    elements.confirmError.hidden = false;
    elements.confirmOperator.focus();
    return;
  }

  const submitLabel = elements.confirmSubmit.textContent;
  elements.confirmSubmit.disabled = true;
  elements.confirmSubmit.textContent = "Recording…";
  elements.confirmError.hidden = true;
  try {
    const intentSummaryHash = await ensureIntentHash(record.requestId);
    const response = await fetch("/api/audit/decide", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        requestId: record.requestId,
        decision: pendingDecision,
        operatorId,
        intentSummaryHash,
      }),
    });
    const payload = await response.json();
    if (!response.ok) throw new Error(payload.error || "Decision failed.");
    elements.confirmDialog.close();
    setNotice(payload.message || "Decision recorded.");
    await load();
    selectedId = record.requestId;
    renderList();
    renderDetail();
  } catch (error) {
    elements.confirmError.textContent = error.message;
    elements.confirmError.hidden = false;
  } finally {
    elements.confirmSubmit.disabled = false;
    elements.confirmSubmit.textContent = submitLabel;
  }
});

function closeConfirm() {
  elements.confirmDialog.close();
  pendingDecision = null;
}

elements.approveButton.addEventListener("click", () => openConfirm("APPROVE"));
elements.denyButton.addEventListener("click", () => openConfirm("DENY"));
elements.confirmClose.addEventListener("click", closeConfirm);
elements.confirmCancel.addEventListener("click", closeConfirm);
elements.detailClose.addEventListener("click", () => {
  selectedId = null;
  elements.detail.hidden = true;
  renderList();
});
elements.refreshButton.addEventListener("click", load);
elements.retryButton.addEventListener("click", load);
elements.clearFilters.addEventListener("click", () => {
  elements.filterStatus.value = "";
  elements.filterAgent.value = "";
  elements.filterAction.value = "";
  elements.filterRequest.value = "";
  elements.filterSince.value = "";
  load();
});

elements.moreFiltersToggle.addEventListener("click", () => {
  const open = elements.moreFilters.hidden;
  elements.moreFilters.hidden = !open;
  elements.moreFiltersToggle.setAttribute("aria-expanded", String(open));
  elements.moreFiltersToggle.textContent = open ? "Fewer filters" : "More filters";
});

elements.filtersMobileToggle.addEventListener("click", () => {
  const open = !elements.filtersBody.classList.contains("is-open");
  elements.filtersBody.classList.toggle("is-open", open);
  elements.filtersMobileToggle.setAttribute("aria-expanded", String(open));
});

for (const element of [
  elements.filterStatus,
  elements.filterAgent,
  elements.filterAction,
  elements.filterRequest,
  elements.filterSince,
]) {
  element.addEventListener("change", load);
  if (element.tagName === "INPUT" && element.type !== "datetime-local") {
    element.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        load();
      }
    });
  }
}

load();
