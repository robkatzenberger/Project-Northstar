const elements = Object.fromEntries([
  'rules', 'save-state', 'rule-error', 'reload-rules', 'save-rules',
  'policy-version', 'policy-hash', 'action-list', 'catalog-error',
  'evaluate-form', 'agent-id', 'credential', 'action', 'target', 'evaluate',
  'unsaved-note', 'result', 'result-title', 'result-status', 'result-reason',
  'result-meta', 'matched', 'matched-rules', 'execution-note',
  'sign-in-panel', 'sign-in-form', 'operator-key', 'sign-in', 'sign-in-error',
  'issue-credential', 'credential-state', 'refresh-audit', 'audit-error',
  'audit-empty', 'audit-records', 'audit-status-filter',
  'refresh-reviews', 'reviews-error', 'reviews-empty', 'review-records',
  'reviews-paused', 'agent-connection', 'connection-form', 'connection-agent-id',
  'issue-agent-credential', 'connection-state', 'connection-secret',
  'connection-credential', 'copy-agent-credential', 'clear-agent-credential', 'agent-endpoint',
  'live-strip', 'live-pulse', 'live-label', 'live-activity', 'live-pending', 'live-updated', 'live-error',
].map((id) => [id, document.getElementById(id)]));

let policy = null;
let actions = [];
let saving = false;
let loading = false;
let conflict = false;
let evaluating = false;
let authenticated = false;
let signingIn = false;
let issuing = false;
let loadingAudit = false;
let loadingReviews = false;
let reviewRefreshQueued = false;
let provisioning = false;
let pollingTimer = null;
let polling = false;
let liveStripTicker = null;
let knownAuditIds = new Set();
let auditIdsSeeded = false;
let auditStatusFilter = '';
let latestAuditRecords = [];
let newestActivityAt = null;
let pendingReviewCount = null;
let lastPollSuccessAt = null;
let livePollError = '';
const reviewDrafts = new Map();
const gateStatuses = ['APPROVE', 'ESCALATE', 'BLOCKED', 'ERROR'];


