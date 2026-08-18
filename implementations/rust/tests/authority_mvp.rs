use rusqlite::Connection;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tlpx::{
    exact_match_policy_content_hash, Adapter, AdapterContract, AdapterRegistry, ApprovalOutcome,
    ApprovalPresentation, ApprovalState, AuthenticatedAdapterSession, AuthenticatedIdentity,
    Authority, AuthorityConfig, AuthorizationTemplate, AuthzState, CancellationOutcome,
    CancellationReason, CancellationRole, CapabilityRegistry, ConfiguredPolicyBundle, Decision,
    EvidenceConfig, ExecutedAction, ExecutionResultEvidence, ExecutionState, LocalAuthenticator,
    LocalPrincipalMapping, LocalRole, PartyType, PolicyBundle, PolicyBundleManifest, PolicyCatalog,
    PolicyEffect, PolicyIssuer, PolicyIssuerType, PolicyRule, Principal, RevocationReason,
    RevocationScope, Risk, SubmittedIntent, Switchboard, Value, ADAPTER_MATERIAL_FIELDS,
    EXACT_MATCH_POLICY_CONTENT_TYPE, POLICY_PRECEDENCE,
};

const NOW: i64 = 1_800_000_000_000;

fn hash(ch: char) -> String {
    format!("sha256:{}", ch.to_string().repeat(64))
}

fn authenticated_identity(
    principal_id: &str,
    party_type: PartyType,
    roles: Vec<LocalRole>,
    approval_routes: Vec<&str>,
) -> AuthenticatedIdentity {
    let authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: principal_id.into(),
        party_type,
        roles,
        approval_routes: approval_routes.into_iter().map(str::to_string).collect(),
    }])
    .unwrap();
    let (server, client) = UnixStream::pair().unwrap();
    let identity = authenticator.authenticate_stream(&server).unwrap();
    drop(client);
    identity
}

fn config() -> AuthorityConfig {
    let policy = PolicyBundle {
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
                effect: PolicyEffect::require_approval(
                    "POLICY_REQUIRE_APPROVAL",
                    AuthorizationTemplate {
                        derived_risk: Risk::High,
                        capability: "mailer.send".into(),
                        resource_scope: vec!["customer:123".into()],
                        risk_reasons: vec!["deployment_change".into()],
                        risk_source: "policy:mvp-policy@1.0.0".into(),
                    },
                    vec!["ops.deploy".into()],
                ),
            },
        ],
        default: PolicyEffect::deny("POLICY_DENY"),
    };
    AuthorityConfig {
        policy: PolicyCatalog::new(vec![configured_policy("mvp-policy", "1.0.0", policy)]),
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
        adapters: AdapterRegistry::new(vec![AdapterContract {
            adapter_id: "adapter.mailer".into(),
            adapter_version: "1.0.0".into(),
            authenticated_principal: "adapter.mailer.local".into(),
            authenticated_authority: "authority.local".into(),
            binary_hash: hash('8'),
            capabilities: vec!["mailer.send".into()],
            actions: vec!["send_email".into(), "deploy".into()],
            material_fields: ADAPTER_MATERIAL_FIELDS
                .iter()
                .map(|field| (*field).to_string())
                .collect(),
        }])
        .unwrap(),
        evidence: EvidenceConfig {
            evaluator_id: "authority.local".into(),
            router_id: "switchboard.local".into(),
            requester_type: PartyType::Machine,
            seal_key_id: "audit-test-v1".into(),
            seal_key: vec![0x5a; 32],
        },
        approval_window_ms: 600_000,
        claim_window_ms: 5_000,
        execution_lease_ms: 30_000,
    }
}

fn configured_policy(id: &str, version: &str, policy: PolicyBundle) -> ConfiguredPolicyBundle {
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
            activated_at: "2020-01-01T00:00:00.000Z".into(),
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

fn rebind_policy(config: &mut AuthorityConfig) {
    let configured = &mut config.policy.bundles[0];
    configured.manifest.content_hash = exact_match_policy_content_hash(&configured.policy).unwrap();
    configured.manifest.default_decision = configured.policy.default.decision;
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
fn approval_routes_are_validated_and_policy_content_bound() {
    let base = config().policy.bundles[0].policy.clone();
    let base_hash = exact_match_policy_content_hash(&base).unwrap();
    let mut changed = base.clone();
    changed.rules[1].effect.approval_route = Some(vec!["security.deploy".into()]);
    assert_ne!(
        exact_match_policy_content_hash(&changed).unwrap(),
        base_hash
    );

    let mut missing = base;
    missing.rules[1].effect.approval_route = Some(vec![]);
    let error = exact_match_policy_content_hash(&missing).unwrap_err();
    assert_eq!(error.code(), "POLICY_COMPILE_FAILED");
}

#[test]
fn local_peer_authentication_uses_kernel_uid_gid_and_rejects_unknown_peers() {
    let identity = authenticated_identity(
        "agent.requester",
        PartyType::Machine,
        vec![LocalRole::Requester],
        vec![],
    );
    assert_eq!(identity.principal_id(), "agent.requester");
    assert_eq!(identity.uid(), nix::unistd::Uid::effective().as_raw());
    assert_eq!(identity.gid(), nix::unistd::Gid::effective().as_raw());
    assert!(identity.has_role(LocalRole::Requester));
    assert!(!identity.has_role(LocalRole::Executor));

    let unknown = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw().wrapping_add(1),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: "agent.other".into(),
        party_type: PartyType::Machine,
        roles: vec![LocalRole::Requester],
        approval_routes: vec![],
    }])
    .unwrap();
    let (server, _client) = UnixStream::pair().unwrap();
    let error = unknown.authenticate_stream(&server).unwrap_err();
    assert_eq!(error.code(), "AUTHENTICATION_FAILED");
}

