/** TL-PX 0.2 slice 3.2 typed action/hash contract tests. */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  ACTION_BINDING_FIELDS,
  actionBindingsMatchV02,
  authorizedActionBindingHashV02,
  authorizedActionBindingV02,
  authorizedActionHashV02,
  canonicalAuthorizedActionV02,
  canonicalExecutedActionV02,
  canonicalSubmittedIntentV02,
  executedActionBindingHashV02,
  executedActionHashV02,
  submittedIntentHashV02
} from "../src/action-v02.mjs";
import { canonicalize, utf8Hex } from "../src/jcs.mjs";
import { validateV02 } from "../src/validate-v02.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const fixturePath = path.join(root, "tests/fixtures/tlpx-0.2/actions/golden.json");
const golden = JSON.parse(fs.readFileSync(fixturePath, "utf8"));
const hash = (char) => `sha256:${char.repeat(64)}`;

let n = 0;
function assert(condition, message) {
  if (!condition) throw new Error(message);
  n += 1;
  console.log(`  ok  ${message}`);
}

function throws(fn, code, message) {
  let error;
  try {
    fn();
  } catch (caught) {
    error = caught;
  }
  assert(error instanceof Error, `${message}: threw`);
  if (code) assert(error.code === code, `${message}: ${code}`);
}

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

console.log("TL-PX 0.2 typed action/hash fixtures\n");

assert(golden.profile === "tlpx-action-types-v1", "fixture profile");
assert(golden.standard_version === "0.2.0", "fixture standard version");
assert(golden.cases.length === 2, "two absent/null and bound-digest cases");
assert(
  ACTION_BINDING_FIELDS.join(",") ===
    "executing_principal,action,target,arguments,environment,tenant,payload_hash,artifact_hash,adapter",
  "exact Action Binding field list"
);

for (const row of golden.cases) {
  console.log(row.id);
  assert(validateV02("submitted-intent", row.submitted_intent).ok, `${row.id} intent schema`);
  assert(validateV02("authorized-action", row.authorized_action).ok, `${row.id} authorized schema`);
  assert(validateV02("executed-action", row.executed_action).ok, `${row.id} executed schema`);
  assert(
    canonicalSubmittedIntentV02(row.submitted_intent) === row.canonical.submitted_intent,
    `${row.id} intent canonical`
  );
  assert(
    canonicalAuthorizedActionV02(row.authorized_action) === row.canonical.authorized_action,
    `${row.id} authorized canonical`
  );
  assert(
    canonicalExecutedActionV02(row.executed_action) === row.canonical.executed_action,
    `${row.id} executed canonical`
  );
  const binding = authorizedActionBindingV02(row.authorized_action);
  assert(canonicalize(binding) === row.canonical.action_binding, `${row.id} binding canonical`);
  assert(
    utf8Hex(row.canonical.submitted_intent) === row.canonical_utf8_hex.submitted_intent,
    `${row.id} intent UTF-8`
  );
  assert(
    submittedIntentHashV02(row.submitted_intent) === row.sha256.intent_hash,
    `${row.id} intent hash`
  );
  assert(
    authorizedActionHashV02(row.authorized_action) === row.sha256.authorized_action_hash,
    `${row.id} authorized hash`
  );
  assert(
    executedActionHashV02(row.executed_action) === row.sha256.executed_action_hash,
    `${row.id} executed hash`
  );
  assert(
    authorizedActionBindingHashV02(row.authorized_action) === row.sha256.action_binding_hash,
    `${row.id} binding hash`
  );
  assert(
    executedActionBindingHashV02(row.executed_action) === row.sha256.executed_action_hash,
    `${row.id} executed value is its binding`
  );
  assert(actionBindingsMatchV02(row.authorized_action, row.executed_action), `${row.id} bindings match`);
  assert(
    row.sha256.authorized_action_hash !== row.sha256.executed_action_hash,
    `${row.id} full authorized hash remains distinct`
  );
  for (const [name, digest] of Object.entries(row.digest_hex)) {
    assert(digest === row.sha256[name].slice(7), `${row.id} ${name} raw digest`);
  }
}

const base = golden.cases[0];

console.log("fail-closed schema and canonicalization");
throws(
  () => submittedIntentHashV02({ ...base.submitted_intent, derived_risk: "low" }),
  "ACTION_SCHEMA_INVALID",
  "intent rejects authority-derived field"
);
throws(
  () => authorizedActionHashV02({ ...base.authorized_action, request_id: "forbidden" }),
  "ACTION_SCHEMA_INVALID",
  "authorized action rejects requester correlation"
);
throws(
  () => executedActionHashV02({ ...base.executed_action, capability: "forbidden" }),
  "ACTION_SCHEMA_INVALID",
  "executed action rejects authorization constraint"
);
const missingDigest = { ...base.submitted_intent };
delete missingDigest.payload_hash;
throws(
  () => submittedIntentHashV02(missingDigest),
  "ACTION_SCHEMA_INVALID",
  "required null-capable digest may not be absent"
);
const riskOrder = { low: 0, medium: 1, high: 2 };
for (const derived of Object.keys(riskOrder)) {
  for (const effective of Object.keys(riskOrder)) {
    const valid = validateV02("authorized-action", {
      ...base.authorized_action,
      derived_risk: derived,
      effective_risk: effective
    }).ok;
    assert(
      valid === (riskOrder[effective] >= riskOrder[derived]),
      `${derived} derived risk with ${effective} effective risk`
    );
  }
}
throws(
  () => submittedIntentHashV02({ ...base.submitted_intent, arguments: { ratio: 1.5 } }),
  undefined,
  "float cannot enter a canonical preimage"
);

