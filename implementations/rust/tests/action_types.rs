use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use tlpx::{
    bindings_match, canonicalize, canonicalize_json_text, utf8_hex, Adapter, AuthorizedAction,
    ExecutedAction, Risk, SubmittedIntent, Value,
};

#[derive(Debug, Deserialize)]
struct Fixture {
    profile: String,
    standard_version: String,
    cases: Vec<ActionCase>,
}

#[derive(Debug, Deserialize)]
struct ActionCase {
    id: String,
    submitted_intent: serde_json::Value,
    authorized_action: serde_json::Value,
    executed_action: serde_json::Value,
    action_binding: serde_json::Value,
    canonical: ExpectedStrings,
    canonical_utf8_hex: ExpectedStrings,
    sha256: ExpectedHashes,
    digest_hex: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedStrings {
    submitted_intent: String,
    authorized_action: String,
    executed_action: String,
    action_binding: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedHashes {
    intent_hash: String,
    authorized_action_hash: String,
    executed_action_hash: String,
    action_binding_hash: String,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tlpx-0.2/actions/golden.json")
}

fn load_fixture() -> Fixture {
    let raw = fs::read_to_string(fixture_path()).expect("action golden fixture");
    serde_json::from_str(&raw).expect("parse action golden fixture")
}

fn hash(char: char) -> String {
    format!("sha256:{}", char.to_string().repeat(64))
}

fn adapter(id: &str, version: &str) -> Adapter {
    Adapter {
        id: id.into(),
        version: version.into(),
    }
}

fn null_digest_case() -> (SubmittedIntent, AuthorizedAction, ExecutedAction) {
    let args = Value::Object(vec![
        ("template".into(), Value::String("invoice-ready".into())),
        ("invoice_id".into(), Value::String("inv_456".into())),
    ]);
    let adapter = adapter("adapter.mailer", "1.0.0");
    let intent = SubmittedIntent {
        requesting_principal: "agent.a".into(),
        executing_principal: "agent.a".into(),
        action: "send_email".into(),
        intent_class: "external_communication".into(),
        target: "customer:123".into(),
        arguments: args.clone(),
        environment: "production".into(),
        tenant: "tenant_abc".into(),
        declared_risk: Risk::Medium,
        data_classes: vec!["PII".into(), "FINANCIAL".into()],
        requested_capability: "mailer.send".into(),
        resource_scope: vec!["customer:123".into()],
        payload_hash: None,
        artifact_hash: None,
        adapter: adapter.clone(),
        request_id: "req-typed-1".into(),
        retry_of_receipt_id: None,
    };
    let authorized = AuthorizedAction {
        requesting_principal: "agent.a".into(),
        executing_principal: "agent.a".into(),
        action: "send_email".into(),
        target: "customer:123".into(),
        arguments: args.clone(),
        environment: "production".into(),
        tenant: "tenant_abc".into(),
        derived_risk: Risk::Medium,
        effective_risk: Risk::High,
        risk_reasons: vec![
            "policy.external_communication".into(),
            "policy.financial_data".into(),
        ],
        risk_source: "policy".into(),
        data_classes: vec!["PII".into(), "FINANCIAL".into()],
        capability: "mailer.send".into(),
        resource_scope: vec!["customer:123".into()],
        payload_hash: None,
        artifact_hash: None,
        policy_bundle_hash: hash('e'),
        adapter: adapter.clone(),
    };
    let executed = ExecutedAction {
        executing_principal: "agent.a".into(),
        action: "send_email".into(),
        target: "customer:123".into(),
        arguments: args,
        environment: "production".into(),
        tenant: "tenant_abc".into(),
        payload_hash: None,
        artifact_hash: None,
        adapter,
    };
    (intent, authorized, executed)
}

fn bound_digest_case() -> (SubmittedIntent, AuthorizedAction, ExecutedAction) {
    let args = Value::Object(vec![
        (
            "content_ref".into(),
            Value::String("artifact:résumé".into()),
        ),
        ("mode".into(), Value::Int(416)),
        (
            "labels".into(),
            Value::Array(vec![
                Value::String("final".into()),
                Value::String("reviewed".into()),
            ]),
        ),
    ]);
    let adapter = adapter("adapter.filesystem", "2.1.0");
    let intent = SubmittedIntent {
        requesting_principal: "agent.é".into(),
        executing_principal: "agent.worker".into(),
        action: "write_file".into(),
        intent_class: "filesystem_change".into(),
        target: "/srv/reports/résumé.json".into(),
        arguments: args.clone(),
        environment: "staging".into(),
        tenant: "tenant_eu".into(),
        declared_risk: Risk::Low,
        data_classes: vec![],
        requested_capability: "filesystem.write".into(),
        resource_scope: vec!["/srv/reports".into(), "/srv/reports/résumé.json".into()],
        payload_hash: Some(hash('a')),
        artifact_hash: Some(hash('b')),
        adapter: adapter.clone(),
        request_id: "req-typed-2".into(),
        retry_of_receipt_id: Some("rcpt_previous_error".into()),
    };
    let authorized = AuthorizedAction {
        requesting_principal: "agent.é".into(),
        executing_principal: "agent.worker".into(),
        action: "write_file".into(),
        target: "/srv/reports/résumé.json".into(),
        arguments: args.clone(),
        environment: "staging".into(),
        tenant: "tenant_eu".into(),
        derived_risk: Risk::Low,
        effective_risk: Risk::Medium,
        risk_reasons: vec!["policy.mutable_target".into()],
        risk_source: "policy".into(),
        data_classes: vec![],
        capability: "filesystem.write".into(),
        resource_scope: vec!["/srv/reports".into(), "/srv/reports/résumé.json".into()],
        payload_hash: Some(hash('a')),
        artifact_hash: Some(hash('b')),
        policy_bundle_hash: hash('f'),
        adapter: adapter.clone(),
    };
    let executed = ExecutedAction {
        executing_principal: "agent.worker".into(),
        action: "write_file".into(),
        target: "/srv/reports/résumé.json".into(),
        arguments: args,
        environment: "staging".into(),
        tenant: "tenant_eu".into(),
        payload_hash: Some(hash('a')),
        artifact_hash: Some(hash('b')),
        adapter,
    };
    (intent, authorized, executed)
}

fn typed_case(id: &str) -> (SubmittedIntent, AuthorizedAction, ExecutedAction) {
    match id {
        "null-digests-no-retry" => null_digest_case(),
        "bound-digests-with-retry-and-unicode" => bound_digest_case(),
        _ => panic!("unknown action fixture case {id}"),
    }
}

fn generic_canonical(value: &serde_json::Value) -> String {
    let json = serde_json::to_string(value).expect("serialize fixture value");
    canonicalize_json_text(&json)
        .expect("canonicalize fixture value")
        .as_str()
        .to_string()
}

#[test]
fn typed_objects_match_shared_canonical_and_hash_fixture() {
    let fixture = load_fixture();
    assert_eq!(fixture.profile, "tlpx-action-types-v1");
    assert_eq!(fixture.standard_version, "0.2.0");
    assert_eq!(fixture.cases.len(), 2);

    for row in &fixture.cases {
        let (intent, authorized, executed) = typed_case(&row.id);
        let intent_canonical = canonicalize(&intent.to_value()).expect("typed intent canonical");
        let authorized_canonical =
            canonicalize(&authorized.to_value()).expect("typed authorized canonical");
        let executed_canonical =
            canonicalize(&executed.to_value()).expect("typed executed canonical");
        let binding_canonical =
            canonicalize(&authorized.binding().to_value()).expect("typed binding canonical");

        assert_eq!(
            generic_canonical(&row.submitted_intent),
            row.canonical.submitted_intent
        );
        assert_eq!(
            generic_canonical(&row.authorized_action),
            row.canonical.authorized_action
        );
        assert_eq!(
            generic_canonical(&row.executed_action),
            row.canonical.executed_action
        );
        assert_eq!(
            generic_canonical(&row.action_binding),
            row.canonical.action_binding
        );
        assert_eq!(intent_canonical.as_str(), row.canonical.submitted_intent);
        assert_eq!(
            authorized_canonical.as_str(),
            row.canonical.authorized_action
        );
        assert_eq!(executed_canonical.as_str(), row.canonical.executed_action);
        assert_eq!(binding_canonical.as_str(), row.canonical.action_binding);
        assert_eq!(
            utf8_hex(intent_canonical.as_str()),
            row.canonical_utf8_hex.submitted_intent
        );
        assert_eq!(
            utf8_hex(authorized_canonical.as_str()),
            row.canonical_utf8_hex.authorized_action
        );
        assert_eq!(
            utf8_hex(executed_canonical.as_str()),
            row.canonical_utf8_hex.executed_action
        );
        assert_eq!(
            utf8_hex(binding_canonical.as_str()),
            row.canonical_utf8_hex.action_binding
        );
        assert_eq!(intent.intent_hash().unwrap(), row.sha256.intent_hash);
        assert_eq!(
            authorized.authorized_action_hash().unwrap(),
            row.sha256.authorized_action_hash
        );
        assert_eq!(
            executed.executed_action_hash().unwrap(),
            row.sha256.executed_action_hash
        );
        assert_eq!(
            authorized.binding_hash().unwrap(),
            row.sha256.action_binding_hash
        );
        assert_eq!(
            row.sha256.executed_action_hash, row.sha256.action_binding_hash,
            "matching executed action is exactly the shared binding"
        );
        assert_ne!(
            row.sha256.authorized_action_hash, row.sha256.executed_action_hash,
            "full Authorized Action hash remains a different object/domain"
        );
        assert!(bindings_match(&authorized, &executed).unwrap());
        for (name, digest) in &row.digest_hex {
            let expected = match name.as_str() {
                "intent_hash" => &row.sha256.intent_hash,
                "authorized_action_hash" => &row.sha256.authorized_action_hash,
                "executed_action_hash" => &row.sha256.executed_action_hash,
                "action_binding_hash" => &row.sha256.action_binding_hash,
                _ => panic!("unknown digest {name}"),
            };
            assert_eq!(digest, &expected[7..]);
        }
    }
}

#[test]
fn required_null_digests_and_optional_retry_are_distinct() {
    let (without_retry, _, _) = null_digest_case();
    let Value::Object(without_fields) = without_retry.to_value() else {
        panic!("intent must be object")
    };
    assert!(without_fields
        .iter()
        .any(|(key, value)| { key == "payload_hash" && matches!(value, Value::Null) }));
    assert!(without_fields
        .iter()
        .any(|(key, value)| { key == "artifact_hash" && matches!(value, Value::Null) }));
    assert!(!without_fields
        .iter()
        .any(|(key, _)| key == "retry_of_receipt_id"));

    let (with_retry, _, _) = bound_digest_case();
    let Value::Object(with_fields) = with_retry.to_value() else {
        panic!("intent must be object")
    };
    assert!(with_fields.iter().any(|(key, value)| {
        key == "retry_of_receipt_id"
            && matches!(value, Value::String(id) if id == "rcpt_previous_error")
    }));
}

#[test]
fn action_binding_field_set_is_exact_and_authority_fields_are_excluded() {
    let (_, authorized, executed) = null_digest_case();
    let Value::Object(fields) = authorized.binding().to_value() else {
        panic!("binding must be object")
    };
    let mut names: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec![
            "action",
            "adapter",
            "arguments",
            "artifact_hash",
            "environment",
            "executing_principal",
            "payload_hash",
            "target",
            "tenant",
        ]
    );

    let original_full = authorized.authorized_action_hash().unwrap();
    let original_binding = authorized.binding_hash().unwrap();
    let mut authority_only = authorized.clone();
    authority_only.requesting_principal = "agent.other".into();
    authority_only.policy_bundle_hash = hash('9');
    assert_ne!(
        authority_only.authorized_action_hash().unwrap(),
        original_full
    );
    assert_eq!(authority_only.binding_hash().unwrap(), original_binding);
    assert!(bindings_match(&authority_only, &executed).unwrap());
}