#[test]
fn authenticated_facade_enforces_requester_and_executor_roles() {
    let authority = Authority::in_memory(config()).unwrap();
    let requester = authenticated_identity(
        "agent.requester",
        PartyType::Machine,
        vec![LocalRole::Requester],
        vec![],
    );
    let executor = authenticated_identity(
        "runtime.mailer",
        PartyType::Machine,
        vec![LocalRole::Executor],
        vec![],
    );
    let outcome = authority
        .evaluate_authenticated_at(&requester, &intent("req-local-auth"), NOW)
        .unwrap();
    let issued = outcome.authorization.unwrap();

    let wrong_role = authority
        .claim_authenticated_at(&issued.authorization_id, &requester, &executed(), NOW + 1)
        .unwrap_err();
    assert_eq!(wrong_role.code(), "AUTHENTICATION_FAILED");
    assert_eq!(
        authority.state(&issued.authorization_id).unwrap(),
        Some(AuthzState::AuthorizedUnclaimed)
    );

    let claim = authority
        .claim_authenticated_at(&issued.authorization_id, &executor, &executed(), NOW + 2)
        .unwrap();
    assert_eq!(claim.executing_principal, "runtime.mailer");

    let cannot_request = authority
        .evaluate_authenticated_at(&executor, &intent("req-wrong-role"), NOW + 3)
        .unwrap_err();
    assert_eq!(cannot_request.code(), "AUTHENTICATION_FAILED");
}

#[test]
fn pending_cancellation_is_authenticated_route_scoped_atomic_and_evidenced() {
    let requester = authenticated_identity(
        "agent.requester",
        PartyType::Machine,
        vec![LocalRole::Requester],
        vec![],
    );
    let operator = authenticated_identity(
        "operator.deploy",
        PartyType::Human,
        vec![LocalRole::Operator],
        vec!["ops.deploy"],
    );
    let wrong_operator = authenticated_identity(
        "operator.other",
        PartyType::Human,
        vec![LocalRole::Operator],
        vec!["ops.other"],
    );
    let emergency = authenticated_identity(
        "authority.emergency",
        PartyType::Machine,
        vec![LocalRole::EmergencyCanceller],
        vec![],
    );

    let authority = Authority::in_memory(config()).unwrap();
    let mut pending_intent = intent("req-cancel-requester");
    pending_intent.action = "deploy".into();
    let pending = authority
        .evaluate_authenticated_at(&requester, &pending_intent, NOW)
        .unwrap();
    assert_eq!(pending.decision, Decision::RequireApproval);

    let wrong_reason = authority
        .cancel_pending_authenticated_at(
            &pending.receipt_id,
            &requester,
            CancellationReason::OperatorCancelled,
            NOW + 1,
        )
        .unwrap_err();
    assert_eq!(wrong_reason.code(), "CANCELLATION_UNAUTHORIZED");
    let wrong_route = authority
        .cancel_pending_authenticated_at(
            &pending.receipt_id,
            &wrong_operator,
            CancellationReason::OperatorCancelled,
            NOW + 2,
        )
        .unwrap_err();
    assert_eq!(wrong_route.code(), "CANCELLATION_UNAUTHORIZED");

    let cancelled = authority
        .cancel_pending_authenticated_at(
            &pending.receipt_id,
            &requester,
            CancellationReason::RequesterWithdrawn,
            NOW + 3,
        )
        .unwrap();
    assert_eq!(cancelled.canceller_role, CancellationRole::Requester);
    assert!(cancelled.sequence > pending.sequence);
    let repeated = authority
        .cancel_pending_authenticated_at(
            &pending.receipt_id,
            &requester,
            CancellationReason::RequesterWithdrawn,
            NOW + 4,
        )
        .unwrap_err();
    assert_eq!(repeated.code(), "APPROVAL_TERMINAL");
    let evidence = authority.pending_evidence(10).unwrap();
    let operator_record = evidence
        .iter()
        .find(|row| row.record_type == "tlpx.operator_action")
        .unwrap();
    let operator_json: serde_json::Value =
        serde_json::from_str(&operator_record.record_json).unwrap();
    assert_eq!(operator_json["outcome"], "CANCEL");
    assert_eq!(operator_json["operator"]["id"], "agent.requester");

    let operator_authority = Authority::in_memory(config()).unwrap();
    let mut operator_intent = intent("req-cancel-operator");
    operator_intent.action = "deploy".into();
    let operator_pending = operator_authority
        .evaluate_authenticated_at(&requester, &operator_intent, NOW + 10)
        .unwrap();
    let operator_cancel = operator_authority
        .cancel_pending_authenticated_at(
            &operator_pending.receipt_id,
            &operator,
            CancellationReason::OperatorCancelled,
            NOW + 11,
        )
        .unwrap();
    assert_eq!(operator_cancel.canceller_role, CancellationRole::Operator);

    let emergency_authority = Authority::in_memory(config()).unwrap();
    let mut emergency_intent = intent("req-cancel-emergency");
    emergency_intent.action = "deploy".into();
    let emergency_pending = emergency_authority
        .evaluate_authenticated_at(&requester, &emergency_intent, NOW + 20)
        .unwrap();
    let emergency_cancel = emergency_authority
        .cancel_pending_authenticated_at(
            &emergency_pending.receipt_id,
            &emergency,
            CancellationReason::EmergencyRevocation,
            NOW + 21,
        )
        .unwrap();
    assert_eq!(
        emergency_cancel.canceller_role,
        CancellationRole::EmergencyAuthority
    );
}

