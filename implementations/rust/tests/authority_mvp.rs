use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tlpx::{
    Adapter, Authority, AuthorityConfig, AuthorizationTemplate, AuthzState, CapabilityRegistry,
    Decision, ExecutedAction, PolicyBundle, PolicyEffect, PolicyRule, Principal, Risk,
    SubmittedIntent, Switchboard, Value,
};

const NOW: i64 = 1_800_000_000_000;

fn hash(ch: char) -> String {
    format!("sha256:{}", ch.to_string().repeat(64))
}

fn config() -> AuthorityConfig {
    AuthorityConfig {
        policy: PolicyBundle {
            id: "mvp-policy".into(),
            version: "1.0.0".into(),
            hash: hash('a'),
            rules: vec![
                PolicyRule {
                    id: "allow-email".into(),
                    action: "send_email".into(),
                    effect: PolicyEffect::allow(
                        "POLICY_ALLOW",
                        AuthorizationTemplate {
                            derived_risk: Risk::High,
                            capability: "mailer.send".into(),
                            resource_scope: vec!["customer:123".into()],
                            risk_reasons: vec!["external_communication".into()],
                            risk_source: "policy:mvp-policy@1.0.0".into(),
                        },
                    ),
                },
                PolicyRule {
                    id: "approval-deploy".into(),
                    action: "deploy".into(),
                    effect: PolicyEffect::require_approval("POLICY_REQUIRE_APPROVAL"),
                },
            ],
            default: PolicyEffect::deny("POLICY_DENY"),
        },
        switchboard: Switchboard::new(vec![
            Principal {
                id: "agent.requester".into(),
                active: true,
                allowed_actions: vec!["send_email".into(), "deploy".into(), "delete".into()],
            },
            Principal {
                id: "runtime.mailer".into(),
                active: true,
                allowed_actions: vec!["send_email".into(), "deploy".into(), "delete".into()],
            },
        ])
        .unwrap(),
        capabilities: CapabilityRegistry::new(vec![(
            "mailer.send".into(),
            vec!["adapter.mailer".into()],
        )])
        .unwrap(),
        claim_window_ms: 5_000,
        execution_lease_ms: 30_000,
    }
}

fn config_with_principal(principal: Principal) -> AuthorityConfig {
    let mut value = config();
    value.switchboard = Switchboard::new(vec![
        Principal {
            id: "agent.requester".into(),
            active: true,
            allowed_actions: vec!["send_email".into(), "deploy".into(), "delete".into()],
        },
        Principal {
            id: "runtime.mailer".into(),
            active: true,
            allowed_actions: vec!["send_email".into(), "deploy".into(), "delete".into()],
        },
        principal,
    ])
    .unwrap();
    value
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

fn temp_db(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "northstar-{label}-{}-{nonce}.sqlite",
        std::process::id()
    ))
}

fn clean_db(path: &Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}

#[test]
fn evaluate_allow_issues_authority_owned_binding_and_ids() {
    let authority = Authority::in_memory(config()).unwrap();
    let outcome = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-allow"), NOW)
        .unwrap();
    assert_eq!(outcome.decision, Decision::Allow);
    let issued = outcome.authorization.unwrap();
    assert!(issued.authorization_id.starts_with("authz_"));
    assert!(issued.authorization_nonce.starts_with("nonce_"));
    assert_ne!(issued.authorization_id, issued.authorization_nonce);
    assert_ne!(issued.authorized_action_hash, issued.action_binding_hash);
    assert_eq!(issued.claim_expires_at_ms, NOW + 5_000);
    assert_eq!(issued.state, AuthzState::AuthorizedUnclaimed);
}

