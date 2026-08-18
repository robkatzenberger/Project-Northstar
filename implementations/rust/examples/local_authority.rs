//! Runnable authority-only MVP. It deliberately performs no external side effect.

use std::os::unix::net::UnixStream;
use tlpx::{
    exact_match_policy_content_hash, Adapter, AdapterContract, AdapterRegistry, ApprovalOutcome,
    ApprovalPresentation, AuthenticatedAdapterSession, AuthenticatedIdentity, Authority,
    AuthorityConfig, AuthorizationTemplate, CancellationOutcome, CancellationReason,
    CapabilityRegistry, ConfiguredPolicyBundle, EvidenceConfig, ExecutionResultEvidence,
    ExecutionState, LocalAuthenticator, LocalPrincipalMapping, LocalRole, PartyType, PolicyBundle,
    PolicyBundleManifest, PolicyCatalog, PolicyEffect, PolicyIssuer, PolicyIssuerType, PolicyRule,
    Principal, Risk, SubmittedIntent, Switchboard, Value, ADAPTER_MATERIAL_FIELDS,
    EXACT_MATCH_POLICY_CONTENT_TYPE, POLICY_PRECEDENCE,
};

fn main() -> tlpx::Result<()> {
    let mut args = std::env::args().skip(1);
    let database = args.next().ok_or_else(|| {
        tlpx::Error::authority(
            "usage: cargo run --example local_authority -- DATABASE REQUEST_ID [--evidence-jsonl]",
        )
    })?;
    let request_id = args.next().ok_or_else(|| {
        tlpx::Error::authority(
            "usage: cargo run --example local_authority -- DATABASE REQUEST_ID [--evidence-jsonl]",
        )
    })?;
    let evidence_jsonl = match args.next().as_deref() {
        None => false,
        Some("--evidence-jsonl") => true,
        Some(_) => return Err(tlpx::Error::authority("unexpected extra argument")),
    };
    if args.next().is_some() {
        return Err(tlpx::Error::authority("unexpected extra argument"));
    }

    let authority = Authority::open(database, pilot_config()?)?;
    let intent = pilot_intent(request_id);
    let outcome = authority.evaluate_and_issue("agent.requester", &intent)?;
    if !evidence_jsonl {
        println!(
            "decision={} receipt={}",
            outcome.decision.as_str(),
            outcome.receipt_id
        );
    }
    let issued = outcome
        .authorization
        .ok_or_else(|| tlpx::Error::authority("pilot request was not authorized"))?;
    if !evidence_jsonl {
        println!(
            "authorization={} binding={} expires_ms={}",
            issued.authorization_id, issued.action_binding_hash, issued.claim_expires_at_ms
        );
    }

    let executed = tlpx::ExecutedAction {
        executing_principal: intent.executing_principal,
        action: intent.action,
        target: intent.target,
        arguments: intent.arguments,
        environment: intent.environment,
        tenant: intent.tenant,
        payload_hash: intent.payload_hash,
        artifact_hash: intent.artifact_hash,
        adapter: intent.adapter,
    };
    let claim = authority.claim(&issued.authorization_id, "runtime.mailer", &executed)?;
    let executor = pilot_authenticated_executor()?;
    let adapter = pilot_authenticated_adapter()?;
    let execution = authority.begin_execution_authenticated_at(
        &claim.claim_id,
        &issued.idempotency_key,
        &executed,
        &executor,
        &adapter,
        &format!("sha256:{}", "8".repeat(64)),
        claim.claimed_at_ms + 1,
    )?;
    let terminal = authority.finish_execution_authenticated_at(
        &execution.execution_id,
        &executor,
        ExecutionState::Cancelled,
        ExecutionResultEvidence {
            result_summary: Some("schema example stopped before any protected side effect".into()),
            result_hash: None,
            external_evidence_reference: None,
        },
        Some(CancellationOutcome::CancelledBeforeSideEffect),
        claim.claimed_at_ms + 2,
    )?;
    if evidence_jsonl {
        let mut invalid = pilot_intent(format!("{}-error", issued.request_id));
        invalid.requesting_principal = "agent.untrusted-body".into();
        let error = authority
            .evaluate_and_issue("agent.requester", &invalid)
            .expect_err("authentication mismatch must fail");
        if error.code() != "AUTHENTICATION_FAILED" {
            return Err(error);
        }
        let requester = pilot_authenticated_requester()?;
        let mut pending = pilot_intent(format!("{}-pending", issued.request_id));
        pending.action = "deploy".into();
        let pending = authority.evaluate_authenticated(&requester, &pending)?;
        authority.cancel_pending_authenticated(
            &pending.receipt_id,
            &requester,
            CancellationReason::RequesterWithdrawn,
        )?;
        let operator = pilot_authenticated_operator()?;
        let mut approval = pilot_intent(format!("{}-approval", issued.request_id));
        approval.action = "deploy".into();
        let approval = authority.evaluate_authenticated(&requester, &approval)?;
        let view = authority.pending_approval_authenticated(&approval.receipt_id, &operator)?;
        authority.resolve_pending_authenticated(
            &approval.receipt_id,
            &operator,
            ApprovalOutcome::Approve,
            ApprovalPresentation {
                authorized_action_hash: view.authorized_action_hash,
                renderer_id: "approval.example".into(),
                renderer_version: "1.0.0".into(),
            },
        )?;
        for row in authority.pending_evidence(100)? {
            println!("{}", row.record_json);
        }
    } else {
        println!(
            "claim={} execution={} state={} executed_action_hash={}",
            claim.claim_id,
            terminal.execution_id,
            terminal.state.as_str(),
            claim.executed_action_hash
        );
    }
    Ok(())
}