#[test]
fn concurrent_pending_cancellation_has_one_terminal_winner() {
    let path = temp_db("cancel-race");
    let requester = authenticated_identity(
        "agent.requester",
        PartyType::Machine,
        vec![LocalRole::Requester],
        vec![],
    );
    let receipt_id = {
        let authority = Authority::open(&path, config()).unwrap();
        let mut request = intent("req-cancel-race");
        request.action = "deploy".into();
        authority
            .evaluate_authenticated_at(&requester, &request, NOW)
            .unwrap()
            .receipt_id
    };
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for offset in [1_i64, 2_i64] {
        let path = path.clone();
        let receipt_id = receipt_id.clone();
        let requester = requester.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let authority = Authority::open(path, config()).unwrap();
            barrier.wait();
            authority.cancel_pending_authenticated_at(
                &receipt_id,
                &requester,
                CancellationReason::RequesterWithdrawn,
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
    let loser = results
        .iter()
        .find_map(|result| result.as_ref().err())
        .unwrap();
    assert_eq!(loser.code(), "APPROVAL_TERMINAL");
    let authority = Authority::open(&path, config()).unwrap();
    let evidence = authority.pending_evidence(10).unwrap();
    assert_eq!(
        evidence
            .iter()
            .filter(|row| row.record_type == "tlpx.operator_action")
            .count(),
        1
    );
    clean_db(&path);
}

#[test]
fn human_approval_binds_the_rendered_action_and_issues_a_fresh_authorization() {
    let requester = authenticated_identity(
        "agent.requester",
        PartyType::Machine,
        vec![LocalRole::Requester],
        vec![],
    );
    let operator = authenticated_identity(
        "operator.deploy",
        PartyType::Human,
        vec![LocalRole::Operator],
        vec!["ops.deploy"],
    );
    let machine_operator = authenticated_identity(
        "machine.operator",
        PartyType::Machine,
        vec![LocalRole::Operator],
        vec!["ops.deploy"],
    );
    let executor = authenticated_identity(
        "runtime.mailer",
        PartyType::Machine,
        vec![LocalRole::Executor],
        vec![],
    );
    let authority = Authority::in_memory(config()).unwrap();
    let mut request = intent("req-approve");
    request.action = "deploy".into();
    let pending = authority
        .evaluate_authenticated_at(&requester, &request, NOW)
        .unwrap();
    assert_eq!(pending.decision, Decision::RequireApproval);
    assert!(pending.authorization.is_none());

    let machine_view = authority
        .pending_approval_authenticated_at(&pending.receipt_id, &machine_operator, NOW + 1)
        .unwrap_err();
    assert_eq!(machine_view.code(), "APPROVAL_UNAUTHORIZED");
    let view = authority
        .pending_approval_authenticated_at(&pending.receipt_id, &operator, NOW + 2)
        .unwrap();
    assert_eq!(view.action, "deploy");
    assert_eq!(view.approval_route, vec!["ops.deploy"]);
    assert_eq!(view.approval_expires_at_ms, NOW + 600_000);
    assert!(view
        .authorized_action_json
        .contains("\"action\":\"deploy\""));

    let wrong_display = authority
        .resolve_pending_authenticated_at(
            &pending.receipt_id,
            &operator,
            ApprovalOutcome::Approve,
            ApprovalPresentation {
                authorized_action_hash: hash('f'),
                renderer_id: "approval.terminal".into(),
                renderer_version: "1.0.0".into(),
            },
            NOW + 3,
        )
        .unwrap_err();
    assert_eq!(wrong_display.code(), "APPROVAL_PRESENTATION_MISMATCH");
    assert_eq!(
        authority.approval_state(&pending.receipt_id).unwrap(),
        Some(ApprovalState::Pending)
    );

    let resolution = authority
        .resolve_pending_authenticated_at(
            &pending.receipt_id,
            &operator,
            ApprovalOutcome::Approve,
            ApprovalPresentation {
                authorized_action_hash: view.authorized_action_hash.clone(),
                renderer_id: "approval.terminal".into(),
                renderer_version: "1.0.0".into(),
            },
            NOW + 1_000,
        )
        .unwrap();
    let issued = resolution.authorization.as_ref().unwrap();
    assert_eq!(issued.issued_at_ms, NOW + 1_000);
    assert_eq!(issued.claim_expires_at_ms, NOW + 6_000);
    assert!(issued.authorization_id.starts_with("authz_"));
    assert!(issued.authorization_nonce.starts_with("nonce_"));
    assert_eq!(issued.authorized_action_hash, view.authorized_action_hash);
    assert_eq!(
        authority.approval_state(&pending.receipt_id).unwrap(),
        Some(ApprovalState::Approved)
    );

    let records = authority.pending_evidence(10).unwrap();
    let operator_record = records
        .iter()
        .find(|row| row.record_type == "tlpx.operator_action")
        .unwrap();
    let operator_json: serde_json::Value =
        serde_json::from_str(&operator_record.record_json).unwrap();
    assert_eq!(operator_json["outcome"], "APPROVE");
    assert_eq!(
        operator_json["authorized_action_hash"],
        view.authorized_action_hash
    );
    assert_eq!(operator_json["renderer_id"], "approval.terminal");
    assert!(records.iter().any(|row| {
        row.record_type == "tlpx.authorization" && row.source_id == issued.authorization_id
    }));

    let mut presented = executed();
    presented.action = "deploy".into();
    let claim = authority
        .claim_authenticated_at(&issued.authorization_id, &executor, &presented, NOW + 2_000)
        .unwrap();
    assert_eq!(claim.authorization_id, issued.authorization_id);
}

#[test]
fn human_rejection_is_terminal_and_issues_no_authorization() {
    let requester = authenticated_identity(
        "agent.requester",
        PartyType::Machine,
        vec![LocalRole::Requester],
        vec![],
    );
    let operator = authenticated_identity(
        "operator.deploy",
        PartyType::Human,
        vec![LocalRole::Operator],
        vec!["ops.deploy"],
    );
    let authority = Authority::in_memory(config()).unwrap();
    let mut request = intent("req-reject");
    request.action = "deploy".into();
    let pending = authority
        .evaluate_authenticated_at(&requester, &request, NOW)
        .unwrap();
    let view = authority
        .pending_approval_authenticated_at(&pending.receipt_id, &operator, NOW + 1)
        .unwrap();
    let rejected = authority
        .resolve_pending_authenticated_at(
            &pending.receipt_id,
            &operator,
            ApprovalOutcome::Reject,
            ApprovalPresentation {
                authorized_action_hash: view.authorized_action_hash.clone(),
                renderer_id: "approval.terminal".into(),
                renderer_version: "1.0.0".into(),
            },
            NOW + 2,
        )
        .unwrap();
    assert!(rejected.authorization.is_none());
    assert_eq!(
        authority.approval_state(&pending.receipt_id).unwrap(),
        Some(ApprovalState::Rejected)
    );
    let late = authority
        .resolve_pending_authenticated_at(
            &pending.receipt_id,
            &operator,
            ApprovalOutcome::Approve,
            ApprovalPresentation {
                authorized_action_hash: view.authorized_action_hash,
                renderer_id: "approval.terminal".into(),
                renderer_version: "1.0.0".into(),
            },
            NOW + 3,
        )
        .unwrap_err();
    assert_eq!(late.code(), "APPROVAL_TERMINAL");
    let evidence = authority.pending_evidence(10).unwrap();
    assert_eq!(
        evidence
            .iter()
            .filter(|row| row.record_type == "tlpx.authorization")
            .count(),
        0
    );
    let operator_json: serde_json::Value = serde_json::from_str(
        &evidence
            .iter()
            .find(|row| row.record_type == "tlpx.operator_action")
            .unwrap()
            .record_json,
    )
    .unwrap();
    assert_eq!(operator_json["outcome"], "REJECT");
}

#[test]
fn approval_expiry_closes_the_pending_request_before_issuance() {
    let requester = authenticated_identity(
        "agent.requester",
        PartyType::Machine,
        vec![LocalRole::Requester],
        vec![],
    );
    let operator = authenticated_identity(
        "operator.deploy",
        PartyType::Human,
        vec![LocalRole::Operator],
        vec!["ops.deploy"],
    );
    let authority = Authority::in_memory(config()).unwrap();
    let mut request = intent("req-expire-approval");
    request.action = "deploy".into();
    let pending = authority
        .evaluate_authenticated_at(&requester, &request, NOW)
        .unwrap();
    let view = authority
        .pending_approval_authenticated_at(&pending.receipt_id, &operator, NOW + 1)
        .unwrap();
    let early = authority
        .expire_pending_at(&pending.receipt_id, NOW + 599_999)
        .unwrap_err();
    assert_eq!(early.code(), "APPROVAL_NOT_EXPIRED");
    let expired = authority
        .resolve_pending_authenticated_at(
            &pending.receipt_id,
            &operator,
            ApprovalOutcome::Approve,
            ApprovalPresentation {
                authorized_action_hash: view.authorized_action_hash,
                renderer_id: "approval.terminal".into(),
                renderer_version: "1.0.0".into(),
            },
            NOW + 600_000,
        )
        .unwrap_err();
    assert_eq!(expired.code(), "APPROVAL_EXPIRED");
    assert_eq!(
        authority.approval_state(&pending.receipt_id).unwrap(),
        Some(ApprovalState::Expired)
    );
    let late_cancel = authority
        .cancel_pending_authenticated_at(
            &pending.receipt_id,
            &requester,
            CancellationReason::RequesterWithdrawn,
            NOW + 600_001,
        )
        .unwrap_err();
    assert_eq!(late_cancel.code(), "APPROVAL_TERMINAL");
    let evidence = authority.pending_evidence(10).unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].record_type, "tlpx.decision");
}

