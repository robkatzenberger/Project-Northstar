const elements = {
  agents: document.querySelector("#agents"),
  empty: document.querySelector("#empty"),
  loading: document.querySelector("#loading"),
  loadError: document.querySelector("#load-error"),
  loadErrorDetail: document.querySelector("#load-error-detail"),
  notice: document.querySelector("#notice"),
  workspace: document.querySelector(".workspace"),
  registryId: document.querySelector("#registry-id"),
  registryVersion: document.querySelector("#registry-version"),
  registryHash: document.querySelector("#registry-hash"),
  enrollForm: document.querySelector("#enroll-form"),
  agentId: document.querySelector("#agent-id"),
  enrollError: document.querySelector("#enroll-error"),
  saveButton: document.querySelector("#save-button"),
  saveState: document.querySelector("#save-state"),
  retryButton: document.querySelector("#retry-button"),
  grantDialog: document.querySelector("#grant-dialog"),
  grantForm: document.querySelector("#grant-form"),
  grantAgentId: document.querySelector("#grant-agent-id"),
  grantId: document.querySelector("#grant-id"),
  grantAction: document.querySelector("#grant-action"),
  grantTarget: document.querySelector("#grant-target"),
  grantIssuedBy: document.querySelector("#grant-issued-by"),
  grantError: document.querySelector("#grant-error"),
  credentialDialog: document.querySelector("#credential-dialog"),
  credentialValue: document.querySelector("#credential-value"),
  copyCredential: document.querySelector("#copy-credential"),
};

let registry = null;
let dirty = false;
const identifierPattern = /^[a-zA-Z0-9][a-zA-Z0-9._:-]{0,127}$/;
const actionPattern = /^[a-z][a-z0-9]*(?:[._:-][a-z0-9]+)*$/;
const targetPattern = /^(?:[a-z][a-z0-9+.-]*:\/\/|\/|urn:|[a-z0-9])\S{0,254}$/i;

function setNotice(message, type = "success") {
  elements.notice.textContent = message;
  elements.notice.className = type === "error" ? "notice error" : "notice";
  elements.notice.hidden = false;
}

function clearNotice() {
  elements.notice.hidden = true;
}

function markDirty() {
  dirty = true;
  elements.saveButton.disabled = false;
  elements.saveState.textContent = "Unsaved changes";
  elements.registryHash.hidden = true;
  clearNotice();
}

function fieldError(element, message) {
  element.textContent = message;
  element.hidden = false;
}

function makeButton(label, className, onClick) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = className;
  button.textContent = label;
  button.addEventListener("click", onClick);
  return button;
}

function renderGrant(grant, agent) {
  const row = document.createElement("div");
  row.className = "grant";

  const id = document.createElement("span");
  id.className = "grant-id";
  id.textContent = grant.id;
  id.title = grant.id;

  const action = document.createElement("span");
  action.className = "grant-value";
  action.textContent = grant.action;
  action.title = `Action: ${grant.action}`;

  const target = document.createElement("span");
  target.className = "grant-value";
  target.textContent = grant.target;
  target.title = `Target: ${grant.target} · Issued by ${grant.issuedBy}`;

  const revoke = makeButton("Revoke", "revoke", () => {
    agent.grants = agent.grants.filter((candidate) => candidate.id !== grant.id);
    markDirty();
    render();
  });
  revoke.setAttribute("aria-label", `Revoke ${grant.id}`);
  row.append(id, action, target, revoke);
  return row;
}