#[test]
fn deterministic_deny_and_pending_issue_no_authorization() {
    let authority = Authority::in_memory(config()).unwrap();
    let mut denied = intent("req-deny");
    denied.action = "delete".into();
    let denied = authority
        .evaluate_and_issue_at("agent.requester", &denied, NOW)
        .unwrap();
    assert_eq!(denied.decision, Decision::Deny);
    assert!(denied.authorization.is_none());

    let mut pending = intent("req-pending");
    pending.action = "deploy".into();
    let pending = authority
        .evaluate_and_issue_at("agent.requester", &pending, NOW)
        .unwrap();
    assert_eq!(pending.decision, Decision::RequireApproval);
    assert!(pending.authorization.is_none());
}

#[test]
fn policy_cannot_silently_replace_requested_capability() {
    let authority = Authority::in_memory(config()).unwrap();
    let mut request = intent("req-capability-mismatch");
    request.requested_capability = "admin.root".into();
    let outcome = authority
        .evaluate_and_issue_at("agent.requester", &request, NOW)
        .unwrap();
    assert_eq!(outcome.decision, Decision::Deny);
    assert_eq!(outcome.reason_code, "POLICY_CAPABILITY_MISMATCH");
    assert!(outcome.authorization.is_none());
}

#[test]
fn malformed_authorization_templates_fail_activation() {
    let mut duplicate_scope = config();
    let template = duplicate_scope.policy.rules[0]
        .effect
        .authorization
        .as_mut()
        .unwrap();
    template.resource_scope.push("customer:123".into());
    let error = Authority::in_memory(duplicate_scope).err().unwrap();
    assert_eq!(error.code(), "POLICY_COMPILE_FAILED");
    assert!(error.message().contains("unique"));

    let mut unknown_capability = config();
    unknown_capability.policy.rules[0]
        .effect
        .authorization
        .as_mut()
        .unwrap()
        .capability = "mailer.unregistered".into();
    let error = Authority::in_memory(unknown_capability).err().unwrap();
    assert_eq!(error.code(), "POLICY_COMPILE_FAILED");
    assert!(error.message().contains("capability registry"));

    let mut missing_policy_id = config();
    missing_policy_id.policy.id.clear();
    let error = Authority::in_memory(missing_policy_id).err().unwrap();
    assert_eq!(error.code(), "POLICY_COMPILE_FAILED");

    let mut invented_reason = config();
    invented_reason.policy.rules[0].effect.reason_code = "LOCAL_ALLOW_ALIAS".into();
    let error = Authority::in_memory(invented_reason).err().unwrap();
    assert_eq!(error.code(), "POLICY_COMPILE_FAILED");
    assert!(error.message().contains("POLICY_ALLOW"));
}

#[test]
fn switchboard_refusals_are_durable_denies() {
    let authority = Authority::in_memory(config()).unwrap();

    let mut unknown = intent("req-unknown-principal");
    unknown.requesting_principal = "agent.unknown".into();
    let unknown = authority
        .evaluate_and_issue_at("agent.unknown", &unknown, NOW)
        .unwrap();
    assert_eq!(unknown.decision, Decision::Deny);
    assert_eq!(unknown.reason_code, "SWITCHBOARD_UNKNOWN_PRINCIPAL");

    let mut inactive = intent("req-inactive-principal");
    inactive.requesting_principal = "agent.inactive".into();
    let inactive_authority = Authority::in_memory(config_with_principal(Principal {
        id: "agent.inactive".into(),
        active: false,
        allowed_actions: vec!["send_email".into()],
    }))
    .unwrap();
    let inactive = inactive_authority
        .evaluate_and_issue_at("agent.inactive", &inactive, NOW)
        .unwrap();
    assert_eq!(inactive.decision, Decision::Deny);
    assert_eq!(inactive.reason_code, "SWITCHBOARD_PRINCIPAL_INACTIVE");

    let mut action_denied = intent("req-action-denied");
    action_denied.requesting_principal = "agent.denied".into();
    let denied_authority = Authority::in_memory(config_with_principal(Principal {
        id: "agent.denied".into(),
        active: true,
        allowed_actions: vec![],
    }))
    .unwrap();
    let action_denied = denied_authority
        .evaluate_and_issue_at("agent.denied", &action_denied, NOW)
        .unwrap();
    assert_eq!(action_denied.decision, Decision::Deny);
    assert_eq!(action_denied.reason_code, "SWITCHBOARD_ACTION_DENIED");
}