console.log("every Action Binding mutation mismatches");
const bindingMutations = [
  ["executing_principal", (v) => (v.executing_principal = "agent.b")],
  ["action", (v) => (v.action = "send_sms")],
  ["target", (v) => (v.target = "customer:999")],
  ["arguments", (v) => (v.arguments = { template: "other" })],
  ["environment", (v) => (v.environment = "staging")],
  ["tenant", (v) => (v.tenant = "tenant_other")],
  ["payload_hash", (v) => (v.payload_hash = hash("c"))],
  ["artifact_hash", (v) => (v.artifact_hash = hash("d"))],
  ["adapter.id", (v) => (v.adapter.id = "adapter.other")],
  ["adapter.version", (v) => (v.adapter.version = "9.0.0")]
];
for (const [name, mutate] of bindingMutations) {
  const changed = clone(base.executed_action);
  mutate(changed);
  assert(validateV02("executed-action", changed).ok, `${name} mutation remains schema-valid`);
  assert(!actionBindingsMatchV02(base.authorized_action, changed), `${name} mutation mismatches`);
}

console.log("authority-only fields do not leak into the Action Binding");
const originalAuthorizedHash = authorizedActionHashV02(base.authorized_action);
const originalBindingHash = authorizedActionBindingHashV02(base.authorized_action);
const detachedBinding = authorizedActionBindingV02(base.authorized_action);
detachedBinding.arguments.template = "mutated-copy";
detachedBinding.adapter.version = "9.0.0";
assert(
  base.authorized_action.arguments.template === "invoice-ready",
  "binding arguments are detached from source"
);
assert(base.authorized_action.adapter.version === "1.0.0", "binding adapter is detached from source");
const authorityOnlyMutations = [
  ["requesting_principal", (v) => (v.requesting_principal = "agent.requester.other")],
  ["derived_risk", (v) => (v.derived_risk = "low")],
  ["effective_risk", (v) => (v.effective_risk = "medium")],
  ["risk_reasons", (v) => (v.risk_reasons = ["policy.changed"])],
  ["risk_source", (v) => (v.risk_source = "operator_policy")],
  ["data_classes", (v) => (v.data_classes = [...v.data_classes].reverse())],
  ["capability", (v) => (v.capability = "mailer.send.restricted")],
  ["resource_scope", (v) => (v.resource_scope = ["customer:123", "customer:456"])],
  ["policy_bundle_hash", (v) => (v.policy_bundle_hash = hash("9"))]
];
for (const [name, mutate] of authorityOnlyMutations) {
  const changed = clone(base.authorized_action);
  mutate(changed);
  assert(validateV02("authorized-action", changed).ok, `${name} change remains schema-valid`);
  assert(authorizedActionHashV02(changed) !== originalAuthorizedHash, `${name} changes full hash`);
  assert(authorizedActionBindingHashV02(changed) === originalBindingHash, `${name} excluded from binding`);
}

console.log("material Submitted Intent fields change intent_hash");
const originalIntentHash = submittedIntentHashV02(base.submitted_intent);
const intentMutations = [
  ["requesting_principal", (v) => (v.requesting_principal = "agent.other")],
  ["executing_principal", (v) => (v.executing_principal = "agent.worker")],
  ["action", (v) => (v.action = "send_sms")],
  ["intent_class", (v) => (v.intent_class = "notification")],
  ["target", (v) => (v.target = "customer:999")],
  ["arguments", (v) => (v.arguments = { template: "other" })],
  ["environment", (v) => (v.environment = "staging")],
  ["tenant", (v) => (v.tenant = "tenant_other")],
  ["declared_risk", (v) => (v.declared_risk = "high")],
  ["data_classes", (v) => (v.data_classes = [...v.data_classes].reverse())],
  ["requested_capability", (v) => (v.requested_capability = "mailer.send.restricted")],
  ["resource_scope", (v) => (v.resource_scope = ["customer:123", "customer:456"])],
  ["payload_hash", (v) => (v.payload_hash = hash("c"))],
  ["artifact_hash", (v) => (v.artifact_hash = hash("d"))],
  ["adapter.id", (v) => (v.adapter.id = "adapter.other")],
  ["adapter.version", (v) => (v.adapter.version = "9.0.0")],
  ["request_id", (v) => (v.request_id = "req-other")],
  ["retry_of_receipt_id", (v) => (v.retry_of_receipt_id = "rcpt_retry")]
];
for (const [name, mutate] of intentMutations) {
  const changed = clone(base.submitted_intent);
  mutate(changed);
  assert(validateV02("submitted-intent", changed).ok, `${name} intent mutation remains valid`);
  assert(submittedIntentHashV02(changed) !== originalIntentHash, `${name} changes intent hash`);
}

console.log(`\nTyped action/hash tests passed (${n}).`);