#[test]
fn concurrent_approve_and_reject_have_one_terminal_winner() {
    let path = temp_db("approval-race");
    let requester = authenticated_identity(
        "agent.requester",
        PartyType::Machine,
        vec![LocalRole::Requester],
        vec![],
    );
    let operator = authenticated_identity(
        "operator.deploy",
        PartyType::Human,
        vec![LocalRole::Operator],
        vec!["ops.deploy"],
    );
    let (receipt_id, action_hash) = {
        let authority = Authority::open(&path, config()).unwrap();
        let mut request = intent("req-approval-race");
        request.action = "deploy".into();
        let pending = authority
            .evaluate_authenticated_at(&requester, &request, NOW)
            .unwrap();
        let view = authority
            .pending_approval_authenticated_at(&pending.receipt_id, &operator, NOW + 1)
            .unwrap();
        (pending.receipt_id, view.authorized_action_hash)
    };
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for outcome in [ApprovalOutcome::Approve, ApprovalOutcome::Reject] {
        let path = path.clone();
        let receipt_id = receipt_id.clone();
        let action_hash = action_hash.clone();
        let operator = operator.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let authority = Authority::open(path, config()).unwrap();
            barrier.wait();
            authority.resolve_pending_authenticated_at(
                &receipt_id,
                &operator,
                outcome,
                ApprovalPresentation {
                    authorized_action_hash: action_hash,
                    renderer_id: "approval.terminal".into(),
                    renderer_version: "1.0.0".into(),
                },
                NOW + 2,
            )
        }));
    }
    barrier.wait();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .find_map(|result| result.as_ref().err())
            .unwrap()
            .code(),
        "APPROVAL_TERMINAL"
    );
    let authority = Authority::open(&path, config()).unwrap();
    let evidence = authority.pending_evidence(10).unwrap();
    assert_eq!(
        evidence
            .iter()
            .filter(|row| row.record_type == "tlpx.operator_action")
            .count(),
        1
    );
    assert!(matches!(
        authority.approval_state(&receipt_id).unwrap(),
        Some(ApprovalState::Approved | ApprovalState::Rejected)
    ));
    clean_db(&path);
}