function formatRelativeAge(isoOrMs, now = Date.now()) {
  if (isoOrMs == null || isoOrMs === '') return null;
  const ms = typeof isoOrMs === 'number' ? isoOrMs : Date.parse(isoOrMs);
  if (!Number.isFinite(ms)) return null;
  const seconds = Math.max(0, Math.floor((now - ms) / 1000));
  if (seconds < 5) return 'just now';
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

function isPollingActive() {
  return authenticated && document.visibilityState === 'visible' && pollingTimer !== null;
}

function updateLiveStrip() {
  if (!elements['live-strip']) return;
  const live = isPollingActive();
  elements['live-pulse'].dataset.active = live ? 'true' : 'false';
  if (!authenticated) {
    elements['live-label'].textContent = 'Waiting for sign-in';
  } else if (live) {
    elements['live-label'].textContent = `Gate live · ${window.location.host || 'localhost'}`;
  } else {
    elements['live-label'].textContent = `Gate signed in · ${window.location.host || 'localhost'}`;
  }
  const activityAge = formatRelativeAge(newestActivityAt);
  elements['live-activity'].textContent = activityAge ? `Last activity: ${activityAge}` : 'Last activity: —';
  elements['live-pending'].textContent = pendingReviewCount == null
    ? 'Pending reviews: —'
    : `Pending reviews: ${pendingReviewCount}`;
  const updatedAge = formatRelativeAge(lastPollSuccessAt);
  elements['live-updated'].textContent = updatedAge ? `Updated ${updatedAge}` : 'Updated —';
  if (livePollError) {
    elements['live-error'].textContent = livePollError;
    elements['live-error'].hidden = false;
  } else {
    elements['live-error'].textContent = '';
    elements['live-error'].hidden = true;
  }
}

function manageLiveStripTicker() {
  if (authenticated && document.visibilityState === 'visible') {
    if (liveStripTicker === null) liveStripTicker = setInterval(updateLiveStrip, 1000);
  } else if (liveStripTicker !== null) {
    clearInterval(liveStripTicker);
    liveStripTicker = null;
  }
  updateLiveStrip();
}

function noteActivityTimestamp(value) {
  if (typeof value !== 'string') return;
  const ms = Date.parse(value);
  if (!Number.isFinite(ms)) return;
  if (newestActivityAt == null || ms > newestActivityAt) newestActivityAt = ms;
}

function highlightNewAuditRows(records) {
  const ids = [];
  const fresh = [];
  for (const record of records) {
    if (typeof record?.id !== 'string') continue;
    ids.push(record.id);
    if (auditIdsSeeded && !knownAuditIds.has(record.id)) fresh.push(record.id);
  }
  if (!auditIdsSeeded) {
    knownAuditIds = new Set(ids);
    auditIdsSeeded = true;
    return new Set();
  }
  for (const id of ids) knownAuditIds.add(id);
  return new Set(fresh);
}

function requireSignIn() {
  authenticated = false;
  elements['sign-in-panel'].hidden = false;
  elements['audit-records'].replaceChildren();
  elements['audit-empty'].textContent = 'Sign in to inspect the saved records.';
  elements['audit-empty'].hidden = false;
  elements['review-records'].replaceChildren();
  elements['reviews-empty'].textContent = 'Sign in to inspect escalations.';
  elements['reviews-empty'].hidden = false;
  clearConnectionCredential();
  knownAuditIds = new Set();
  auditIdsSeeded = false;
  newestActivityAt = null;
  pendingReviewCount = null;
  lastPollSuccessAt = null;
  livePollError = '';
  updateControls();
}

function isDirty() {
  return policy !== null && elements.rules.value !== policy.text;
}

function updateControls(message) {
  const dirty = isDirty();
  elements['save-rules'].disabled = !authenticated || !policy || !dirty || saving || loading || conflict;
  elements['reload-rules'].disabled = saving || loading;
  elements.rules.disabled = !authenticated || !policy || saving || loading;
  elements.evaluate.disabled = actions.length === 0 || evaluating || saving || loading;
  elements['issue-credential'].disabled = !authenticated || issuing;
  elements['refresh-audit'].disabled = !authenticated || loadingAudit;
  elements['refresh-reviews'].disabled = !authenticated || loadingReviews;
  elements['sign-in'].disabled = signingIn;
  elements['agent-connection'].hidden = !authenticated;
  elements['issue-agent-credential'].disabled = !authenticated || provisioning;
  elements['connection-agent-id'].disabled = provisioning;
  elements['unsaved-note'].hidden = !dirty;
  elements['save-state'].classList.toggle('dirty', dirty);
  elements['save-state'].textContent = message || (!authenticated ? 'Sign in to edit' : conflict ? 'Reload required' : dirty ? 'Unsaved changes' : policy ? 'Saved rules active' : 'Rules unavailable');
  managePolling();
}

function managePolling() {
  if (!authenticated || document.visibilityState !== 'visible') {
    if (pollingTimer !== null) clearInterval(pollingTimer);
    pollingTimer = null;
  } else if (pollingTimer === null) {
    pollingTimer = setInterval(pollDashboard, 3000);
  }
  manageLiveStripTicker();
}

async function pollDashboard() {
  if (!authenticated || document.visibilityState !== 'visible' || polling || loadingAudit || loadingReviews) return;
  polling = true;
  try {
    const results = await Promise.all([loadAudit(), loadReviews({ automatic: true })]);
    const failed = results.some((ok) => ok === false);
    if (!failed) {
      lastPollSuccessAt = Date.now();
      livePollError = '';
    } else if (!livePollError) {
      livePollError = 'Live refresh hiccup — retrying. Displayed data may be briefly stale.';
    }
  } catch {
    livePollError = 'Live refresh unavailable — page remains usable. Retrying…';
  } finally {
    polling = false;
    updateLiveStrip();
  }
}

function setRuleError(message = '') {
  elements['rule-error'].textContent = message;
  elements['rule-error'].hidden = !message;
  elements.rules.setAttribute('aria-invalid', message ? 'true' : 'false');
}

async function request(path, options = {}) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 15000);
  try {
    const response = await fetch(path, {
      ...options,
      credentials: 'same-origin',
      cache: 'no-store',
      headers: { Accept: 'application/json', ...(options.body ? { 'Content-Type': 'application/json' } : {}) },
      signal: controller.signal,
    });
    let data;
    try { data = await response.json(); } catch { throw new Error('The gate returned an unreadable response.'); }
    if (!response.ok) {
      if (response.status === 401 && path !== '/api/gate/evaluate') requireSignIn();
      const error = new Error(typeof data.error === 'string' ? data.error : `Request failed (${response.status}).`);
      error.status = response.status;
      error.code = data.code;
      error.line = data.line;
      throw error;
    }
    return data;
  } catch (error) {
    if (error.name === 'AbortError') throw new Error('The gate did not respond in time. Reload saved rules before retrying a save.');
    throw error;
  } finally {
    clearTimeout(timer);
  }
}

function checkPolicy(value) {
  if (!value || !Number.isSafeInteger(value.version) || value.version < 1 || typeof value.text !== 'string' || typeof value.hash !== 'string') {
    throw new Error('The gate returned an invalid rules document.');
  }
  return value;
}

function showPolicy(value) {
  policy = checkPolicy(value);
  elements.rules.value = policy.text;
  elements['policy-version'].textContent = `Policy version ${policy.version}`;
  elements['policy-hash'].textContent = policy.hash;
  elements['policy-hash'].title = `Saved policy hash: ${policy.hash}`;
  conflict = false;
}

