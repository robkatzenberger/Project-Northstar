use serde_json::Value as JsonValue;
use std::fs;
use std::path::PathBuf;
use tlpx::{
    exact_match_policy_content_hash, Adapter, Authority, AuthorityConfig, AuthorizationTemplate,
    CapabilityRegistry, ConfiguredPolicyBundle, Decision, EvidenceConfig, ExecutedAction,
    PartyType, PolicyBundle, PolicyBundleManifest, PolicyCatalog, PolicyEffect, PolicyIssuer,
    PolicyIssuerType, PolicyRule, PolicySupersedes, Principal, Risk, SubmittedIntent, Switchboard,
    Value, EXACT_MATCH_POLICY_CONTENT_TYPE, POLICY_PRECEDENCE,
};

const NOW: i64 = 1_800_000_000_000;

fn exact_match_policy() -> PolicyBundle {
    PolicyBundle {
        rules: vec![PolicyRule {
            id: "allow-email".into(),
            action: "send_email".into(),
            effect: PolicyEffect::allow(
                "POLICY_ALLOW",
                AuthorizationTemplate {
                    derived_risk: Risk::High,
                    capability: "mailer.send".into(),
                    resource_scope: vec!["customer:123".into()],
                    risk_reasons: vec!["external_communication".into()],
                    risk_source: "policy:authority-test".into(),
                },
            ),
        }],
        default: PolicyEffect::deny("POLICY_DENY"),
    }
}

fn configured(id: &str, version: &str, activated_at: &str) -> ConfiguredPolicyBundle {
    let policy = exact_match_policy();
    let content_hash = exact_match_policy_content_hash(&policy).unwrap();
    ConfiguredPolicyBundle {
        manifest: PolicyBundleManifest {
            policy_bundle_id: id.into(),
            policy_bundle_version: version.into(),
            issuer: PolicyIssuer {
                id: "security.platform".into(),
                kind: PolicyIssuerType::Human,
            },
            content_type: EXACT_MATCH_POLICY_CONTENT_TYPE.into(),
            content_hash,
            activated_at: activated_at.into(),
            retired_at: None,
            environment: "production".into(),
            tenant: "tenant_abc".into(),
            precedence: POLICY_PRECEDENCE
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            default_decision: policy.default.decision,
            supersedes: None,
        },
        policy,
    }
}

fn config(bundles: Vec<ConfiguredPolicyBundle>) -> AuthorityConfig {
    AuthorityConfig {
        policy: PolicyCatalog::new(bundles),
        switchboard: Switchboard::new(vec![
            Principal {
                id: "agent.requester".into(),
                active: true,
                allowed_actions: vec!["send_email".into()],
            },
            Principal {
                id: "runtime.mailer".into(),
                active: true,
                allowed_actions: vec!["send_email".into()],
            },
        ])
        .unwrap(),
        capabilities: CapabilityRegistry::new(vec![(
            "mailer.send".into(),
            vec!["adapter.mailer".into()],
        )])
        .unwrap(),
        evidence: EvidenceConfig {
            evaluator_id: "authority.local".into(),
            router_id: "switchboard.local".into(),
            requester_type: PartyType::Machine,
            seal_key_id: "audit-test-v1".into(),
            seal_key: vec![0x41; 32],
        },
        approval_window_ms: 600_000,
        claim_window_ms: 5_000,
        execution_lease_ms: 30_000,
    }
}

fn intent(request_id: &str) -> SubmittedIntent {
    SubmittedIntent {
        requesting_principal: "agent.requester".into(),
        executing_principal: "runtime.mailer".into(),
        action: "send_email".into(),
        intent_class: "external_communication".into(),
        target: "customer:123".into(),
        arguments: Value::Object(vec![(
            "template".into(),
            Value::String("invoice-ready".into()),
        )]),
        environment: "production".into(),
        tenant: "tenant_abc".into(),
        declared_risk: Risk::Medium,
        data_classes: vec!["PII".into()],
        requested_capability: "mailer.send".into(),
        resource_scope: vec!["customer:123".into()],
        payload_hash: None,
        artifact_hash: None,
        adapter: Adapter {
            id: "adapter.mailer".into(),
            version: "1.0.0".into(),
        },
        request_id: request_id.into(),
        retry_of_receipt_id: None,
    }
}