#[test]
fn stored_switchboard_deny_cannot_drift_to_allow_after_reconfiguration() {
    let path = temp_db("switchboard-deny-restart");
    let mut request = intent("req-switchboard-stable");
    request.requesting_principal = "agent.new".into();
    let first = {
        let authority = Authority::open(&path, config()).unwrap();
        authority
            .evaluate_and_issue_at("agent.new", &request, NOW)
            .unwrap()
    };
    assert_eq!(first.reason_code, "SWITCHBOARD_UNKNOWN_PRINCIPAL");

    let reopened = Authority::open(
        &path,
        config_with_principal(Principal {
            id: "agent.new".into(),
            active: true,
            allowed_actions: vec!["send_email".into()],
        }),
    )
    .unwrap();
    let repeated = reopened
        .evaluate_and_issue_at("agent.new", &request, NOW + 1)
        .unwrap();
    assert_eq!(repeated.receipt_id, first.receipt_id);
    assert_eq!(repeated.decision, Decision::Deny);
    assert!(repeated.authorization.is_none());
    drop(reopened);
    clean_db(&path);
}

#[test]
fn request_constraint_refusal_is_durable_and_requires_a_new_request_id() {
    let authority = Authority::in_memory(config()).unwrap();
    let mut request = intent("req-target-refusal");
    request.target = "customer:999".into();
    let first = authority
        .evaluate_and_issue_at("agent.requester", &request, NOW)
        .unwrap();
    assert_eq!(first.decision, Decision::Deny);
    assert_eq!(first.reason_code, "POLICY_TARGET_OUT_OF_SCOPE");

    let repeated = authority
        .evaluate_and_issue_at("agent.requester", &request, NOW + 1)
        .unwrap();
    assert_eq!(repeated.receipt_id, first.receipt_id);

    request.target = "customer:123".into();
    let conflict = authority
        .evaluate_and_issue_at("agent.requester", &request, NOW + 2)
        .unwrap_err();
    assert_eq!(conflict.code(), "IDEMPOTENCY_CONFLICT");
    assert!(conflict.receipt_id().is_some());

    request.request_id = "req-target-corrected".into();
    let corrected = authority
        .evaluate_and_issue_at("agent.requester", &request, NOW + 3)
        .unwrap();
    assert_eq!(corrected.decision, Decision::Allow);
}

#[test]
fn authentication_error_is_durable_without_poisoning_the_proposed_principal() {
    let authority = Authority::in_memory(config_with_principal(Principal {
        id: "agent.victim".into(),
        active: true,
        allowed_actions: vec!["send_email".into()],
    }))
    .unwrap();
    let mut proposed_victim = intent("req-shared-name");
    proposed_victim.requesting_principal = "agent.victim".into();

    let first = authority
        .evaluate_and_issue_at("agent.requester", &proposed_victim, NOW)
        .unwrap_err();
    assert_eq!(first.code(), "AUTHENTICATION_FAILED");
    let first_receipt = first.receipt_id().unwrap().to_string();
    assert!(first.sequence().is_some());

    let repeated = authority
        .evaluate_and_issue_at("agent.requester", &proposed_victim, NOW + 1)
        .unwrap_err();
    assert_eq!(repeated.receipt_id(), Some(first_receipt.as_str()));

    let victim = authority
        .evaluate_and_issue_at("agent.victim", &proposed_victim, NOW + 2)
        .unwrap();
    assert_eq!(victim.decision, Decision::Allow);

    let mut corrected_attacker = proposed_victim.clone();
    corrected_attacker.requesting_principal = "agent.requester".into();
    let conflict = authority
        .evaluate_and_issue_at("agent.requester", &corrected_attacker, NOW + 3)
        .unwrap_err();
    assert_eq!(conflict.code(), "IDEMPOTENCY_CONFLICT");

    corrected_attacker.request_id = "req-auth-retry".into();
    corrected_attacker.retry_of_receipt_id = Some(first_receipt.clone());
    let retry = authority
        .evaluate_and_issue_at("agent.requester", &corrected_attacker, NOW + 4)
        .unwrap();
    assert_eq!(retry.decision, Decision::Allow);
    assert_eq!(
        retry.retry_of_receipt_id.as_deref(),
        Some(first_receipt.as_str())
    );

    let mut cross_principal = proposed_victim;
    cross_principal.request_id = "req-cross-principal-retry".into();
    cross_principal.retry_of_receipt_id = Some(first_receipt);
    let invalid_link = authority
        .evaluate_and_issue_at("agent.victim", &cross_principal, NOW + 5)
        .unwrap_err();
    assert_eq!(invalid_link.code(), "ACTION_DATA_AMBIGUOUS");
}