async function loadRules() {
  if (isDirty() && !window.confirm('Discard your unsaved changes and load the current saved rules?')) return;
  loading = true;
  setRuleError();
  updateControls('Loading rules…');
  try {
    showPolicy(await request('/api/gate-rules'));
    authenticated = true;
    elements['sign-in-panel'].hidden = true;
  } catch (error) {
    if (error.status !== 401) setRuleError(error.message);
  } finally {
    loading = false;
    updateControls();
  }
}

function focusRuleLine(line) {
  if (!Number.isSafeInteger(line) || line < 1) return;
  const rows = elements.rules.value.split('\n');
  if (line > rows.length) return;
  const start = rows.slice(0, line - 1).reduce((offset, row) => offset + row.length + 1, 0);
  elements.rules.focus();
  elements.rules.setSelectionRange(start, start + rows[line - 1].length);
}

async function saveRules() {
  if (!authenticated || !policy || saving || loading || conflict || !isDirty()) return;
  saving = true;
  setRuleError();
  updateControls('Saving…');
  let errorLine;
  try {
    showPolicy(await request('/api/gate-rules', {
      method: 'PUT',
      body: JSON.stringify({ version: policy.version, text: elements.rules.value }),
    }));
    void loadAudit();
  } catch (error) {
    conflict = error.status === 409;
    errorLine = error.line;
    const detail = [Number.isSafeInteger(error.line) ? `Line ${error.line}` : '', typeof error.code === 'string' ? error.code : ''].filter(Boolean).join(' · ');
    const message = conflict ? 'The saved rules changed in another session. Your edits are preserved here. Copy them if needed, then reload before saving again.' : error.message;
    setRuleError(`${detail ? `${detail}: ` : ''}${message}`);
  } finally {
    saving = false;
    updateControls();
    focusRuleLine(errorLine);
  }
}

async function loadActions() {
  try {
    const catalog = await request('/api/action-catalog');
    if (!Array.isArray(catalog.actions) || !catalog.actions.length || catalog.actions.some((action) => typeof action.id !== 'string' || typeof action.label !== 'string')) {
      throw new Error('The gate returned an invalid action catalog.');
    }
    actions = catalog.actions;
    elements['action-list'].replaceChildren();
    elements.action.replaceChildren();
    const prompt = document.createElement('option');
    prompt.value = '';
    prompt.textContent = 'Choose an action';
    elements.action.append(prompt);
    elements['catalog-error'].hidden = true;
    for (const action of actions) {
      const item = document.createElement('li');
      const id = document.createElement('span');
      id.className = 'action-id';
      id.textContent = action.id;
      const label = document.createElement('span');
      label.className = 'action-label';
      label.textContent = action.label;
      item.append(id, label);
      elements['action-list'].append(item);
      const option = document.createElement('option');
      option.value = action.id;
      option.textContent = `${action.id} — ${action.label}`;
      elements.action.append(option);
    }
    elements.action.disabled = false;
  } catch (error) {
    elements['action-list'].replaceChildren();
    if (error.status !== 401) {
      elements['catalog-error'].textContent = `${error.message} Reload the page to retry.`;
      elements['catalog-error'].hidden = false;
    }
  } finally {
    updateControls();
  }
}

function addResultDetail(label, value) {
  const term = document.createElement('dt');
  const detail = document.createElement('dd');
  term.textContent = label;
  detail.textContent = value;
  elements['result-meta'].append(term, detail);
}

function setResult(status, reason, title = 'Gate result') {
  elements.result.hidden = false;
  elements['result-title'].textContent = title;
  elements['result-status'].className = `badge ${status.toLowerCase()}`;
  elements['result-status'].textContent = status.replaceAll('_', ' ');
  elements['result-reason'].textContent = reason;
  elements['result-meta'].replaceChildren();
  elements['matched-rules'].replaceChildren();
  elements.matched.hidden = true;
}

function showResult(result) {
  if (!result || result.version !== '0.0.2' || result.decisionModel !== 'approve_escalate_v1' || !gateStatuses.includes(result.status) || result.execution !== 'NOT_EXECUTED' || typeof result.reason !== 'string' || typeof result.code !== 'string' || !['SWITCHBOARD', 'GATE'].includes(result.stage) || !Array.isArray(result.matchedRules)) {
    throw new Error('The gate returned an invalid decision. No permission can be inferred.');
  }
  if (!result.audit || typeof result.audit.id !== 'string' || typeof result.audit.recordedAt !== 'string') {
    throw new Error('The auditor did not return a recorded decision. No permission can be inferred.');
  }
  setResult(result.status, result.reason);
  addResultDetail('Stage', result.stage);
  addResultDetail('Code', result.code);
  if (result.switchboard) {
    addResultDetail('Switchboard', `${result.switchboard.status} · ${result.switchboard.code}`);
    if (result.switchboard.principalId) addResultDetail('Verified agent', result.switchboard.principalId);
  }
  if (result.policy) {
    addResultDetail('Policy version', String(result.policy.version));
    addResultDetail('Policy hash', result.policy.hash);
  }
  if (result.audit && typeof result.audit.id === 'string' && typeof result.audit.recordedAt === 'string') {
    addResultDetail('Receipt ID', result.audit.id);
    addResultDetail('Recorded at', result.audit.recordedAt);
  }
  for (const rule of result.matchedRules) {
    const item = document.createElement('li');
    item.textContent = `Line ${rule.line} · ${rule.decision} ${rule.action} ${rule.target}`;
    elements['matched-rules'].append(item);
  }
  elements.matched.hidden = result.matchedRules.length === 0;
  elements['execution-note'].textContent = result.status === 'ESCALATE' ? 'Waiting for human review. An operator must approve or deny with context. No action was executed.' : result.status === 'BLOCKED' ? 'Switchboard blocked this request. Human review cannot override missing authority. No action was executed.' : result.status === 'ERROR' ? 'Gate evaluation failed. A journal receipt was recorded for audit/debug. This is not human-reviewable and nothing was executed.' : 'No action was executed. This result is an action-and-target check, not an execution credential.';
}

