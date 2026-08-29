use std::os::unix::net::UnixStream;
use tlpx::{
    exact_match_policy_content_hash, Adapter, AdapterContract, AdapterRegistry, Authority,
    AuthorityConfig, AuthorizationTemplate, AuthzState, CapabilityRegistry, ConfiguredPolicyBundle,
    Decision, EvidenceConfig, ExecutedAction, KeyRing, LocalAuthenticator, LocalPrincipalMapping,
    LocalRole, PartyType, PolicyBundle, PolicyBundleManifest, PolicyCatalog, PolicyEffect,
    PolicyIssuer, PolicyIssuerType, PolicyRule, Principal, Risk, SubmittedIntent, Switchboard,
    Value, ADAPTER_MATERIAL_FIELDS, EXACT_MATCH_POLICY_CONTENT_TYPE, POLICY_PRECEDENCE,
};

const NOW: i64 = 1_900_000_000_000;

fn identity(principal_id: &str, roles: Vec<LocalRole>) -> tlpx::AuthenticatedIdentity {
    let authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: principal_id.into(),
        party_type: PartyType::Machine,
        roles,
        approval_routes: vec![],
    }])
    .unwrap();
    let (server, client) = UnixStream::pair().unwrap();
    let identity = authenticator.authenticate_stream(&server).unwrap();
    drop(client);
    identity
}