#[test]
fn evaluation_error_survives_restart() {
    let path = temp_db("error-restart");
    let config = config_with_principal(Principal {
        id: "agent.victim".into(),
        active: true,
        allowed_actions: vec!["send_email".into()],
    });
    let mut proposed_victim = intent("req-error-restart");
    proposed_victim.requesting_principal = "agent.victim".into();
    let first = {
        let authority = Authority::open(&path, config.clone()).unwrap();
        authority
            .evaluate_and_issue_at("agent.requester", &proposed_victim, NOW)
            .unwrap_err()
    };
    let reopened = Authority::open(&path, config).unwrap();
    let repeated = reopened
        .evaluate_and_issue_at("agent.requester", &proposed_victim, NOW + 1)
        .unwrap_err();
    assert_eq!(repeated.code(), "AUTHENTICATION_FAILED");
    assert_eq!(repeated.receipt_id(), first.receipt_id());
    assert_eq!(repeated.sequence(), first.sequence());
    drop(reopened);
    clean_db(&path);
}

#[test]
fn post_decision_sql_failure_becomes_durable_evaluation_error() {
    let path = temp_db("authorization-persistence-error");
    let authority = Authority::open(&path, config()).unwrap();
    let injector = Connection::open(&path).unwrap();
    injector
        .execute_batch(
            "CREATE TRIGGER reject_authorization
             BEFORE INSERT ON tlpx_authorizations
             BEGIN SELECT RAISE(FAIL, 'injected authorization failure'); END;",
        )
        .unwrap();
    drop(injector);

    let request = intent("req-persistence-error");
    let first = authority
        .evaluate_and_issue_at("agent.requester", &request, NOW)
        .unwrap_err();
    assert_eq!(first.code(), "AUTHORITY_INTERNAL_ERROR");
    assert!(first.receipt_id().is_some());
    assert!(first.sequence().is_some());
    assert_eq!(authority.evaluation_event_count().unwrap(), 1);

    let repeated = authority
        .evaluate_and_issue_at("agent.requester", &request, NOW + 1)
        .unwrap_err();
    assert_eq!(repeated.receipt_id(), first.receipt_id());
    assert_eq!(repeated.sequence(), first.sequence());
    drop(authority);
    clean_db(&path);
}

#[test]
fn never_error_is_immutable_and_cannot_be_a_retry_parent() {
    let authority = Authority::in_memory(config()).unwrap();
    let mut invalid = intent("req-invalid");
    invalid.action.clear();
    let first = authority
        .evaluate_and_issue_at("agent.requester", &invalid, NOW)
        .unwrap_err();
    assert_eq!(first.code(), "INTENT_INVALID");
    let first_receipt = first.receipt_id().unwrap().to_string();

    invalid.action = "send_email".into();
    let repeated = authority
        .evaluate_and_issue_at("agent.requester", &invalid, NOW + 1)
        .unwrap_err();
    assert_eq!(repeated.code(), "INTENT_INVALID");
    assert_eq!(repeated.receipt_id(), Some(first_receipt.as_str()));

    let mut successor = intent("req-invalid-successor");
    successor.retry_of_receipt_id = Some(first_receipt);
    let error = authority
        .evaluate_and_issue_at("agent.requester", &successor, NOW + 2)
        .unwrap_err();
    assert_eq!(error.code(), "ACTION_DATA_AMBIGUOUS");

    let mut missing_parent = intent("req-missing-retry-parent");
    missing_parent.retry_of_receipt_id = Some("rcpt_does_not_exist".into());
    let error = authority
        .evaluate_and_issue_at("agent.requester", &missing_parent, NOW + 3)
        .unwrap_err();
    assert_eq!(error.code(), "ACTION_DATA_AMBIGUOUS");
    assert!(error.receipt_id().is_some());
}