async function signIn(key) {
  if (signingIn) return;
  signingIn = true;
  elements['sign-in-error'].hidden = true;
  updateControls();
  try {
    await request('/api/operator/session', { method: 'POST', body: JSON.stringify({ key }) });
    authenticated = true;
    elements['sign-in-panel'].hidden = true;
    await Promise.allSettled([loadRules(), loadActions(), loadAudit(), loadReviews()]);
  } catch (error) {
    requireSignIn();
    elements['sign-in-error'].textContent = error.message;
    elements['sign-in-error'].hidden = false;
  } finally {
    signingIn = false;
    updateControls();
  }
}

async function issueCredential() {
  if (!authenticated || issuing) return;
  if (!elements['agent-id'].reportValidity()) return;
  issuing = true;
  elements.credential.value = '';
  elements['credential-state'].hidden = true;
  updateControls();
  try {
    const response = await request('/api/gate/credentials', {
      method: 'POST', body: JSON.stringify({ agentId: elements['agent-id'].value }),
    });
    if (typeof response.credential !== 'string' || !/^[a-f0-9]{64}$/.test(response.credential) || typeof response.principalId !== 'string' || response.ephemeral !== true) {
      throw new Error('The auditor returned an invalid credential response.');
    }
    elements.credential.value = response.credential;
    elements['credential-state'].className = 'credential-state';
    elements['credential-state'].textContent = `Temporary credential issued for ${response.principalId}. Ready for a check.`;
    void loadAudit();
  } catch (error) {
    elements['credential-state'].className = 'credential-state error';
    elements['credential-state'].textContent = error.message;
  } finally {
    elements['credential-state'].hidden = false;
    issuing = false;
    updateControls();
  }
}

function clearConnectionCredential() {
  elements['connection-credential'].value = '';
  elements['connection-secret'].hidden = true;
}

async function provisionAgent(event) {
  event.preventDefault();
  if (!authenticated || provisioning) return;
  const principalId = elements['connection-agent-id'].value;
  provisioning = true;
  clearConnectionCredential();
  elements['connection-state'].className = 'credential-state';
  elements['connection-state'].textContent = 'Issuing agent credential…';
  updateControls();
  try {
    const response = await request('/api/gate/credentials', { method: 'POST', body: JSON.stringify({ agentId: principalId }) });
    if (response.principalId !== principalId || response.ephemeral !== true || !/^[a-f0-9]{64}$/.test(response.credential)) {
      throw new Error('The auditor returned an invalid credential response.');
    }
    elements['connection-credential'].value = response.credential;
    elements['connection-secret'].hidden = false;
    elements['connection-state'].textContent = `Credential issued for ${principalId}. Give it to the agent with the connection instructions; requests will appear when received.`;
    void loadAudit();
  } catch (error) {
    elements['connection-state'].className = 'credential-state error';
    elements['connection-state'].textContent = error.message;
  } finally {
    provisioning = false;
    updateControls();
  }
}

async function copyConnectionCredential() {
  const credential = elements['connection-credential'].value;
  if (!credential) return;
  try {
    await navigator.clipboard.writeText(credential);
    clearConnectionCredential();
    elements['connection-state'].className = 'credential-state';
    elements['connection-state'].textContent = 'Credential copied and cleared from this page. Paste it only into the agent’s private runtime.';
  } catch {
    elements['connection-state'].className = 'credential-state error';
    elements['connection-state'].textContent = 'Clipboard access was unavailable. Select and copy the masked credential manually, then clear it.';
    elements['connection-credential'].focus();
    elements['connection-credential'].select();
  }
}


function recordMatchesAuditFilter(record) {
  if (!auditStatusFilter) return true;
  if (record?.result && gateStatuses.includes(record.result.status)) {
    return record.result.status === auditStatusFilter;
  }
  if (auditStatusFilter === 'HUMAN' && ['APPROVE', 'DENY'].includes(record?.decision) && typeof record?.assessmentId === 'string') {
    return true;
  }
  return false;
}