function renderAgent(agent) {
  const card = document.createElement("article");
  card.className = "agent";

  const header = document.createElement("div");
  header.className = "agent-header";
  const identity = document.createElement("div");
  identity.className = "agent-identity";
  const id = document.createElement("span");
  id.className = "agent-id";
  id.textContent = agent.id;
  identity.append(id);

  const actions = document.createElement("div");
  actions.className = "agent-actions";
  const whitelistLabel = document.createElement("label");
  whitelistLabel.className = "switch";
  whitelistLabel.title = agent.whitelisted ? "Whitelisted" : "Not whitelisted";
  const toggle = document.createElement("input");
  toggle.type = "checkbox";
  toggle.checked = agent.whitelisted;
  toggle.setAttribute("aria-label", `Whitelisted: ${agent.id}`);
  const track = document.createElement("span");
  track.className = "switch-track";
  toggle.addEventListener("change", () => {
    agent.whitelisted = toggle.checked;
    markDirty();
    render();
  });
  whitelistLabel.append(toggle, track);
  const remove = makeButton("Remove", "remove-agent", () => {
    registry.agents = registry.agents.filter((candidate) => candidate.id !== agent.id);
    markDirty();
    render();
  });
  actions.append(whitelistLabel, remove);
  header.append(identity, actions);

  const grants = document.createElement("div");
  grants.className = "grants";
  const grantsHeading = document.createElement("div");
  grantsHeading.className = "grants-heading";
  const grantLabel = document.createElement("span");
  grantLabel.className = "grants-label";
  grantLabel.textContent = `${agent.grants.length} Granted`;
  const addGrant = makeButton("Add grant", "add-grant", () => openGrantDialog(agent.id));
  grantsHeading.append(grantLabel, addGrant);
  grants.append(grantsHeading);
  if (agent.grants.length) {
    for (const grant of agent.grants) grants.append(renderGrant(grant, agent));
  } else {
    const empty = document.createElement("p");
    empty.className = "no-grants";
    empty.textContent = "No scopes granted.";
    grants.append(empty);
  }
  card.append(header, grants);
  return card;
}

function render() {
  elements.registryId.textContent = registry.id;
  elements.registryVersion.textContent = registry.version;
  elements.agents.replaceChildren(...registry.agents.map(renderAgent));
  elements.agents.hidden = registry.agents.length === 0;
  elements.empty.hidden = registry.agents.length > 0;
}

async function load() {
  elements.loading.hidden = false;
  elements.loadError.hidden = true;
  elements.agents.hidden = true;
  elements.empty.hidden = true;
  clearNotice();
  try {
    const response = await fetch("/api/registry", { cache: "no-store" });
    const payload = await response.json();
    if (!response.ok) throw new Error(payload.error || "The request failed.");
    registry = payload.registry;
    dirty = false;
    elements.saveButton.disabled = true;
    elements.saveState.textContent = "No unsaved changes";
    render();
  } catch (error) {
    elements.loadErrorDetail.textContent = error.message;
    elements.loadError.hidden = false;
  } finally {
    elements.loading.hidden = true;
  }
}

function openGrantDialog(agentId) {
  elements.grantForm.reset();
  elements.grantAgentId.value = agentId;
  elements.grantIssuedBy.value = "operator";
  elements.grantError.hidden = true;
  elements.grantDialog.showModal();
  requestAnimationFrame(() => elements.grantId.focus());
}

elements.enrollForm.addEventListener("submit", (event) => {
  event.preventDefault();
  const id = elements.agentId.value.trim();
  elements.enrollError.hidden = true;
  if (!id) {
    fieldError(elements.enrollError, "Enter an operator id.");
    elements.agentId.focus();
    return;
  }
  if (!identifierPattern.test(id)) {
    fieldError(elements.enrollError, "Use letters, numbers, dots, colons, underscores, or hyphens.");
    elements.agentId.focus();
    return;
  }
  if (registry.agents.some((agent) => agent.id === id)) {
    fieldError(elements.enrollError, `Operator "${id}" is already enrolled.`);
    return;
  }
  registry.agents.push({ id, whitelisted: false, grants: [] });
  elements.enrollForm.reset();
  markDirty();
  render();
});