#[test]
fn authority_sequence_orders_decisions_errors_and_claims() {
    let authority = Authority::in_memory(config()).unwrap();
    let allowed = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-sequence-allow"), NOW)
        .unwrap();
    let issued = allowed.authorization.clone().unwrap();

    let error = authority
        .evaluate_and_issue_at("agent.impostor", &intent("req-sequence-error"), NOW + 1)
        .unwrap_err();
    let conflict = {
        let mut changed = intent("req-sequence-allow");
        changed.target = "customer:999".into();
        authority
            .evaluate_and_issue_at("agent.requester", &changed, NOW + 2)
            .unwrap_err()
    };
    let claim = authority
        .claim_at(
            &issued.authorization_id,
            "runtime.mailer",
            &executed(),
            NOW + 3,
        )
        .unwrap();

    assert!(allowed.sequence < error.sequence().unwrap());
    assert!(error.sequence().unwrap() < conflict.sequence().unwrap());
    assert!(conflict.sequence().unwrap() < claim.sequence);
    assert_eq!(authority.evaluation_event_count().unwrap(), 3);
}

#[test]
fn switchboard_and_authenticated_requester_precede_policy() {
    let authority = Authority::in_memory(config()).unwrap();
    let error = authority
        .evaluate_and_issue_at("agent.impostor", &intent("req-authn"), NOW)
        .unwrap_err();
    assert_eq!(error.code(), "AUTHENTICATION_FAILED");
}

#[test]
fn idempotency_returns_existing_and_rejects_mutation() {
    let authority = Authority::in_memory(config()).unwrap();
    let original = intent("req-idempotent");
    let first = authority
        .evaluate_and_issue_at("agent.requester", &original, NOW)
        .unwrap();
    let repeated = authority
        .evaluate_and_issue_at("agent.requester", &original, NOW + 10)
        .unwrap();
    assert_eq!(first.receipt_id, repeated.receipt_id);
    assert_eq!(
        first.authorization.unwrap().authorization_id,
        repeated.authorization.unwrap().authorization_id
    );

    let mut changed = original;
    changed.target = "customer:999".into();
    let error = authority
        .evaluate_and_issue_at("agent.requester", &changed, NOW + 20)
        .unwrap_err();
    assert_eq!(error.code(), "IDEMPOTENCY_CONFLICT");
}

#[test]
fn idempotent_authorization_preserves_resource_scope_order() {
    let mut ordered_config = config();
    ordered_config.policy.rules[0]
        .effect
        .authorization
        .as_mut()
        .unwrap()
        .resource_scope = vec!["customer:z".into(), "customer:123".into()];
    let authority = Authority::in_memory(ordered_config).unwrap();
    let request = intent("req-scope-order");
    let first = authority
        .evaluate_and_issue_at("agent.requester", &request, NOW)
        .unwrap()
        .authorization
        .unwrap();
    let repeated = authority
        .evaluate_and_issue_at("agent.requester", &request, NOW + 1)
        .unwrap()
        .authorization
        .unwrap();
    assert_eq!(first.resource_scope, repeated.resource_scope);
    assert_eq!(
        repeated.resource_scope,
        vec!["customer:z".to_string(), "customer:123".to_string()]
    );
}