function renderAuditList(records, freshIds = new Set()) {
  const filtered = records.filter(recordMatchesAuditFilter);
  elements['audit-records'].replaceChildren();
  elements['audit-empty'].textContent = records.length === 0
    ? 'No saved records yet.'
    : filtered.length === 0
      ? `No ${auditStatusFilter.replaceAll('_', ' ')} records in the latest journal window.`
      : 'No saved records yet.';
  elements['audit-empty'].hidden = filtered.length !== 0;
  for (const record of filtered) {
    const item = document.createElement('li');
    if (typeof record?.id === 'string') item.dataset.auditId = record.id;
    if (freshIds.has(record?.id)) item.classList.add('audit-row-new');
    const heading = document.createElement('div');
    heading.className = 'audit-record-heading';
    const title = document.createElement('strong');
    const humanReview = ['APPROVE', 'DENY'].includes(record.decision) && typeof record.assessmentId === 'string';
    title.textContent = humanReview ? 'Human decision' : record.type === 'assessment' ? 'Intent assessed' : record.type === 'policy_saved' ? 'Rules saved' : record.type === 'credential_issued' ? 'Credential issued' : 'Auditor record';
    heading.append(title);
    if (record.result && gateStatuses.includes(record.result.status)) {
      const badge = document.createElement('span');
      badge.className = `badge ${record.result.status.toLowerCase()}`;
      badge.textContent = record.result.status.replaceAll('_', ' ');
      heading.append(badge);
    }
    if (humanReview) {
      const badge = document.createElement('span');
      badge.className = `badge ${record.decision.toLowerCase()}`;
      badge.textContent = record.decision;
      heading.append(badge);
    }
    const time = document.createElement('time');
    time.textContent = typeof record.recordedAt === 'string' ? record.recordedAt : 'Time unavailable';
    heading.append(time);
    item.append(heading);
    if (record.request) {
      const intent = document.createElement('p');
      intent.className = 'audit-record-request';
      intent.textContent = `${record.request.agentId} · ${record.request.action} · ${record.request.target}`;
      item.append(intent);
    }
    if (record.result && typeof record.result.reason === 'string') {
      const reason = document.createElement('p');
      reason.className = 'audit-record-reason';
      reason.textContent = `${record.result.code}: ${record.result.reason}`;
      item.append(reason);
    }
    if (humanReview) {
      const context = document.createElement('p');
      context.className = 'review-comment';
      context.textContent = typeof record.comment === 'string' ? record.comment : 'Context unavailable';
      const binding = document.createElement('p');
      binding.className = 'audit-record-id';
      binding.textContent = `Reviewer: ${record.reviewer} · Assessment: ${record.assessmentId}`;
      item.append(context, binding);
    }
    const id = document.createElement('p');
    id.className = 'audit-record-id';
    id.textContent = typeof record.id === 'string' ? record.id : 'Record ID unavailable';
    item.append(id);
    elements['audit-records'].append(item);
  }
  if (freshIds.size) {
    window.setTimeout(() => {
      for (const node of elements['audit-records'].querySelectorAll('li.audit-row-new')) {
        node.classList.remove('audit-row-new');
      }
    }, 1200);
  }
}

async function loadAudit() {
  if (loadingAudit) return true;
  loadingAudit = true;
  elements['audit-error'].hidden = true;
  updateControls();
  let ok = false;
  try {
    const response = await request('/api/audit?limit=100');
    if (!response || !Array.isArray(response.records)) throw new Error('The auditor returned an invalid record list.');
    latestAuditRecords = response.records;
    const freshIds = highlightNewAuditRows(response.records);
    for (const record of response.records) noteActivityTimestamp(record?.recordedAt);
    renderAuditList(response.records, freshIds);
    lastPollSuccessAt = Date.now();
    if (!polling) livePollError = '';
    ok = true;
  } catch (error) {
    if (error.status !== 401) {
      elements['audit-error'].textContent = `${error.message} Previously displayed records may be stale.`;
      elements['audit-error'].hidden = false;
      livePollError = error.message;
    }
  } finally {
    loadingAudit = false;
    updateControls();
    updateLiveStrip();
  }
  return ok;
}

function appendDetail(list, label, value) {
  const term = document.createElement('dt');
  const detail = document.createElement('dd');
  term.textContent = label;
  detail.textContent = value;
  list.append(term, detail);
}