fn executed() -> ExecutedAction {
    let source = intent("unused");
    ExecutedAction {
        executing_principal: source.executing_principal,
        action: source.action,
        target: source.target,
        arguments: source.arguments,
        environment: source.environment,
        tenant: source.tenant,
        payload_hash: source.payload_hash,
        artifact_hash: source.artifact_hash,
        adapter: source.adapter,
    }
}

#[test]
fn native_manifest_parser_matches_the_js_golden_hash() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tlpx-0.2/policy/manifest-golden.json");
    let raw = fs::read_to_string(path).unwrap();
    let wrapper: JsonValue = serde_json::from_str(&raw).unwrap();
    let manifest_json = serde_json::to_string(&wrapper["manifest"]).unwrap();
    let manifest = PolicyBundleManifest::from_json(&manifest_json).unwrap();
    assert_eq!(
        manifest.manifest_hash().unwrap(),
        wrapper["policy_bundle_hash"].as_str().unwrap()
    );

    let mut invalid = manifest;
    invalid.precedence.reverse();
    assert_eq!(
        invalid.validate().unwrap_err().code(),
        "POLICY_PROVENANCE_INVALID"
    );
}

#[test]
fn authority_uses_the_selected_manifest_hash() {
    let configured = configured("authority-policy", "1.0.0", "2020-01-01T00:00:00.000Z");
    let expected = configured.manifest.manifest_hash().unwrap();
    let authority = Authority::in_memory(config(vec![configured])).unwrap();
    let outcome = authority
        .evaluate_and_issue_at("agent.requester", &intent("manifest-bound"), NOW)
        .unwrap();
    assert_eq!(outcome.decision, Decision::Allow);
    assert_eq!(outcome.policy_bundle_hash, expected);
    assert_eq!(
        outcome.authorization.unwrap().policy_bundle_hash,
        outcome.policy_bundle_hash
    );
}

#[test]
fn startup_rejects_content_and_supersession_provenance_mismatch() {
    let mut content_mismatch = configured("authority-policy", "1.0.0", "2020-01-01T00:00:00.000Z");
    content_mismatch.manifest.content_hash = format!("sha256:{}", "a".repeat(64));
    let error = Authority::in_memory(config(vec![content_mismatch]))
        .err()
        .unwrap();
    assert_eq!(error.code(), "POLICY_PROVENANCE_INVALID");
    assert!(error.message().contains("content_hash mismatch"));

    let first = configured("authority-policy", "1.0.0", "2020-01-01T00:00:00.000Z");
    let mut successor = configured("authority-policy", "2.0.0", "2021-01-01T00:00:00.000Z");
    successor.manifest.supersedes = Some(PolicySupersedes {
        policy_bundle_id: "authority-policy".into(),
        policy_bundle_version: "1.0.0".into(),
        policy_bundle_hash: format!("sha256:{}", "b".repeat(64)),
    });
    let error = Authority::in_memory(config(vec![first, successor]))
        .err()
        .unwrap();
    assert_eq!(error.code(), "POLICY_PROVENANCE_INVALID");
    assert!(error.message().contains("predecessor hash mismatch"));

    let mut cycle_a = configured("cycle-policy", "3.0.0", "2022-01-01T00:00:00.000Z");
    let mut cycle_b = configured("cycle-policy", "4.0.0", "2023-01-01T00:00:00.000Z");
    cycle_a.manifest.supersedes = Some(PolicySupersedes {
        policy_bundle_id: "cycle-policy".into(),
        policy_bundle_version: "4.0.0".into(),
        policy_bundle_hash: format!("sha256:{}", "c".repeat(64)),
    });
    cycle_b.manifest.supersedes = Some(PolicySupersedes {
        policy_bundle_id: "cycle-policy".into(),
        policy_bundle_version: "3.0.0".into(),
        policy_bundle_hash: format!("sha256:{}", "d".repeat(64)),
    });
    let error = Authority::in_memory(config(vec![cycle_a, cycle_b]))
        .err()
        .unwrap();
    assert_eq!(error.code(), "POLICY_PROVENANCE_INVALID");
    assert!(error.message().contains("cycle"));
}