fn config() -> AuthorityConfig {
    let policy = PolicyBundle {
        rules: vec![PolicyRule {
            id: "allow-agent-task".into(),
            action: "agent_task".into(),
            effect: PolicyEffect::allow(
                "POLICY_ALLOW",
                AuthorizationTemplate {
                    derived_risk: Risk::Medium,
                    capability: "agent.execute".into(),
                    resource_scope: vec!["case:123".into()],
                    risk_reasons: vec!["multi_agent_action".into()],
                    risk_source: "policy:handoff-test".into(),
                },
            ),
        }],
        default: PolicyEffect::deny("POLICY_DENY"),
    };
    let content_hash = exact_match_policy_content_hash(&policy).unwrap();
    let bundle = ConfiguredPolicyBundle {
        manifest: PolicyBundleManifest {
            policy_bundle_id: "policy.handoff".into(),
            policy_bundle_version: "1.0.0".into(),
            issuer: PolicyIssuer {
                id: "security.platform".into(),
                kind: PolicyIssuerType::Human,
            },
            content_type: EXACT_MATCH_POLICY_CONTENT_TYPE.into(),
            content_hash,
            activated_at: "2026-08-27T00:00:00.000Z".into(),
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
    };

    AuthorityConfig {
        policy: PolicyCatalog::new(vec![bundle]),
        switchboard: Switchboard::new(
            ["agent.a", "agent.b", "agent.c"]
                .into_iter()
                .map(|id| Principal {
                    id: id.into(),
                    active: true,
                    allowed_actions: vec!["agent_task".into()],
                })
                .collect(),
        )
        .unwrap(),
        capabilities: CapabilityRegistry::new(vec![(
            "agent.execute".into(),
            vec!["adapter.agent".into()],
        )])
        .unwrap(),
        adapters: AdapterRegistry::new(vec![AdapterContract {
            adapter_id: "adapter.agent".into(),
            adapter_version: "1.0.0".into(),
            authenticated_principal: "adapter.agent.local".into(),
            authenticated_authority: "authority.local".into(),
            binary_hash: format!("sha256:{}", "8".repeat(64)),
            capabilities: vec!["agent.execute".into()],
            actions: vec!["agent_task".into()],
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
            max_export_bytes: tlpx::MAX_AUDIT_SINK_BYTES,
            keys: KeyRing::active_local_authority_profile([
                ("audit-handoff-v1".into(), vec![0x51; 32]),
                ("authorization-handoff-v1".into(), vec![0x52; 32]),
            ])
            .unwrap(),
        },
        approval_window_ms: 600_000,
        claim_window_ms: 5_000,
        execution_lease_ms: 30_000,
    }
}

fn intent(requester: &str, executor: &str, request_id: &str) -> SubmittedIntent {
    SubmittedIntent {
        requesting_principal: requester.into(),
        executing_principal: executor.into(),
        action: "agent_task".into(),
        intent_class: "multi_agent_action".into(),
        target: "case:123".into(),
        arguments: Value::Object(vec![(
            "operation".into(),
            Value::String("summarize".into()),
        )]),
        environment: "production".into(),
        tenant: "tenant_abc".into(),
        declared_risk: Risk::Medium,
        data_classes: vec!["INTERNAL".into()],
        requested_capability: "agent.execute".into(),
        resource_scope: vec!["case:123".into()],
        payload_hash: None,
        artifact_hash: None,
        adapter: Adapter {
            id: "adapter.agent".into(),
            version: "1.0.0".into(),
        },
        request_id: request_id.into(),
        retry_of_receipt_id: None,
    }
}

fn executed(executor: &str) -> ExecutedAction {
    let source = intent("unused", executor, "unused");
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
fn exact_handoff_names_one_executor_and_is_consumed_once() {
    let authority = Authority::in_memory(config()).unwrap();
    let agent_a = identity("agent.a", vec![LocalRole::Requester, LocalRole::Executor]);
    let agent_b = identity("agent.b", vec![LocalRole::Executor]);
    let agent_c = identity("agent.c", vec![LocalRole::Executor]);
    let handoff = intent("agent.a", "agent.b", "handoff-once");

    let outcome = authority
        .evaluate_handoff_authenticated_at(&agent_a, &agent_b, &handoff, NOW)
        .unwrap();
    assert_eq!(outcome.decision, Decision::Allow);
    let issued = outcome.authorization.unwrap();
    assert_eq!(issued.requesting_principal, "agent.a");
    assert_eq!(issued.executing_principal, "agent.b");

    let wrong_requester = authority
        .claim_authenticated_at(
            &issued.authorization_id,
            &agent_a,
            &executed("agent.a"),
            NOW + 1,
        )
        .unwrap_err();
    assert_eq!(wrong_requester.code(), "EXECUTOR_MISMATCH");

    let forwarded = authority
        .claim_authenticated_at(
            &issued.authorization_id,
            &agent_c,
            &executed("agent.c"),
            NOW + 2,
        )
        .unwrap_err();
    assert_eq!(forwarded.code(), "EXECUTOR_MISMATCH");

    let mut mutated = executed("agent.b");
    mutated.arguments = Value::Object(vec![("operation".into(), Value::String("delete".into()))]);
    let mutation = authority
        .claim_authenticated_at(&issued.authorization_id, &agent_b, &mutated, NOW + 3)
        .unwrap_err();
    assert_eq!(mutation.code(), "ACTION_MISMATCH");

    let claim = authority
        .claim_authenticated_at(
            &issued.authorization_id,
            &agent_b,
            &executed("agent.b"),
            NOW + 4,
        )
        .unwrap();
    assert_eq!(claim.executing_principal, "agent.b");

    let replay = authority
        .claim_authenticated_at(
            &issued.authorization_id,
            &agent_b,
            &executed("agent.b"),
            NOW + 5,
        )
        .unwrap_err();
    assert_eq!(replay.code(), "ALREADY_CLAIMED");
}

#[test]
fn handoff_preflight_rejects_identity_substitution_and_retry_links() {
    let authority = Authority::in_memory(config()).unwrap();
    let agent_a = identity("agent.a", vec![LocalRole::Requester, LocalRole::Executor]);
    let agent_b = identity("agent.b", vec![LocalRole::Executor]);
    let agent_b_wrong_role = identity("agent.b", vec![LocalRole::Requester]);
    let agent_c = identity("agent.c", vec![LocalRole::Executor]);
    let handoff = intent("agent.a", "agent.b", "handoff-preflight");

    let substituted_executor = authority
        .evaluate_handoff_authenticated_at(&agent_a, &agent_c, &handoff, NOW)
        .unwrap_err();
    assert_eq!(substituted_executor.code(), "HANDOFF_EXECUTOR_MISMATCH");

    let mut substituted_requester = handoff.clone();
    substituted_requester.requesting_principal = "agent.b".into();
    let error = authority
        .evaluate_handoff_authenticated_at(&agent_a, &agent_b, &substituted_requester, NOW)
        .unwrap_err();
    assert_eq!(error.code(), "HANDOFF_REQUESTER_MISMATCH");

    let same_agent = intent("agent.a", "agent.a", "handoff-self");
    let error = authority
        .evaluate_handoff_authenticated_at(&agent_a, &agent_a, &same_agent, NOW)
        .unwrap_err();
    assert_eq!(error.code(), "HANDOFF_REQUIRES_DISTINCT_PRINCIPALS");

    let error = authority
        .evaluate_handoff_authenticated_at(&agent_a, &agent_b_wrong_role, &handoff, NOW)
        .unwrap_err();
    assert_eq!(error.code(), "AUTHENTICATION_FAILED");

    let mut linked_retry = handoff;
    linked_retry.retry_of_receipt_id = Some("receipt-parent".into());
    let error = authority
        .evaluate_handoff_authenticated_at(&agent_a, &agent_b, &linked_retry, NOW)
        .unwrap_err();
    assert_eq!(error.code(), "HANDOFF_RETRY_LINK_FORBIDDEN");

    assert_eq!(authority.operational_snapshot().unwrap().evidence.total, 0);
}

#[test]
fn onward_hop_requires_a_new_authenticated_evaluation() {
    let authority = Authority::in_memory(config()).unwrap();
    let agent_a = identity("agent.a", vec![LocalRole::Requester]);
    let agent_b = identity("agent.b", vec![LocalRole::Requester, LocalRole::Executor]);
    let agent_c = identity("agent.c", vec![LocalRole::Executor]);

    let parent = authority
        .evaluate_handoff_authenticated_at(
            &agent_a,
            &agent_b,
            &intent("agent.a", "agent.b", "handoff-parent"),
            NOW,
        )
        .unwrap();
    let parent_auth = parent.authorization.unwrap();

    let forwarded = authority
        .claim_authenticated_at(
            &parent_auth.authorization_id,
            &agent_c,
            &executed("agent.c"),
            NOW + 1,
        )
        .unwrap_err();
    assert_eq!(forwarded.code(), "EXECUTOR_MISMATCH");
    assert_eq!(
        authority.state(&parent_auth.authorization_id).unwrap(),
        Some(AuthzState::AuthorizedUnclaimed)
    );

    let child = authority
        .evaluate_handoff_authenticated_at(
            &agent_b,
            &agent_c,
            &intent("agent.b", "agent.c", "handoff-child"),
            NOW + 2,
        )
        .unwrap();
    let child_auth = child.authorization.unwrap();
    assert_ne!(child.receipt_id, parent.receipt_id);
    assert_ne!(child_auth.authorization_id, parent_auth.authorization_id);
    assert_ne!(child_auth.intent_hash, parent_auth.intent_hash);
    assert_eq!(child_auth.requesting_principal, "agent.b");
    assert_eq!(child_auth.executing_principal, "agent.c");

    authority
        .claim_authenticated_at(
            &child_auth.authorization_id,
            &agent_c,
            &executed("agent.c"),
            NOW + 3,
        )
        .unwrap();
}

#[test]
fn handoff_is_only_an_evaluation_profile_not_a_permission_shortcut() {
    let authority = Authority::in_memory(config()).unwrap();
    let agent_a = identity("agent.a", vec![LocalRole::Requester]);
    let agent_b = identity("agent.b", vec![LocalRole::Executor]);
    let mut denied = intent("agent.a", "agent.b", "handoff-denied");
    denied.action = "unapproved_task".into();

    let outcome = authority
        .evaluate_handoff_authenticated_at(&agent_a, &agent_b, &denied, NOW)
        .unwrap();
    assert_eq!(outcome.decision, Decision::Deny);
    assert!(outcome.authorization.is_none());

    let approved = intent("agent.a", "agent.b", "handoff-idempotent");
    let first = authority
        .evaluate_handoff_authenticated_at(&agent_a, &agent_b, &approved, NOW + 1)
        .unwrap();
    let retry = authority
        .evaluate_handoff_authenticated_at(&agent_a, &agent_b, &approved, NOW + 2)
        .unwrap();
    assert_eq!(retry.receipt_id, first.receipt_id);
    assert_eq!(
        retry.authorization.unwrap().authorization_id,
        first.authorization.unwrap().authorization_id
    );

    let mut conflict = approved;
    conflict.target = "case:999".into();
    let error = authority
        .evaluate_handoff_authenticated_at(&agent_a, &agent_b, &conflict, NOW + 3)
        .unwrap_err();
    assert_eq!(error.code(), "IDEMPOTENCY_CONFLICT");
}
