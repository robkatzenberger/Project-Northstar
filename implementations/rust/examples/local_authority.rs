//! Runnable authority-only MVP. It deliberately performs no external side effect.

use std::os::unix::net::UnixStream;
use tlpx::{
    exact_match_policy_content_hash, Adapter, AuthenticatedIdentity, Authority, AuthorityConfig,
    AuthorizationTemplate, CancellationReason, CapabilityRegistry, ConfiguredPolicyBundle,
    EvidenceConfig, LocalAuthenticator, LocalPrincipalMapping, LocalRole, PartyType, PolicyBundle,
    PolicyBundleManifest, PolicyCatalog, PolicyEffect, PolicyIssuer, PolicyIssuerType, PolicyRule,
    Principal, Risk, SubmittedIntent, Switchboard, Value, EXACT_MATCH_POLICY_CONTENT_TYPE,
    POLICY_PRECEDENCE,
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
        for row in authority.pending_evidence(100)? {
            println!("{}", row.record_json);
        }
    } else {
        println!(
            "claim={} state=CLAIMED executed_action_hash={}",
            claim.claim_id, claim.executed_action_hash
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
        evidence: EvidenceConfig {
            evaluator_id: "authority.local".into(),
            router_id: "switchboard.local".into(),
            requester_type: PartyType::Machine,
            // Deliberately public and insecure: local demonstration/schema checks only.
            seal_key_id: "insecure-example-only".into(),
            seal_key: vec![0x42; 32],
        },
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