#[test]
fn concurrent_cancellation_and_approval_expiry_have_one_terminal_winner() {
    let path = temp_db("cancel-expiry-race");
    let requester = authenticated_identity(
        "agent.requester",
        PartyType::Machine,
        vec![LocalRole::Requester],
        vec![],
    );
    let receipt_id = {
        let authority = Authority::open(&path, config()).unwrap();
        let mut request = intent("req-cancel-expiry-race");
        request.action = "deploy".into();
        authority
            .evaluate_authenticated_at(&requester, &request, NOW)
            .unwrap()
            .receipt_id
    };
    let barrier = Arc::new(Barrier::new(3));
    let cancel_handle = {
        let path = path.clone();
        let receipt_id = receipt_id.clone();
        let requester = requester.clone();
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            let authority = Authority::open(path, config()).unwrap();
            barrier.wait();
            authority
                .cancel_pending_authenticated_at(
                    &receipt_id,
                    &requester,
                    CancellationReason::RequesterWithdrawn,
                    NOW + 599_999,
                )
                .map(|_| ())
        })
    };
    let expiry_handle = {
        let path = path.clone();
        let receipt_id = receipt_id.clone();
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            let authority = Authority::open(path, config()).unwrap();
            barrier.wait();
            authority
                .expire_pending_at(&receipt_id, NOW + 600_000)
                .map(|_| ())
        })
    };
    barrier.wait();
    let results = [cancel_handle.join().unwrap(), expiry_handle.join().unwrap()];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .find_map(|result| result.as_ref().err())
            .unwrap()
            .code(),
        "APPROVAL_TERMINAL"
    );
    let authority = Authority::open(&path, config()).unwrap();
    assert!(matches!(
        authority.approval_state(&receipt_id).unwrap(),
        Some(ApprovalState::Cancelled | ApprovalState::Expired)
    ));
    assert!(authority.pending_evidence(10).is_ok());
    clean_db(&path);
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
    let template = duplicate_scope.policy.bundles[0].policy.rules[0]
        .effect
        .authorization
        .as_mut()
        .unwrap();
    template.resource_scope.push("customer:123".into());
    let error = Authority::in_memory(duplicate_scope).err().unwrap();
    assert_eq!(error.code(), "POLICY_COMPILE_FAILED");
    assert!(error.message().contains("unique"));

    let mut unknown_capability = config();
    unknown_capability.policy.bundles[0].policy.rules[0]
        .effect
        .authorization
        .as_mut()
        .unwrap()
        .capability = "mailer.unregistered".into();
    rebind_policy(&mut unknown_capability);
    let error = Authority::in_memory(unknown_capability).err().unwrap();
    assert_eq!(error.code(), "POLICY_COMPILE_FAILED");
    assert!(error.message().contains("capability registry"));

    let mut missing_policy_id = config();
    missing_policy_id.policy.bundles[0]
        .manifest
        .policy_bundle_id
        .clear();
    let error = Authority::in_memory(missing_policy_id).err().unwrap();
    assert_eq!(error.code(), "POLICY_PROVENANCE_INVALID");

    let mut invented_reason = config();
    invented_reason.policy.bundles[0].policy.rules[0]
        .effect
        .reason_code = "LOCAL_ALLOW_ALIAS".into();
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
    ordered_config.policy.bundles[0].policy.rules[0]
        .effect
        .authorization
        .as_mut()
        .unwrap()
        .resource_scope = vec!["customer:z".into(), "customer:123".into()];
    rebind_policy(&mut ordered_config);
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
    changed.policy.bundles[0].manifest.policy_bundle_version = "2.0.0".into();
    changed.policy.bundles[0].policy.rules.clear();
    rebind_policy(&mut changed);
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
fn authenticated_scoped_revocations_block_every_later_unclaimed_use() {
    let revoker = authenticated_identity(
        "authority.emergency",
        PartyType::Machine,
        vec![LocalRole::EmergencyCanceller],
        vec![],
    );
    let cases = [
        (RevocationScope::Authorization, "authorization"),
        (RevocationScope::Principal, "principal"),
        (RevocationScope::PolicyBundle, "policy"),
        (RevocationScope::Tenant, "tenant"),
        (RevocationScope::Environment, "environment"),
        (RevocationScope::Capability, "capability"),
    ];
    for (index, (scope, label)) in cases.into_iter().enumerate() {
        let authority = Authority::in_memory(config()).unwrap();
        let issued = authority
            .evaluate_and_issue_at(
                "agent.requester",
                &intent(&format!("req-revoke-{label}")),
                NOW,
            )
            .unwrap()
            .authorization
            .unwrap();
        let scope_id = match scope {
            RevocationScope::Authorization => issued.authorization_id.clone(),
            RevocationScope::Principal => issued.executing_principal.clone(),
            RevocationScope::PolicyBundle => issued.policy_bundle_hash.clone(),
            RevocationScope::Tenant => issued.tenant.clone(),
            RevocationScope::Environment => issued.environment.clone(),
            RevocationScope::Capability => issued.capability.clone(),
            RevocationScope::SigningKey => unreachable!(),
        };
        let record = authority
            .revoke_authenticated_at(
                &revoker,
                scope,
                &scope_id,
                RevocationReason::EmergencyDeny,
                NOW + index as i64 + 1,
            )
            .unwrap();
        assert_eq!(record.scope, scope);
        assert_eq!(record.scope_id, scope_id);
        assert_eq!(record.revoking_principal, "authority.emergency");
        let error = authority
            .claim_at(
                &issued.authorization_id,
                "runtime.mailer",
                &executed(),
                NOW + 100,
            )
            .unwrap_err();
        assert_eq!(error.code(), "AUTHORIZATION_REVOKED");
    }
}

#[test]
fn revocation_requires_emergency_authority_and_is_immutable() {
    let authority = Authority::in_memory(config()).unwrap();
    let issued = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-revoke-auth"), NOW)
        .unwrap()
        .authorization
        .unwrap();
    let requester = authenticated_identity(
        "agent.requester",
        PartyType::Machine,
        vec![LocalRole::Requester],
        vec![],
    );
    let unauthorized = authority
        .revoke_authenticated_at(
            &requester,
            RevocationScope::Authorization,
            &issued.authorization_id,
            RevocationReason::AuthorizationWithdrawn,
            NOW + 1,
        )
        .unwrap_err();
    assert_eq!(unauthorized.code(), "AUTHENTICATION_FAILED");
    assert_eq!(
        authority.state(&issued.authorization_id).unwrap(),
        Some(AuthzState::AuthorizedUnclaimed)
    );

    let revoker = authenticated_identity(
        "authority.emergency",
        PartyType::Machine,
        vec![LocalRole::EmergencyCanceller],
        vec![],
    );
    authority
        .revoke_authenticated_at(
            &revoker,
            RevocationScope::Authorization,
            &issued.authorization_id,
            RevocationReason::AuthorizationWithdrawn,
            NOW + 2,
        )
        .unwrap();
    let repeated = authority
        .revoke_authenticated_at(
            &revoker,
            RevocationScope::Authorization,
            &issued.authorization_id,
            RevocationReason::EmergencyDeny,
            NOW + 3,
        )
        .unwrap_err();
    assert_eq!(repeated.code(), "REVOCATION_ALREADY_ACTIVE");
}

#[test]
fn scoped_revocation_survives_restart() {
    let path = temp_db("revocation-restart");
    let issued = {
        let authority = Authority::open(&path, config()).unwrap();
        let issued = authority
            .evaluate_and_issue_at("agent.requester", &intent("req-revoke-restart"), NOW)
            .unwrap()
            .authorization
            .unwrap();
        let revoker = authenticated_identity(
            "authority.emergency",
            PartyType::Machine,
            vec![LocalRole::EmergencyCanceller],
            vec![],
        );
        authority
            .revoke_authenticated_at(
                &revoker,
                RevocationScope::Capability,
                &issued.capability,
                RevocationReason::CapabilityDisabled,
                NOW + 1,
            )
            .unwrap();
        issued
    };
    let reopened = Authority::open(&path, config()).unwrap();
    let error = reopened
        .claim_at(
            &issued.authorization_id,
            "runtime.mailer",
            &executed(),
            NOW + 2,
        )
        .unwrap_err();
    assert_eq!(error.code(), "AUTHORIZATION_REVOKED");
    drop(reopened);
    clean_db(&path);
}