#[test]
fn idempotent_result_survives_policy_and_scope_change() {
    let path = temp_db("idempotent-restart");
    let original = intent("req-idempotent-restart");
    let first = {
        let authority = Authority::open(&path, config()).unwrap();
        authority
            .evaluate_and_issue_at("agent.requester", &original, NOW)
            .unwrap()
    };

    let mut changed = config();
    changed.policy.hash = hash('b');
    changed.policy.rules.clear();
    changed.switchboard = Switchboard::new(vec![Principal {
        id: "agent.requester".into(),
        active: true,
        allowed_actions: vec![],
    }])
    .unwrap();
    let reopened = Authority::open(&path, changed).unwrap();
    let repeated = reopened
        .evaluate_and_issue_at("agent.requester", &original, NOW + 1)
        .unwrap();

    assert_eq!(repeated.receipt_id, first.receipt_id);
    assert_eq!(repeated.policy_bundle_hash, first.policy_bundle_hash);
    assert_eq!(
        repeated.authorization.unwrap().authorization_id,
        first.authorization.unwrap().authorization_id
    );
    drop(reopened);
    clean_db(&path);
}

#[test]
fn claim_is_exact_authenticated_and_single_use() {
    let authority = Authority::in_memory(config()).unwrap();
    let issued = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-claim"), NOW)
        .unwrap()
        .authorization
        .unwrap();
    let claim = authority
        .claim_at(
            &issued.authorization_id,
            "runtime.mailer",
            &executed(),
            NOW + 1,
        )
        .unwrap();
    assert_eq!(claim.action_binding_hash, claim.executed_action_hash);
    assert_eq!(
        authority.state(&issued.authorization_id).unwrap(),
        Some(AuthzState::Claimed)
    );
    let replay = authority
        .claim_at(
            &issued.authorization_id,
            "runtime.mailer",
            &executed(),
            NOW + 2,
        )
        .unwrap_err();
    assert_eq!(replay.code(), "ALREADY_CLAIMED");
}

#[test]
fn mismatch_wrong_executor_and_scope_do_not_consume() {
    let authority = Authority::in_memory(config()).unwrap();
    let issued = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-negative"), NOW)
        .unwrap()
        .authorization
        .unwrap();

    let wrong_executor = authority
        .claim_at(
            &issued.authorization_id,
            "agent.requester",
            &executed(),
            NOW + 1,
        )
        .unwrap_err();
    assert_eq!(wrong_executor.code(), "EXECUTOR_MISMATCH");

    let mut mutation = executed();
    mutation.arguments = Value::Object(vec![(
        "template".into(),
        Value::String("wire-instructions".into()),
    )]);
    let mismatch = authority
        .claim_at(
            &issued.authorization_id,
            "runtime.mailer",
            &mutation,
            NOW + 2,
        )
        .unwrap_err();
    assert_eq!(mismatch.code(), "ACTION_MISMATCH");

    let mut out_of_scope = executed();
    out_of_scope.target = "customer:999".into();
    let scope = authority
        .claim_at(
            &issued.authorization_id,
            "runtime.mailer",
            &out_of_scope,
            NOW + 3,
        )
        .unwrap_err();
    assert_eq!(scope.code(), "AUTHORIZATION_SCOPE_DENIED");

    let mut malformed = executed();
    malformed.action.clear();
    let malformed = authority
        .claim_at(
            &issued.authorization_id,
            "runtime.mailer",
            &malformed,
            NOW + 4,
        )
        .unwrap_err();
    assert_eq!(malformed.code(), "ACTION_MISMATCH");
    assert_eq!(
        authority.state(&issued.authorization_id).unwrap(),
        Some(AuthzState::AuthorizedUnclaimed)
    );
}

