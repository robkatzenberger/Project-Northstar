//! Runnable authority-only MVP. It deliberately performs no external side effect.

use tlpx::{
    Adapter, Authority, AuthorityConfig, AuthorizationTemplate, CapabilityRegistry, PolicyBundle,
    PolicyEffect, PolicyRule, Principal, Risk, SubmittedIntent, Switchboard, Value,
};

fn main() -> tlpx::Result<()> {
    let mut args = std::env::args().skip(1);
    let database = args.next().ok_or_else(|| {
        tlpx::Error::authority("usage: cargo run --example local_authority -- DATABASE REQUEST_ID")
    })?;
    let request_id = args.next().ok_or_else(|| {
        tlpx::Error::authority("usage: cargo run --example local_authority -- DATABASE REQUEST_ID")
    })?;
    if args.next().is_some() {
        return Err(tlpx::Error::authority("unexpected extra argument"));
    }

    let authority = Authority::open(database, pilot_config()?)?;
    let intent = pilot_intent(request_id);
    let outcome = authority.evaluate_and_issue("agent.requester", &intent)?;
    println!(
        "decision={} receipt={}",
        outcome.decision.as_str(),
        outcome.receipt_id
    );
    let issued = outcome
        .authorization
        .ok_or_else(|| tlpx::Error::authority("pilot request was not authorized"))?;
    println!(
        "authorization={} binding={} expires_ms={}",
        issued.authorization_id, issued.action_binding_hash, issued.claim_expires_at_ms
    );

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
    println!(
        "claim={} state=CLAIMED executed_action_hash={}",
        claim.claim_id, claim.executed_action_hash
    );
    Ok(())
}

fn pilot_config() -> tlpx::Result<AuthorityConfig> {
    Ok(AuthorityConfig {
        policy: PolicyBundle {
            id: "pilot-policy".into(),
            version: "1.0.0".into(),
            hash: format!("sha256:{}", "a".repeat(64)),
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
                        risk_source: "policy:pilot-policy@1.0.0".into(),
                    },
                ),
            }],
            default: PolicyEffect::deny("POLICY_DENY"),
        },
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
        ])?,
        capabilities: CapabilityRegistry::new(vec![(
            "mailer.send".into(),
            vec!["adapter.mailer".into()],
        )])?,
        claim_window_ms: 5_000,
        execution_lease_ms: 30_000,
    })
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
