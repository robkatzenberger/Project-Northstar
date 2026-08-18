use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use tlpx::{
    assert_hash_string, bindings_match, canonicalize, canonicalize_json_text, digest_hex,
    hash_canonical, hash_json_text, utf8_hex, Adapter, AuthorizedAction, ExecutedAction, Risk,
    SubmittedIntent, Value,
};

#[derive(Debug, Deserialize)]
struct Golden {
    accept: Vec<Accept>,
    reject: Vec<Reject>,
}

#[derive(Debug, Deserialize)]
struct Accept {
    id: String,
    input_json: String,
    canonical: String,
    canonical_utf8_hex: String,
    digest_hex: BTreeMap<String, String>,
    sha256: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct Reject {
    id: String,
    input_json: String,
    error_contains: String,
}

#[derive(Debug, Deserialize)]
struct PolicyManifestGolden {
    manifest: serde_json::Value,
    policy_bundle_hash: String,
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/tlpx-0.2/jcs/golden.json")
}

fn load() -> Golden {
    let raw = fs::read_to_string(golden_path()).expect("golden.json");
    serde_json::from_str(&raw).expect("parse golden.json")
}

fn policy_manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tlpx-0.2/policy/manifest-golden.json")
}

#[test]
fn policy_manifest_hash_matches_js_oracle() {
    let raw = fs::read_to_string(policy_manifest_path()).expect("manifest-golden.json");
    let golden: PolicyManifestGolden = serde_json::from_str(&raw).expect("parse policy fixture");
    let manifest = serde_json::to_string(&golden.manifest).expect("serialize manifest");
    let got = hash_json_text("policy-bundle", &manifest).expect("hash policy manifest");
    assert_eq!(got, golden.policy_bundle_hash);
}

#[test]
fn accept_vectors_match_js_oracle() {
    let golden = load();
    assert!(!golden.accept.is_empty());
    for row in &golden.accept {
        let canonical =
            canonicalize_json_text(&row.input_json).unwrap_or_else(|e| panic!("{}: {e}", row.id));
        assert_eq!(canonical.as_str(), row.canonical, "{}", row.id);
        assert_eq!(
            utf8_hex(canonical.as_str()),
            row.canonical_utf8_hex,
            "{}",
            row.id
        );
        for (domain, expected) in &row.sha256 {
            assert_hash_string(expected).unwrap();
            let got = hash_canonical(domain, &canonical).unwrap();
            assert_eq!(got, *expected, "{} {domain}", row.id);
            assert_eq!(
                digest_hex(domain, &canonical).unwrap(),
                row.digest_hex[domain],
                "{} {domain} digest",
                row.id
            );
        }
    }
}

#[test]
fn reject_vectors_fail_closed() {
    let golden = load();
    assert!(!golden.reject.is_empty());
    for row in &golden.reject {
        let err =
            canonicalize_json_text(&row.input_json).expect_err(&format!("{} should fail", row.id));
        assert!(
            err.message().contains(&row.error_contains),
            "{}: expected {:?} in {}",
            row.id,
            row.error_contains,
            err.message()
        );
    }
}

#[test]
fn utf16_sorts_astral_before_bmp() {
    let text = r#"{"\uE000":1,"\uD800\uDC00":2}"#;
    let canonical = canonicalize_json_text(text).unwrap();
    let units: Vec<u16> = canonical.as_str().encode_utf16().collect();
    // after {' " '} first key starts with U+10000 lead 0xD800
    assert_eq!(units[0], '{' as u16);
    assert_eq!(units[1], '"' as u16);
    assert_eq!(units[2], 0xD800);
}

#[test]
fn domains_are_distinct() {
    let c = "{}";
    let a = hash_json_text("intent", c).unwrap();
    let b = hash_json_text("authorized-action", c).unwrap();
    let d = hash_json_text("executed-action", c).unwrap();
    let e = hash_json_text("approval-context", c).unwrap();
    let f = hash_json_text("policy-bundle", c).unwrap();
    let set = [a, b, d, e, f];
    for i in 0..set.len() {
        for j in 0..set.len() {
            if i != j {
                assert_ne!(set[i], set[j]);
            }
        }
    }
}