#[test]
fn expiry_and_revocation_fail_closed() {
    let authority = Authority::in_memory(config()).unwrap();
    let expiring = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-expire"), NOW)
        .unwrap()
        .authorization
        .unwrap();
    let expired = authority
        .claim_at(
            &expiring.authorization_id,
            "runtime.mailer",
            &executed(),
            NOW + 5_000,
        )
        .unwrap_err();
    assert_eq!(expired.code(), "AUTHORIZATION_EXPIRED");
    assert_eq!(
        authority.state(&expiring.authorization_id).unwrap(),
        Some(AuthzState::Expired)
    );

    let revoked = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-revoke"), NOW)
        .unwrap()
        .authorization
        .unwrap();
    authority
        .revoke_at(&revoked.authorization_id, NOW + 1)
        .unwrap();
    let error = authority
        .claim_at(
            &revoked.authorization_id,
            "runtime.mailer",
            &executed(),
            NOW + 2,
        )
        .unwrap_err();
    assert_eq!(error.code(), "AUTHORIZATION_REVOKED");
}

#[test]
fn concurrent_claim_has_one_winner() {
    let authority = Arc::new(Authority::in_memory(config()).unwrap());
    let issued = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-race"), NOW)
        .unwrap()
        .authorization
        .unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for offset in [1_i64, 2_i64] {
        let authority = Arc::clone(&authority);
        let barrier = Arc::clone(&barrier);
        let authorization_id = issued.authorization_id.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            authority.claim_at(
                &authorization_id,
                "runtime.mailer",
                &executed(),
                NOW + offset,
            )
        }));
    }
    barrier.wait();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
}

#[test]
fn authorization_survives_restart() {
    let path = temp_db("restart");
    let issued = {
        let authority = Authority::open(&path, config()).unwrap();
        authority
            .evaluate_and_issue_at("agent.requester", &intent("req-restart"), NOW)
            .unwrap()
            .authorization
            .unwrap()
    };
    let reopened = Authority::open(&path, config()).unwrap();
    let claim = reopened
        .claim_at(
            &issued.authorization_id,
            "runtime.mailer",
            &executed(),
            NOW + 1,
        )
        .unwrap();
    assert_eq!(claim.authorization_id, issued.authorization_id);
    drop(reopened);
    clean_db(&path);
}

#[test]
fn separate_connections_share_one_idempotent_evaluation() {
    let path = temp_db("evaluation-connections");
    let authorities = [
        Authority::open(&path, config()).unwrap(),
        Authority::open(&path, config()).unwrap(),
    ];
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for (offset, authority) in [0_i64, 1_i64].into_iter().zip(authorities) {
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier.wait();
            authority.evaluate_and_issue_at(
                "agent.requester",
                &intent("req-evaluation-race"),
                NOW + offset,
            )
        }));
    }
    barrier.wait();
    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap().unwrap())
        .collect();
    assert_eq!(outcomes[0].receipt_id, outcomes[1].receipt_id);
    assert_eq!(outcomes[0].sequence, outcomes[1].sequence);
    assert_eq!(
        outcomes[0].authorization.as_ref().unwrap().authorization_id,
        outcomes[1].authorization.as_ref().unwrap().authorization_id
    );
    let reopened = Authority::open(&path, config()).unwrap();
    assert_eq!(reopened.evaluation_event_count().unwrap(), 1);
    drop(reopened);
    clean_db(&path);
}

#[test]
fn separate_sqlite_connections_still_have_one_claim_winner() {
    let path = temp_db("connections");
    let issuer = Authority::open(&path, config()).unwrap();
    let issued = issuer
        .evaluate_and_issue_at("agent.requester", &intent("req-db-race"), NOW)
        .unwrap()
        .authorization
        .unwrap();
    drop(issuer);

    let authorities = [
        Authority::open(&path, config()).unwrap(),
        Authority::open(&path, config()).unwrap(),
    ];
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for (offset, authority) in [1_i64, 2_i64].into_iter().zip(authorities) {
        let barrier = Arc::clone(&barrier);
        let authorization_id = issued.authorization_id.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            authority.claim_at(
                &authorization_id,
                "runtime.mailer",
                &executed(),
                NOW + offset,
            )
        }));
    }
    barrier.wait();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    clean_db(&path);
}