function isReviewable(record) {
  const result = record?.result;
  const intent = record?.request;
  const screening = result?.switchboard;
  return typeof record?.assessmentId === 'string' && record.assessmentId.length > 0
    && typeof record.recordedAt === 'string'
    && result?.version === '0.0.2' && result.decisionModel === 'approve_escalate_v1'
    && result.status === 'ESCALATE' && result.stage === 'GATE' && result.execution === 'NOT_EXECUTED'
    && typeof result.reason === 'string' && typeof result.code === 'string'
    && ['agentId', 'action', 'target'].every((key) => typeof intent?.[key] === 'string'
      && intent[key] === screening?.intent?.[key])
    && screening?.status === 'PASS' && screening.principalId === intent.agentId
    && Number.isSafeInteger(screening.registry?.version) && typeof screening.registry.id === 'string'
    && Number.isSafeInteger(result.policy?.version) && typeof result.policy.hash === 'string'
    && Array.isArray(result.matchedRules) && result.matchedRules.every((rule) => Number.isSafeInteger(rule.line)
      && ['APPROVE', 'ESCALATE'].includes(rule.decision) && typeof rule.action === 'string' && typeof rule.target === 'string');
}

function reviewDraft(assessmentId) {
  if (!reviewDrafts.has(assessmentId)) reviewDrafts.set(assessmentId, { comment: '', error: '', saving: false, conflict: false, stale: false });
  return reviewDrafts.get(assessmentId);
}