elements.grantForm.addEventListener("submit", (event) => {
  event.preventDefault();
  const grant = {
    id: elements.grantId.value.trim(),
    action: elements.grantAction.value.trim(),
    target: elements.grantTarget.value.trim(),
    issuedBy: elements.grantIssuedBy.value.trim(),
  };
  const agent = registry.agents.find(({ id }) => id === elements.grantAgentId.value);
  const duplicate = registry.agents.some(({ grants }) =>
    grants.some(({ id }) => id === grant.id),
  );
  if (!agent) {
    elements.grantError.textContent = "The selected agent is no longer enrolled.";
  } else if (!Object.values(grant).every(Boolean)) {
    elements.grantError.textContent = "Complete all grant fields.";
  } else if (!identifierPattern.test(grant.id) || !identifierPattern.test(grant.issuedBy)) {
    elements.grantError.textContent = "Grant id and issuer must use identifier characters only.";
  } else if (!actionPattern.test(grant.action)) {
    elements.grantError.textContent = "Action must be a lowercase scope name such as documents.read.";
  } else if (!targetPattern.test(grant.target)) {
    elements.grantError.textContent = "Enter a target without spaces, such as northstar://documents.";
  } else if (duplicate) {
    elements.grantError.textContent = `Grant id "${grant.id}" is already in use.`;
  } else {
    agent.grants.push(grant);
    markDirty();
    render();
    elements.grantDialog.close();
    return;
  }
  elements.grantError.hidden = false;
});

elements.saveButton.addEventListener("click", async () => {
  elements.saveButton.disabled = true;
  elements.saveButton.textContent = "Saving…";
  elements.saveState.textContent = "Saving…";
  elements.workspace.classList.add("is-saving");
  elements.workspace.setAttribute("aria-busy", "true");
  clearNotice();
  try {
    const response = await fetch("/api/registry", {
      method: "PUT",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ registry }),
    });
    const payload = await response.json();
    if (!response.ok) throw new Error(payload.error || "Save failed.");
    registry = payload.registry;
    dirty = false;
    render();
    elements.registryHash.textContent = `SHA-256 ${payload.hash.slice(0, 12)}…`;
    elements.registryHash.hidden = false;
    elements.saveState.textContent = "Saved";
  } catch (error) {
    elements.saveButton.disabled = false;
    elements.saveState.textContent = "Unsaved changes";
    setNotice(`Couldn’t save — nothing was changed. ${error.message}`, "error");
  } finally {
    elements.saveButton.textContent = "Save registry";
    elements.workspace.classList.remove("is-saving");
    elements.workspace.removeAttribute("aria-busy");
  }
});

document.querySelector("#mint-button").addEventListener("click", async () => {
  try {
    const response = await fetch("/api/credentials", { method: "POST" });
    const payload = await response.json();
    if (!response.ok) throw new Error(payload.error || "Credential could not be minted.");
    elements.credentialValue.textContent = payload.credential;
    elements.copyCredential.textContent = "Copy";
    elements.credentialDialog.showModal();
  } catch (error) {
    setNotice(error.message, "error");
  }
});

elements.copyCredential.addEventListener("click", async () => {
  try {
    await navigator.clipboard.writeText(elements.credentialValue.textContent);
    elements.copyCredential.textContent = "Copied";
  } catch {
    elements.copyCredential.textContent = "Select to copy";
    const selection = window.getSelection();
    selection.removeAllRanges();
    const range = document.createRange();
    range.selectNodeContents(elements.credentialValue);
    selection.addRange(range);
  }
});

function closeCredential() {
  elements.credentialDialog.close();
}

document.querySelector("#credential-close").addEventListener("click", closeCredential);
document.querySelector("#credential-done").addEventListener("click", closeCredential);
document.querySelector("#empty-add-button").addEventListener("click", () => {
  elements.agentId.focus();
});
for (const button of document.querySelectorAll(".grant-close")) {
  button.addEventListener("click", () => elements.grantDialog.close());
}
elements.credentialDialog.addEventListener("close", () => {
  elements.credentialValue.textContent = "";
});
elements.retryButton.addEventListener("click", load);
window.addEventListener("beforeunload", (event) => {
  if (!dirty) return;
  event.preventDefault();
});

load();