fn pilot_config() -> tlpx::Result<AuthorityConfig> {
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
                        risk_source: "policy:pilot-policy@1.0.0".into(),
                    },
                ),
            },
            PolicyRule {
                id: "approve-deploy".into(),
                action: "deploy".into(),
                effect: PolicyEffect::require_approval(
                    "POLICY_REQUIRE_APPROVAL",
                    AuthorizationTemplate {
                        derived_risk: Risk::High,
                        capability: "mailer.send".into(),
                        resource_scope: vec!["customer:123".into()],
                        risk_reasons: vec!["deployment_change".into()],
                        risk_source: "policy:pilot-policy@1.0.0".into(),
                    },
                    vec!["ops.deploy".into()],
                ),
            },
        ],
        default: PolicyEffect::deny("POLICY_DENY"),
    };
    let content_hash = exact_match_policy_content_hash(&policy)?;
    Ok(AuthorityConfig {
        policy: PolicyCatalog::new(vec![ConfiguredPolicyBundle {
            manifest: PolicyBundleManifest {
                policy_bundle_id: "pilot-policy".into(),
                policy_bundle_version: "1.0.0".into(),
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
        }]),
        switchboard: Switchboard::new(vec![
            Principal {
                id: "agent.requester".into(),
                active: true,
                allowed_actions: vec!["send_email".into(), "deploy".into()],
            },
            Principal {
                id: "runtime.mailer".into(),
                active: true,
                allowed_actions: vec!["send_email".into(), "deploy".into()],
            },
        ])?,
        capabilities: CapabilityRegistry::new(vec![(
            "mailer.send".into(),
            vec!["adapter.mailer".into()],
        )])?,
        adapters: AdapterRegistry::new(vec![AdapterContract {
            adapter_id: "adapter.mailer".into(),
            adapter_version: "1.0.0".into(),
            authenticated_principal: "adapter.mailer.local".into(),
            authenticated_authority: "authority.local".into(),
            binary_hash: format!("sha256:{}", "8".repeat(64)),
            capabilities: vec!["mailer.send".into()],
            actions: vec!["send_email".into(), "deploy".into()],
            material_fields: ADAPTER_MATERIAL_FIELDS
                .iter()
                .map(|field| (*field).to_string())
                .collect(),
        }])?,
        evidence: EvidenceConfig {
            evaluator_id: "authority.local".into(),
            router_id: "switchboard.local".into(),
            requester_type: PartyType::Machine,
            // Deliberately public and insecure: local demonstration/schema checks only.
            seal_key_id: "insecure-example-only".into(),
            seal_key: vec![0x42; 32],
        },
        approval_window_ms: 600_000,
        claim_window_ms: 5_000,
        execution_lease_ms: 30_000,
    })
}

fn pilot_authenticated_requester() -> tlpx::Result<AuthenticatedIdentity> {
    let authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: "agent.requester".into(),
        party_type: PartyType::Machine,
        roles: vec![LocalRole::Requester],
        approval_routes: vec![],
    }])?;
    let (server, client) = UnixStream::pair()
        .map_err(|error| tlpx::Error::authority(format!("local socket pair: {error}")))?;
    let identity = authenticator.authenticate_stream(&server)?;
    drop(client);
    Ok(identity)
}

fn pilot_authenticated_operator() -> tlpx::Result<AuthenticatedIdentity> {
    let authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: "operator.local".into(),
        party_type: PartyType::Human,
        roles: vec![LocalRole::Operator],
        approval_routes: vec!["ops.deploy".into()],
    }])?;
    let (server, client) = UnixStream::pair()
        .map_err(|error| tlpx::Error::authority(format!("local socket pair: {error}")))?;
    let identity = authenticator.authenticate_stream(&server)?;
    drop(client);
    Ok(identity)
}

fn pilot_authenticated_executor() -> tlpx::Result<AuthenticatedIdentity> {
    let authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: "runtime.mailer".into(),
        party_type: PartyType::Machine,
        roles: vec![LocalRole::Executor],
        approval_routes: vec![],
    }])?;
    let (server, client) = UnixStream::pair()
        .map_err(|error| tlpx::Error::authority(format!("local socket pair: {error}")))?;
    let identity = authenticator.authenticate_stream(&server)?;
    drop(client);
    Ok(identity)
}

fn pilot_authenticated_adapter() -> tlpx::Result<AuthenticatedAdapterSession> {
    let authority_authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: "adapter.mailer.local".into(),
        party_type: PartyType::Machine,
        roles: vec![LocalRole::Adapter],
        approval_routes: vec![],
    }])?;
    let adapter_authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: "authority.local".into(),
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

fn pilot_intent(request_id: String) -> SubmittedIntent {
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
        request_id,
        retry_of_receipt_id: None,
    }
}
