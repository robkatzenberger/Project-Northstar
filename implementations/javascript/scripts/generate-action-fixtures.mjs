/** Emit typed TL-PX 0.2 action-object fixtures for slice 3.2. */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { canonicalize, utf8Hex } from "../src/jcs.mjs";
import {
  actionBindingsMatchV02,
  authorizedActionBindingHashV02,
  authorizedActionBindingV02,
  authorizedActionHashV02,
  canonicalAuthorizedActionV02,
  canonicalExecutedActionV02,
  canonicalSubmittedIntentV02,
  executedActionHashV02,
  submittedIntentHashV02
} from "../src/action-v02.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const outPath = path.join(root, "tests/fixtures/tlpx-0.2/actions/golden.json");
const hash = (char) => `sha256:${char.repeat(64)}`;

const cases = [
  {
    id: "null-digests-no-retry",
    submitted_intent: {
      requesting_principal: "agent.a",
      executing_principal: "agent.a",
      action: "send_email",
      intent_class: "external_communication",
      target: "customer:123",
      arguments: { template: "invoice-ready", invoice_id: "inv_456" },
      environment: "production",
      tenant: "tenant_abc",
      declared_risk: "medium",
      data_classes: ["PII", "FINANCIAL"],
      requested_capability: "mailer.send",
      resource_scope: ["customer:123"],
      payload_hash: null,
      artifact_hash: null,
      adapter: { id: "adapter.mailer", version: "1.0.0" },
      request_id: "req-typed-1"
    },
    authorized_action: {
      requesting_principal: "agent.a",
      executing_principal: "agent.a",
      action: "send_email",
      target: "customer:123",
      arguments: { template: "invoice-ready", invoice_id: "inv_456" },
      environment: "production",
      tenant: "tenant_abc",
      derived_risk: "medium",
      effective_risk: "high",
      risk_reasons: ["policy.external_communication", "policy.financial_data"],
      risk_source: "policy",
      data_classes: ["PII", "FINANCIAL"],
      capability: "mailer.send",
      resource_scope: ["customer:123"],
      payload_hash: null,
      artifact_hash: null,
      policy_bundle_hash: hash("e"),
      adapter: { id: "adapter.mailer", version: "1.0.0" }
    },
    executed_action: {
      executing_principal: "agent.a",
      action: "send_email",
      target: "customer:123",
      arguments: { template: "invoice-ready", invoice_id: "inv_456" },
      environment: "production",
      tenant: "tenant_abc",
      payload_hash: null,
      artifact_hash: null,
      adapter: { id: "adapter.mailer", version: "1.0.0" }
    }
  },
  {
    id: "bound-digests-with-retry-and-unicode",
    submitted_intent: {
      requesting_principal: "agent.é",
      executing_principal: "agent.worker",
      action: "write_file",
      intent_class: "filesystem_change",
      target: "/srv/reports/résumé.json",
      arguments: {
        content_ref: "artifact:résumé",
        mode: 416,
        labels: ["final", "reviewed"]
      },
      environment: "staging",
      tenant: "tenant_eu",
      declared_risk: "low",
      data_classes: [],
      requested_capability: "filesystem.write",
      resource_scope: ["/srv/reports", "/srv/reports/résumé.json"],
      payload_hash: hash("a"),
      artifact_hash: hash("b"),
      adapter: { id: "adapter.filesystem", version: "2.1.0" },
      request_id: "req-typed-2",
      retry_of_receipt_id: "rcpt_previous_error"
    },
    authorized_action: {
      requesting_principal: "agent.é",
      executing_principal: "agent.worker",
      action: "write_file",
      target: "/srv/reports/résumé.json",
      arguments: {
        content_ref: "artifact:résumé",
        mode: 416,
        labels: ["final", "reviewed"]
      },
      environment: "staging",
      tenant: "tenant_eu",
      derived_risk: "low",
      effective_risk: "medium",
      risk_reasons: ["policy.mutable_target"],
      risk_source: "policy",
      data_classes: [],
      capability: "filesystem.write",
      resource_scope: ["/srv/reports", "/srv/reports/résumé.json"],
      payload_hash: hash("a"),
      artifact_hash: hash("b"),
      policy_bundle_hash: hash("f"),
      adapter: { id: "adapter.filesystem", version: "2.1.0" }
    },
    executed_action: {
      executing_principal: "agent.worker",
      action: "write_file",
      target: "/srv/reports/résumé.json",
      arguments: {
        content_ref: "artifact:résumé",
        mode: 416,
        labels: ["final", "reviewed"]
      },
      environment: "staging",
      tenant: "tenant_eu",
      payload_hash: hash("a"),
      artifact_hash: hash("b"),
      adapter: { id: "adapter.filesystem", version: "2.1.0" }
    }
  }
];

const enriched = cases.map((row) => {
  if (!actionBindingsMatchV02(row.authorized_action, row.executed_action)) {
    throw new Error(`${row.id}: fixture action bindings do not match`);
  }
  const actionBinding = authorizedActionBindingV02(row.authorized_action);
  const submittedCanonical = canonicalSubmittedIntentV02(row.submitted_intent);
  const authorizedCanonical = canonicalAuthorizedActionV02(row.authorized_action);
  const executedCanonical = canonicalExecutedActionV02(row.executed_action);
  const bindingCanonical = canonicalize(actionBinding);
  const intentHash = submittedIntentHashV02(row.submitted_intent);
  const authorizedHash = authorizedActionHashV02(row.authorized_action);
  const executedHash = executedActionHashV02(row.executed_action);
  const bindingHash = authorizedActionBindingHashV02(row.authorized_action);
  return {
    ...row,
    action_binding: actionBinding,
    canonical: {
      submitted_intent: submittedCanonical,
      authorized_action: authorizedCanonical,
      executed_action: executedCanonical,
      action_binding: bindingCanonical
    },
    canonical_utf8_hex: {
      submitted_intent: utf8Hex(submittedCanonical),
      authorized_action: utf8Hex(authorizedCanonical),
      executed_action: utf8Hex(executedCanonical),
      action_binding: utf8Hex(bindingCanonical)
    },
    sha256: {
      intent_hash: intentHash,
      authorized_action_hash: authorizedHash,
      executed_action_hash: executedHash,
      action_binding_hash: bindingHash
    },
    digest_hex: {
      intent_hash: intentHash.slice(7),
      authorized_action_hash: authorizedHash.slice(7),
      executed_action_hash: executedHash.slice(7),
      action_binding_hash: bindingHash.slice(7)
    }
  };
});

const fixture = {
  profile: "tlpx-action-types-v1",
  standard: "TL-PX",
  standard_version: "0.2.0",
  notes: [
    "Each source object validates against its distinct TL-PX 0.2 schema before canonicalization.",
    "payload_hash and artifact_hash are required fields whose values may be null; retry_of_receipt_id is optional and absence differs from null.",
    "authorized_action_hash covers the complete Authorized Action under its own domain.",
    "action_binding_hash and executed_action_hash use the executed-action domain over the exact shared Action Binding projection.",
    "A matching Action Binding does not make authorized_action_hash equal executed_action_hash."
  ],
  cases: enriched
};

fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(fixture, null, 2)}\n`);
console.log(`wrote ${outPath}`);
console.log(`cases=${enriched.length}`);