function renderReview(record) {
  const card = document.createElement('article');
  card.className = 'review-card';
  card.dataset.assessmentId = record?.assessmentId ?? '';
  const complete = isReviewable(record);
  const draft = reviewDraft(record?.assessmentId);
  const resolved = record?.review != null;
  card.dataset.reviewState = resolved ? 'resolved' : draft.conflict ? 'stale' : complete ? 'pending' : 'invalid';
  const heading = document.createElement('div');
  heading.className = 'review-heading';
  const title = document.createElement('h3');
  title.textContent = resolved ? 'Reviewed escalation' : 'Escalated intent';
  const badge = document.createElement('span');
  badge.className = `badge ${complete ? 'escalate' : 'error'}`;
  badge.textContent = complete ? 'GATE ESCALATE' : 'UNVERIFIED RECORD';
  heading.append(title, badge);
  card.append(heading);

  const details = document.createElement('dl');
  details.className = 'result-meta review-detail';
  appendDetail(details, 'Assessment ID', record?.assessmentId ?? 'Unavailable');
  appendDetail(details, 'Recorded at', record?.recordedAt ?? 'Unavailable');
  if (record?.request) {
    appendDetail(details, 'Declared agent', record.request.agentId);
    appendDetail(details, 'Action', record.request.action);
    appendDetail(details, 'Exact target', record.request.target);
  }
  const result = record?.result;
  if (result?.switchboard) {
    appendDetail(details, 'Verified agent', result.switchboard.principalId ?? 'Unavailable');
    appendDetail(details, 'Switchboard', `${result.switchboard.status} · ${result.switchboard.code}`);
    appendDetail(details, 'Registry', `${result.switchboard.registry?.id} · version ${result.switchboard.registry?.version}`);
  }
  if (result?.policy) {
    appendDetail(details, 'Policy version', String(result.policy.version));
    appendDetail(details, 'Policy hash', result.policy.hash);
  }
  card.append(details);
  const reason = document.createElement('p');
  reason.className = 'review-comment';
  reason.textContent = result ? `${result.code}: ${result.reason}` : 'Recorded assessment unavailable.';
  card.append(reason);
  const rulesHeading = document.createElement('h4');
  rulesHeading.textContent = 'Matched rules';
  const matched = document.createElement('ul');
  matched.className = 'review-rules';
  for (const rule of Array.isArray(result?.matchedRules) ? result.matchedRules : []) {
    const item = document.createElement('li');
    item.textContent = `Line ${rule.line} · ${rule.decision} ${rule.action} ${rule.target}`;
    matched.append(item);
  }
  if (matched.childElementCount === 0) {
    const item = document.createElement('li');
    item.textContent = 'No matching rule. Unmatched in-scope requests escalate.';
    matched.append(item);
  }
  card.append(rulesHeading, matched);

  if (resolved) {
    const outcome = document.createElement('div');
    outcome.className = 'review-outcome';
    const heading = document.createElement('div');
    heading.className = 'review-outcome-heading';
    const label = document.createElement('span');
    label.textContent = 'Human decision';
    const status = document.createElement('span');
    const valid = ['APPROVE', 'DENY'].includes(record.review.decision)
      && record.review.assessmentId === record.assessmentId;
    status.className = `badge ${valid ? record.review.decision.toLowerCase() : 'error'}`;
    status.textContent = valid ? record.review.decision : 'UNVERIFIED RECORD';
    heading.append(label, status);
    const comment = document.createElement('p');
    comment.className = 'review-comment';
    comment.textContent = typeof record.review.comment === 'string' ? record.review.comment : 'Context unavailable';
    const meta = document.createElement('dl');
    meta.className = 'result-meta';
    appendDetail(meta, 'Reviewer', record.review.reviewer);
    appendDetail(meta, 'Review ID', record.review.id);
    appendDetail(meta, 'Recorded at', record.review.recordedAt);
    appendDetail(meta, 'Assessment ID', record.review.assessmentId);
    outcome.append(heading, comment, meta);
    card.append(outcome);
  } else if (complete) {
    const form = document.createElement('form');
    form.className = 'review-form';
    const label = document.createElement('label');
    label.textContent = 'Human context';
    const comment = document.createElement('textarea');
    comment.name = 'comment';
    comment.rows = 3;
    comment.required = true;
    comment.value = draft.comment;
    comment.disabled = draft.saving;
    comment.placeholder = 'Explain why you approve or deny this exact intent.';
    label.append(comment);
    const help = document.createElement('p');
    help.className = 'field-help';
    help.textContent = 'Required for either decision. Maximum 2,000 UTF-8 bytes. Include context, not credentials or private payloads.';
    const error = document.createElement('p');
    error.className = 'error review-error';
    error.setAttribute('role', 'alert');
    error.textContent = draft.error;
    error.hidden = !draft.error;
    const buttons = document.createElement('div');
    buttons.className = 'review-actions';
    for (const decision of ['DENY', 'APPROVE']) {
      const button = document.createElement('button');
      button.type = 'submit';
      button.name = 'decision';
      button.value = decision;
      button.className = `button ${decision === 'APPROVE' ? 'primary' : 'danger'}`;
      button.textContent = decision === 'APPROVE' ? 'Approve' : 'Deny';
      button.disabled = !authenticated || draft.saving || (draft.conflict && !(draft.stale && decision === 'DENY'));
      buttons.append(button);
    }
    comment.addEventListener('input', () => {
      draft.comment = comment.value;
      comment.setCustomValidity('');
      if (!draft.conflict) { draft.error = ''; error.hidden = true; }
    });
    form.addEventListener('submit', async (event) => {
      event.preventDefault();
      const decision = event.submitter?.value;
      if (!['APPROVE', 'DENY'].includes(decision) || draft.saving || (draft.conflict && !(draft.stale && decision === 'DENY'))) return;
      if (!comment.value.trim()) { comment.setCustomValidity('Explain your decision before submitting.'); comment.reportValidity(); return; }
      if (new TextEncoder().encode(comment.value.trim()).length > 2000) { comment.setCustomValidity('Context must be no more than 2,000 UTF-8 bytes.'); comment.reportValidity(); return; }
      draft.comment = comment.value;
      const submittedComment = draft.comment.trim();
      draft.saving = true;
      comment.disabled = true;
      for (const button of buttons.children) button.disabled = true;
      try {
        const response = await request('/api/reviews/decision', { method: 'POST', body: JSON.stringify({ assessmentId: record.assessmentId, decision, comment: submittedComment }) });
        if (response.review?.assessmentId !== record.assessmentId || response.review.decision !== decision || typeof response.review.id !== 'string' || response.review.comment !== submittedComment) {
          throw new Error('The auditor returned an unverified review response. Refresh before retrying.');
        }
        draft.comment = '';
        draft.error = '';
      } catch (failure) {
        draft.conflict = failure.status === 409;
        draft.stale = failure.code === 'REVIEW_STALE';
        draft.error = draft.conflict ? `${failure.message} The review list has been refreshed; your unsent context is preserved. Reassess the intent if its saved state changed.` : failure.message;
        error.textContent = draft.error;
        error.hidden = false;
      } finally {
        draft.saving = false;
        comment.disabled = false;
        for (const button of buttons.children) button.disabled = !authenticated || (draft.conflict && !(draft.stale && button.value === 'DENY'));
        if (authenticated) await Promise.allSettled([loadReviews(), loadAudit()]);
      }
    });
    form.append(label, help, error, buttons);
    card.append(form);
  } else {
    const error = document.createElement('p');
    error.className = 'error';
    error.textContent = 'This record is not a complete, reviewable escalation. No human decision can be submitted.';
    card.append(error);
  }
  if (resolved && draft.comment && draft.error) {
    const retained = document.createElement('div');
    retained.className = 'review-draft';
    const heading = document.createElement('h4');
    heading.textContent = 'Your unsent context';
    const context = document.createElement('p');
    context.className = 'review-comment';
    context.textContent = draft.comment;
    const error = document.createElement('p');
    error.className = 'error review-error';
    error.textContent = draft.error;
    retained.append(heading, context, error);
    card.append(retained);
  }
  const note = document.createElement('p');
  note.className = 'execution-note';
  note.textContent = 'The human outcome is recorded separately from the original gate assessment. No execution occurs.';
  card.append(note);
  return card;
}

function deferAutomaticReviewRefresh() {
  const editing = elements['review-records'].contains(document.activeElement)
    && document.activeElement?.tagName === 'TEXTAREA';
  if (editing || [...reviewDrafts.values()].some((draft) => draft.saving || draft.comment.trim())) {
    elements['reviews-paused'].hidden = false;
    return true;
  }
  return false;
}