#[test]
fn unavailable_and_ambiguous_policy_fail_with_durable_error_evidence() {
    let mut wrong_scope = configured("staging-policy", "1.0.0", "2020-01-01T00:00:00.000Z");
    wrong_scope.manifest.environment = "staging".into();
    let authority = Authority::in_memory(config(vec![wrong_scope])).unwrap();
    let error = authority
        .evaluate_and_issue_at("agent.requester", &intent("unavailable-policy"), NOW)
        .unwrap_err();
    assert_eq!(error.code(), "POLICY_UNAVAILABLE");
    assert!(error.receipt_id().is_some());
    assert!(error.sequence().is_some());
    let rows = authority.pending_evidence(10).unwrap();
    assert_eq!(rows.len(), 1);
    let record: JsonValue = serde_json::from_str(&rows[0].record_json).unwrap();
    assert_eq!(record["error_code"], "POLICY_UNAVAILABLE");
    assert!(record.get("policy_bundle_id").is_none());

    let authority = Authority::in_memory(config(vec![
        configured("policy-a", "1.0.0", "2020-01-01T00:00:00.000Z"),
        configured("policy-b", "99.0.0", "2020-01-02T00:00:00.000Z"),
    ]))
    .unwrap();
    let error = authority
        .evaluate_and_issue_at("agent.requester", &intent("ambiguous-policy"), NOW)
        .unwrap_err();
    assert_eq!(error.code(), "POLICY_PRECEDENCE_AMBIGUOUS");
    assert!(error.receipt_id().is_some());
}

#[test]
fn explicit_hash_bound_supersession_selects_the_unique_winner() {
    let first = configured("authority-policy", "1.0.0", "2020-01-01T00:00:00.000Z");
    let first_hash = first.manifest.manifest_hash().unwrap();
    let mut successor = configured("authority-policy", "2.0.0", "2021-01-01T00:00:00.000Z");
    successor.manifest.supersedes = Some(PolicySupersedes {
        policy_bundle_id: "authority-policy".into(),
        policy_bundle_version: "1.0.0".into(),
        policy_bundle_hash: first_hash,
    });
    let expected = successor.manifest.manifest_hash().unwrap();
    let authority = Authority::in_memory(config(vec![first, successor])).unwrap();
    let outcome = authority
        .evaluate_and_issue_at("agent.requester", &intent("explicit-successor"), NOW)
        .unwrap();
    assert_eq!(outcome.policy_bundle_hash, expected);
}

#[test]
fn claim_rechecks_that_the_issuing_policy_is_still_active() {
    let mut configured = configured("authority-policy", "1.0.0", "2020-01-01T00:00:00.000Z");
    configured.manifest.retired_at = Some("2027-01-15T08:00:01.000Z".into());
    let authority = Authority::in_memory(config(vec![configured])).unwrap();
    let issued = authority
        .evaluate_and_issue_at("agent.requester", &intent("retiring-policy"), NOW)
        .unwrap()
        .authorization
        .unwrap();
    let error = authority
        .claim_at(
            &issued.authorization_id,
            "runtime.mailer",
            &executed(),
            NOW + 1_500,
        )
        .unwrap_err();
    assert_eq!(error.code(), "POLICY_INACTIVE");
}