fn sample_actions() -> (SubmittedIntent, AuthorizedAction, ExecutedAction) {
    let adapter = Adapter {
        id: "adapter.mailer".into(),
        version: "1.0.0".into(),
    };
    let args = Value::Object(vec![("template".into(), Value::String("invoice".into()))]);
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
        data_classes: vec!["PII".into()],
        requested_capability: "mailer.send".into(),
        resource_scope: vec!["customer:123".into()],
        payload_hash: None,
        artifact_hash: None,
        adapter: adapter.clone(),
        request_id: "req-1".into(),
        retry_of_receipt_id: None,
    };
    let authorized = AuthorizedAction {
        requesting_principal: intent.requesting_principal.clone(),
        executing_principal: intent.executing_principal.clone(),
        action: intent.action.clone(),
        target: intent.target.clone(),
        arguments: args.clone(),
        environment: intent.environment.clone(),
        tenant: intent.tenant.clone(),
        derived_risk: Risk::High,
        effective_risk: Risk::High,
        risk_reasons: vec!["policy.high_data_class".into()],
        risk_source: "policy".into(),
        data_classes: intent.data_classes.clone(),
        capability: "mailer.send".into(),
        resource_scope: intent.resource_scope.clone(),
        payload_hash: None,
        artifact_hash: None,
        policy_bundle_hash: format!("sha256:{}", "a".repeat(64)),
        adapter: adapter.clone(),
    };
    let executed = ExecutedAction {
        executing_principal: authorized.executing_principal.clone(),
        action: authorized.action.clone(),
        target: authorized.target.clone(),
        arguments: args,
        environment: authorized.environment.clone(),
        tenant: authorized.tenant.clone(),
        payload_hash: None,
        artifact_hash: None,
        adapter,
    };
    (intent, authorized, executed)
}

#[test]
fn contract_types_use_binding_not_full_hash_equality() {
    let (intent, authorized, executed) = sample_actions();
    let ih = intent.intent_hash().unwrap();
    let ah = authorized.authorized_action_hash().unwrap();
    let eh = executed.executed_action_hash().unwrap();
    let bh = authorized.binding_hash().unwrap();
    assert_hash_string(&ih).unwrap();
    assert_ne!(
        ih, ah,
        "intent and authorized use different domains/objects"
    );
    assert_ne!(
        ah, eh,
        "full authorized hash must not equal executed/binding hash"
    );
    assert_eq!(bh, eh, "PEP compares Action Bindings under executed-action");
    assert!(bindings_match(&authorized, &executed).unwrap());
}

fn assert_binding_false(authorized: &AuthorizedAction, executed: ExecutedAction, name: &str) {
    assert!(
        !bindings_match(authorized, &executed).unwrap(),
        "{name} must participate in the Action Binding"
    );
}

#[test]
fn binding_mutations_mismatch() {
    let (_, authorized, executed) = sample_actions();
    assert!(bindings_match(&authorized, &executed).unwrap());

    let mut e = executed.clone();
    e.executing_principal = "agent.b".into();
    assert_binding_false(&authorized, e, "executor");

    let mut e = executed.clone();
    e.action = "send_sms".into();
    assert_binding_false(&authorized, e, "action");

    let mut e = executed.clone();
    e.target = "customer:999".into();
    assert_binding_false(&authorized, e, "target");

    let mut e = executed.clone();
    e.arguments = Value::Object(vec![("template".into(), Value::String("other".into()))]);
    assert_binding_false(&authorized, e, "arguments");

    let mut e = executed.clone();
    e.environment = "staging".into();
    assert_binding_false(&authorized, e, "environment");

    let mut e = executed.clone();
    e.tenant = "tenant_other".into();
    assert_binding_false(&authorized, e, "tenant");

    let mut e = executed.clone();
    e.payload_hash = Some(format!("sha256:{}", "c".repeat(64)));
    assert_binding_false(&authorized, e, "payload_hash");

    let mut e = executed.clone();
    e.artifact_hash = Some(format!("sha256:{}", "d".repeat(64)));
    assert_binding_false(&authorized, e, "artifact_hash");

    let mut e = executed.clone();
    e.adapter.id = "adapter.other".into();
    assert_binding_false(&authorized, e, "adapter");
}

#[test]
fn programmatic_value_cannot_bypass_jcs() {
    assert!(canonicalize(&Value::Int(i64::MAX)).is_err());
    let dup = Value::Object(vec![
        ("a".into(), Value::Int(1)),
        ("a".into(), Value::Int(2)),
    ]);
    assert!(canonicalize(&dup).is_err());
    assert!(hash_json_text("intent", r#"{"a":1,"a":2}"#).is_err());
}

#[test]
fn hashing_rejects_invalid_authorized_action() {
    let (_, mut authorized, _) = sample_actions();
    authorized.effective_risk = Risk::Low;
    authorized.derived_risk = Risk::High;
    assert!(authorized.authorized_action_hash().is_err());
    authorized.effective_risk = Risk::High;
    authorized.requesting_principal.clear();
    assert!(authorized.authorized_action_hash().is_err());
    authorized.requesting_principal = "agent.a".into();
    authorized.arguments = Value::String("nope".into());
    assert!(authorized.authorized_action_hash().is_err());
}