async function loadReviews({ automatic = false } = {}) {
  if (automatic && deferAutomaticReviewRefresh()) return true;
  if (loadingReviews) { reviewRefreshQueued = true; return true; }
  loadingReviews = true;
  elements['reviews-error'].hidden = true;
  updateControls();
  let ok = false;
  try {
    const response = await request('/api/reviews');
    if (!response || !Array.isArray(response.reviews)) throw new Error('The auditor returned an invalid review list.');
    if (automatic && deferAutomaticReviewRefresh()) {
      pendingReviewCount = response.reviews.filter((record) => record?.review == null).length;
      for (const record of response.reviews) {
        noteActivityTimestamp(record?.recordedAt);
        noteActivityTimestamp(record?.review?.recordedAt);
      }
      lastPollSuccessAt = Date.now();
      if (!polling) livePollError = '';
      ok = true;
      return ok;
    }
    pendingReviewCount = response.reviews.filter((record) => record?.review == null).length;
    for (const record of response.reviews) {
      noteActivityTimestamp(record?.recordedAt);
      noteActivityTimestamp(record?.review?.recordedAt);
    }
    elements['review-records'].replaceChildren(...response.reviews.map(renderReview));
    elements['reviews-empty'].textContent = 'No escalations to review.';
    elements['reviews-empty'].hidden = response.reviews.length > 0;
    elements['reviews-paused'].hidden = true;
    lastPollSuccessAt = Date.now();
    if (!polling) livePollError = '';
    ok = true;
  } catch (error) {
    if (error.status !== 401) {
      elements['reviews-error'].textContent = `${error.message} Displayed review records may be stale; the server rechecks every decision.`;
      elements['reviews-error'].hidden = false;
      livePollError = error.message;
    }
  } finally {
    loadingReviews = false;
    updateControls();
    updateLiveStrip();
    if (reviewRefreshQueued) { reviewRefreshQueued = false; void loadReviews(); }
  }
  return ok;
}

elements.rules.addEventListener('input', () => {
  if (!conflict) setRuleError();
  updateControls();
});
elements['reload-rules'].addEventListener('click', loadRules);
elements['save-rules'].addEventListener('click', saveRules);
elements['sign-in-form'].addEventListener('submit', (event) => {
  event.preventDefault();
  const key = elements['operator-key'].value;
  elements['operator-key'].value = '';
  void signIn(key);
});
elements['issue-credential'].addEventListener('click', issueCredential);
elements['connection-form'].addEventListener('submit', provisionAgent);
elements['copy-agent-credential'].addEventListener('click', copyConnectionCredential);
elements['clear-agent-credential'].addEventListener('click', () => {
  clearConnectionCredential();
  elements['connection-state'].className = 'credential-state';
  elements['connection-state'].textContent = 'Credential cleared from this page. Issue a replacement if it was not transferred to the agent.';
});
elements['agent-endpoint'].textContent = `${window.location.origin}/api/gate/evaluate`;
elements['refresh-audit'].addEventListener('click', loadAudit);

elements['audit-status-filter']?.addEventListener('change', () => {
  auditStatusFilter = elements['audit-status-filter'].value || '';
  renderAuditList(latestAuditRecords);
});
elements['refresh-reviews'].addEventListener('click', loadReviews);
elements.action.addEventListener('change', () => {
  const action = actions.find((candidate) => candidate.id === elements.action.value);
  elements.target.placeholder = action && typeof action.targetExample === 'string' ? action.targetExample : 'Exact target';
});
elements['evaluate-form'].addEventListener('submit', async (event) => {
  event.preventDefault();
  if (evaluating || !actions.length) return;
  const body = JSON.stringify({
    credential: elements.credential.value,
    intent: { agentId: elements['agent-id'].value, action: elements.action.value, target: elements.target.value },
  });
  elements.credential.value = '';
  elements['credential-state'].hidden = true;
  evaluating = true;
  updateControls();
  setResult('PENDING', 'Waiting for the gate…', 'Checking intent');
  elements['execution-note'].textContent = 'No action is executed by this form.';
  try {
    showResult(await request('/api/gate/evaluate', { method: 'POST', body }));
    if (authenticated) { void loadAudit(); void loadReviews(); }
  } catch (error) {
    setResult('ERROR', error.message, 'Check unavailable');
    elements['execution-note'].textContent = 'No valid decision received. Do not treat this as permission.';
  } finally {
    evaluating = false;
    updateControls();
  }
});
window.addEventListener('beforeunload', (event) => {
  if (isDirty()) { event.preventDefault(); event.returnValue = ''; }
});
document.addEventListener('visibilitychange', () => { managePolling(); if (document.visibilityState === 'visible') void pollDashboard(); });

const fragment = new URLSearchParams(window.location.hash.slice(1));
let startupKey = fragment.get('key');
if (startupKey !== null) {
  fragment.delete('key');
  window.history.replaceState(null, '', window.location.pathname + window.location.search);
  await signIn(startupKey);
  startupKey = null;
} else {
  await Promise.allSettled([loadRules(), loadActions(), loadAudit(), loadReviews()]);
}