#[test]
fn authorization_revocation_and_claim_have_one_transactional_winner() {
    let path = temp_db("revoke-claim-race");
    let issuer = Authority::open(&path, config()).unwrap();
    let issued = issuer
        .evaluate_and_issue_at("agent.requester", &intent("req-revoke-race"), NOW)
        .unwrap()
        .authorization
        .unwrap();
    drop(issuer);

    let claim_authority = Authority::open(&path, config()).unwrap();
    let revoke_authority = Authority::open(&path, config()).unwrap();
    let revoker = authenticated_identity(
        "authority.emergency",
        PartyType::Machine,
        vec![LocalRole::EmergencyCanceller],
        vec![],
    );
    let barrier = Arc::new(Barrier::new(3));
    let claim_barrier = Arc::clone(&barrier);
    let claim_authorization_id = issued.authorization_id.clone();
    let claim_handle = thread::spawn(move || {
        claim_barrier.wait();
        claim_authority.claim_at(
            &claim_authorization_id,
            "runtime.mailer",
            &executed(),
            NOW + 1,
        )
    });
    let revoke_barrier = Arc::clone(&barrier);
    let revoke_authorization_id = issued.authorization_id.clone();
    let revoke_handle = thread::spawn(move || {
        revoke_barrier.wait();
        revoke_authority.revoke_authenticated_at(
            &revoker,
            RevocationScope::Authorization,
            &revoke_authorization_id,
            RevocationReason::EmergencyDeny,
            NOW + 1,
        )
    });
    barrier.wait();
    let claim = claim_handle.join().unwrap();
    let revocation = revoke_handle.join().unwrap();
    assert_ne!(claim.is_ok(), revocation.is_ok());
    match (claim, revocation) {
        (Ok(_), Err(error)) => assert_eq!(error.code(), "AUTHORIZATION_TERMINAL"),
        (Err(error), Ok(_)) => assert_eq!(error.code(), "AUTHORIZATION_REVOKED"),
        _ => unreachable!(),
    }
    clean_db(&path);
}

fn execution_result(summary: &str) -> ExecutionResultEvidence {
    ExecutionResultEvidence {
        result_summary: Some(summary.into()),
        result_hash: None,
        external_evidence_reference: None,
    }
}

fn executor_identity() -> AuthenticatedIdentity {
    authenticated_identity(
        "runtime.mailer",
        PartyType::Machine,
        vec![LocalRole::Executor],
        vec![],
    )
}

fn local_adapter_session(
    adapter_principal: &str,
    adapter_role: LocalRole,
    authority_principal: &str,
) -> tlpx::Result<AuthenticatedAdapterSession> {
    let authority_authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: adapter_principal.into(),
        party_type: PartyType::Machine,
        roles: vec![adapter_role],
        approval_routes: vec![],
    }])?;
    let adapter_authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: authority_principal.into(),
        party_type: PartyType::Machine,
        roles: vec![LocalRole::Authority],
        approval_routes: vec![],
    }])?;
    let (authority_side, adapter_side) = UnixStream::pair()
        .map_err(|error| tlpx::Error::authority(format!("local socket pair: {error}")))?;
    AuthenticatedAdapterSession::authenticate_local(
        &authority_authenticator,
        &adapter_authenticator,
        &authority_side,
        &adapter_side,
    )
}

fn adapter_identity() -> AuthenticatedAdapterSession {
    local_adapter_session(
        "adapter.mailer.local",
        LocalRole::Adapter,
        "authority.local",
    )
    .unwrap()
}

fn reconciler_identity() -> AuthenticatedIdentity {
    authenticated_identity(
        "authority.reconciler",
        PartyType::Machine,
        vec![LocalRole::Reconciler],
        vec![],
    )
}

#[test]
fn adapter_contract_rejects_incomplete_or_reordered_material_mapping() {
    for fields in [
        ADAPTER_MATERIAL_FIELDS[..8]
            .iter()
            .map(|field| (*field).to_string())
            .collect::<Vec<_>>(),
        {
            let mut fields = ADAPTER_MATERIAL_FIELDS
                .iter()
                .map(|field| (*field).to_string())
                .collect::<Vec<_>>();
            fields.swap(0, 1);
            fields
        },
        {
            let mut fields = ADAPTER_MATERIAL_FIELDS
                .iter()
                .map(|field| (*field).to_string())
                .collect::<Vec<_>>();
            fields.push("caller_downgrade".into());
            fields
        },
    ] {
        let error = AdapterRegistry::new(vec![AdapterContract {
            adapter_id: "adapter.mailer".into(),
            adapter_version: "1.0.0".into(),
            authenticated_principal: "adapter.mailer.local".into(),
            authenticated_authority: "authority.local".into(),
            binary_hash: hash('8'),
            capabilities: vec!["mailer.send".into()],
            actions: vec!["send_email".into(), "deploy".into()],
            material_fields: fields,
        }])
        .unwrap_err();
        assert_eq!(error.code(), "ADAPTER_MAPPING_INCOMPLETE");
    }
}

#[test]
fn authority_activation_requires_complete_adapter_coverage() {
    let mut broken = config();
    broken.adapters = AdapterRegistry::new(vec![AdapterContract {
        adapter_id: "adapter.mailer".into(),
        adapter_version: "1.0.0".into(),
        authenticated_principal: "adapter.mailer.local".into(),
        authenticated_authority: "authority.local".into(),
        binary_hash: hash('8'),
        capabilities: vec!["mailer.send".into()],
        actions: vec!["deploy".into()],
        material_fields: ADAPTER_MATERIAL_FIELDS
            .iter()
            .map(|field| (*field).to_string())
            .collect(),
    }])
    .unwrap();
    let error = match Authority::in_memory(broken) {
        Ok(_) => panic!("incomplete adapter coverage must fail activation"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "POLICY_COMPILE_FAILED");
}

#[test]
fn execution_start_authenticates_adapter_integrity_and_exact_mapping() {
    let authority = Authority::in_memory(config()).unwrap();
    let issued = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-adapter-auth"), NOW)
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
    let executor = executor_identity();
    let wrong_role = local_adapter_session(
        "adapter.mailer.local",
        LocalRole::Executor,
        "authority.local",
    )
    .unwrap_err();
    assert_eq!(wrong_role.code(), "AUTHENTICATION_FAILED");

    let wrong_principal = local_adapter_session(
        "adapter.impostor.local",
        LocalRole::Adapter,
        "authority.local",
    )
    .unwrap();
    let error = authority
        .begin_execution_authenticated_at(
            &claim.claim_id,
            &issued.idempotency_key,
            &executed(),
            &executor,
            &wrong_principal,
            &hash('8'),
            NOW + 2,
        )
        .unwrap_err();
    assert_eq!(error.code(), "ADAPTER_AUTHENTICATION_FAILED");

    let wrong_authority = local_adapter_session(
        "adapter.mailer.local",
        LocalRole::Adapter,
        "authority.impostor.local",
    )
    .unwrap();
    let error = authority
        .begin_execution_authenticated_at(
            &claim.claim_id,
            &issued.idempotency_key,
            &executed(),
            &executor,
            &wrong_authority,
            &hash('8'),
            NOW + 2,
        )
        .unwrap_err();
    assert_eq!(error.code(), "ADAPTER_AUTHENTICATION_FAILED");

    let error = authority
        .begin_execution_authenticated_at(
            &claim.claim_id,
            &issued.idempotency_key,
            &executed(),
            &executor,
            &adapter_identity(),
            &hash('9'),
            NOW + 2,
        )
        .unwrap_err();
    assert_eq!(error.code(), "ADAPTER_INTEGRITY_INVALID");

    let mut mutated = executed();
    mutated.target = "customer:other".into();
    let error = authority
        .begin_execution_authenticated_at(
            &claim.claim_id,
            &issued.idempotency_key,
            &mutated,
            &executor,
            &adapter_identity(),
            &hash('8'),
            NOW + 2,
        )
        .unwrap_err();
    assert_eq!(error.code(), "ACTION_MISMATCH");

    let lease = authority
        .begin_execution_authenticated_at(
            &claim.claim_id,
            &issued.idempotency_key,
            &executed(),
            &executor,
            &adapter_identity(),
            &hash('8'),
            NOW + 2,
        )
        .unwrap();
    assert_eq!(lease.state, ExecutionState::Started);
}

#[test]
fn execution_start_rejects_unactivated_adapter_version() {
    let mut versioned = config();
    versioned.adapters = AdapterRegistry::new(vec![AdapterContract {
        adapter_id: "adapter.mailer".into(),
        adapter_version: "2.0.0".into(),
        authenticated_principal: "adapter.mailer.local".into(),
        authenticated_authority: "authority.local".into(),
        binary_hash: hash('8'),
        capabilities: vec!["mailer.send".into()],
        actions: vec!["send_email".into(), "deploy".into()],
        material_fields: ADAPTER_MATERIAL_FIELDS
            .iter()
            .map(|field| (*field).to_string())
            .collect(),
    }])
    .unwrap();
    let authority = Authority::in_memory(versioned).unwrap();
    let issued = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-adapter-version"), NOW)
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
    let error = authority
        .begin_execution_authenticated_at(
            &claim.claim_id,
            &issued.idempotency_key,
            &executed(),
            &executor_identity(),
            &adapter_identity(),
            &hash('8'),
            NOW + 2,
        )
        .unwrap_err();
    assert_eq!(error.code(), "ADAPTER_VERSION_MISMATCH");
}

#[test]
fn execution_start_and_terminal_receipt_are_exactly_idempotent() {
    let authority = Authority::in_memory(config()).unwrap();
    let issued = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-execution"), NOW)
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
    let executor = executor_identity();
    let adapter = adapter_identity();
    let lease = authority
        .begin_execution_authenticated_at(
            &claim.claim_id,
            &issued.idempotency_key,
            &executed(),
            &executor,
            &adapter,
            &hash('8'),
            NOW + 2,
        )
        .unwrap();
    assert_eq!(lease.state, ExecutionState::Started);
    let retry = authority
        .begin_execution_authenticated_at(
            &claim.claim_id,
            &issued.idempotency_key,
            &executed(),
            &executor,
            &adapter,
            &hash('8'),
            NOW + 3,
        )
        .unwrap();
    assert_eq!(retry, lease);
    let conflict = authority
        .begin_execution_authenticated_at(
            &claim.claim_id,
            "different-key",
            &executed(),
            &executor,
            &adapter,
            &hash('8'),
            NOW + 3,
        )
        .unwrap_err();
    assert_eq!(conflict.code(), "IDEMPOTENCY_CONFLICT");

    let result = execution_result("protected system confirmed completion");
    let receipt = authority
        .finish_execution_authenticated_at(
            &lease.execution_id,
            &executor,
            ExecutionState::Completed,
            result.clone(),
            None,
            NOW + 4,
        )
        .unwrap();
    assert_eq!(receipt.state, ExecutionState::Completed);
    assert_eq!(
        receipt.adapter_principal.as_deref(),
        Some("adapter.mailer.local")
    );
    assert_eq!(receipt.adapter_binary_hash, Some(hash('8')));
    let terminal_retry = authority
        .finish_execution_authenticated_at(
            &lease.execution_id,
            &executor,
            ExecutionState::Completed,
            result,
            None,
            NOW + 4,
        )
        .unwrap();
    assert_eq!(terminal_retry, receipt);
    let changed = authority
        .finish_execution_authenticated_at(
            &lease.execution_id,
            &executor,
            ExecutionState::Failed,
            execution_result("changed retry"),
            None,
            NOW + 4,
        )
        .unwrap_err();
    assert_eq!(changed.code(), "IDEMPOTENCY_CONFLICT");
    let resumed = authority
        .begin_execution_authenticated_at(
            &claim.claim_id,
            &issued.idempotency_key,
            &executed(),
            &executor,
            &adapter,
            &hash('8'),
            NOW + 5,
        )
        .unwrap();
    assert_eq!(resumed.state, ExecutionState::Completed);
    assert!(authority
        .pending_evidence(100)
        .unwrap()
        .iter()
        .any(|row| row.record_type == "tlpx.execution"));
}

#[test]
fn execution_result_identity_and_cancellation_fail_closed() {
    let authority = Authority::in_memory(config()).unwrap();
    let issued = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-exec-invalid"), NOW)
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
    let executor = executor_identity();
    let lease = authority
        .begin_execution_authenticated_at(
            &claim.claim_id,
            &issued.idempotency_key,
            &executed(),
            &executor,
            &adapter_identity(),
            &hash('8'),
            NOW + 2,
        )
        .unwrap();
    let empty = authority
        .finish_execution_authenticated_at(
            &lease.execution_id,
            &executor,
            ExecutionState::Completed,
            ExecutionResultEvidence {
                result_summary: None,
                result_hash: None,
                external_evidence_reference: None,
            },
            None,
            NOW + 3,
        )
        .unwrap_err();
    assert_eq!(empty.code(), "EXECUTION_RESULT_INVALID");
    let wrong_cancel = authority
        .finish_execution_authenticated_at(
            &lease.execution_id,
            &executor,
            ExecutionState::Cancelled,
            execution_result("cancelled"),
            None,
            NOW + 3,
        )
        .unwrap_err();
    assert_eq!(wrong_cancel.code(), "EXECUTION_RESULT_INVALID");
    let other = authenticated_identity(
        "runtime.other",
        PartyType::Machine,
        vec![LocalRole::Executor],
        vec![],
    );
    let wrong_executor = authority
        .finish_execution_authenticated_at(
            &lease.execution_id,
            &other,
            ExecutionState::Failed,
            execution_result("failed"),
            None,
            NOW + 3,
        )
        .unwrap_err();
    assert_eq!(wrong_executor.code(), "EXECUTOR_MISMATCH");
    let cancelled = authority
        .finish_execution_authenticated_at(
            &lease.execution_id,
            &executor,
            ExecutionState::Cancelled,
            execution_result("stopped before external acceptance"),
            Some(CancellationOutcome::CancelledBeforeSideEffect),
            NOW + 3,
        )
        .unwrap();
    assert_eq!(cancelled.state, ExecutionState::Cancelled);
}

#[test]
fn unknown_outcome_requires_reconciliation_and_never_reopens() {
    let path = temp_db("execution-reconciliation");
    let (issued, claim, lease) = {
        let authority = Authority::open(&path, config()).unwrap();
        let issued = authority
            .evaluate_and_issue_at("agent.requester", &intent("req-unknown"), NOW)
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
        let lease = authority
            .begin_execution_authenticated_at(
                &claim.claim_id,
                &issued.idempotency_key,
                &executed(),
                &executor_identity(),
                &adapter_identity(),
                &hash('8'),
                NOW + 2,
            )
            .unwrap();
        authority
            .mark_execution_outcome_unknown_authenticated_at(
                &lease.execution_id,
                &executor_identity(),
                NOW + 3,
            )
            .unwrap();
        (issued, claim, lease)
    };
    let authority = Authority::open(&path, config()).unwrap();
    assert_eq!(
        authority.execution_state(&lease.execution_id).unwrap(),
        Some(ExecutionState::ExecutionOutcomeUnknown)
    );
    let direct_finish = authority
        .finish_execution_authenticated_at(
            &lease.execution_id,
            &executor_identity(),
            ExecutionState::Completed,
            execution_result("must not bypass reconciliation"),
            None,
            NOW + 4,
        )
        .unwrap_err();
    assert_eq!(direct_finish.code(), "RECONCILIATION_REQUIRED");
    let reconciler = reconciler_identity();
    authority
        .require_reconciliation_authenticated_at(&lease.execution_id, &reconciler, NOW + 4)
        .unwrap();
    let result = ExecutionResultEvidence {
        result_summary: Some("external status could not be established".into()),
        result_hash: None,
        external_evidence_reference: Some("incident:unknown-1".into()),
    };
    let receipt = authority
        .reconcile_execution_authenticated_at(
            &lease.execution_id,
            &reconciler,
            ExecutionState::OutcomeUnknownFinal,
            result.clone(),
            NOW + 5,
        )
        .unwrap();
    assert_eq!(receipt.state, ExecutionState::OutcomeUnknownFinal);
    let retry = authority
        .reconcile_execution_authenticated_at(
            &lease.execution_id,
            &reconciler,
            ExecutionState::OutcomeUnknownFinal,
            result,
            NOW + 5,
        )
        .unwrap();
    assert_eq!(retry, receipt);
    let begin_retry = authority
        .begin_execution_authenticated_at(
            &claim.claim_id,
            &issued.idempotency_key,
            &executed(),
            &executor_identity(),
            &adapter_identity(),
            &hash('8'),
            NOW + 6,
        )
        .unwrap();
    assert_eq!(begin_retry.state, ExecutionState::OutcomeUnknownFinal);
    drop(authority);
    clean_db(&path);
}

#[test]
fn expired_claim_is_terminal_if_no_execution_started_but_unknown_if_started() {
    let authority = Authority::in_memory(config()).unwrap();
    let unstarted = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-unstarted-expiry"), NOW)
        .unwrap()
        .authorization
        .unwrap();
    let unstarted_claim = authority
        .claim_at(
            &unstarted.authorization_id,
            "runtime.mailer",
            &executed(),
            NOW + 1,
        )
        .unwrap();
    let recovered = authority
        .recover_expired_claim_at(
            &unstarted_claim.claim_id,
            unstarted_claim.lease_expires_at_ms,
        )
        .unwrap();
    assert_eq!(recovered, ExecutionState::LeaseExpired);

    let started = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-started-expiry"), NOW)
        .unwrap()
        .authorization
        .unwrap();
    let started_claim = authority
        .claim_at(
            &started.authorization_id,
            "runtime.mailer",
            &executed(),
            NOW + 1,
        )
        .unwrap();
    let lease = authority
        .begin_execution_authenticated_at(
            &started_claim.claim_id,
            &started.idempotency_key,
            &executed(),
            &executor_identity(),
            &adapter_identity(),
            &hash('8'),
            NOW + 2,
        )
        .unwrap();
    let recovered = authority
        .recover_expired_claim_at(&started_claim.claim_id, started_claim.lease_expires_at_ms)
        .unwrap();
    assert_eq!(recovered, ExecutionState::ExecutionOutcomeUnknown);
    assert_eq!(
        authority.execution_state(&lease.execution_id).unwrap(),
        Some(ExecutionState::ExecutionOutcomeUnknown)
    );
}

#[test]
fn separate_connections_have_one_terminal_execution_winner() {
    let path = temp_db("execution-terminal-race");
    let (execution_id, ended_at_ms) = {
        let authority = Authority::open(&path, config()).unwrap();
        let issued = authority
            .evaluate_and_issue_at("agent.requester", &intent("req-terminal-race"), NOW)
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
        let lease = authority
            .begin_execution_authenticated_at(
                &claim.claim_id,
                &issued.idempotency_key,
                &executed(),
                &executor_identity(),
                &adapter_identity(),
                &hash('8'),
                NOW + 2,
            )
            .unwrap();
        (lease.execution_id, NOW + 3)
    };
    let authorities = [
        Authority::open(&path, config()).unwrap(),
        Authority::open(&path, config()).unwrap(),
    ];
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for (state, authority) in [ExecutionState::Completed, ExecutionState::Failed]
        .into_iter()
        .zip(authorities)
    {
        let barrier = Arc::clone(&barrier);
        let execution_id = execution_id.clone();
        handles.push(thread::spawn(move || {
            let executor = executor_identity();
            barrier.wait();
            authority.finish_execution_authenticated_at(
                &execution_id,
                &executor,
                state,
                execution_result(if state == ExecutionState::Completed {
                    "completed"
                } else {
                    "failed"
                }),
                None,
                ended_at_ms,
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
