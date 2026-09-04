#![cfg(not(feature = "deterministic-time"))]

use rusqlite::Connection;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tlpx::{
    exact_match_policy_content_hash, Adapter, AdapterContract, AdapterRegistry,
    AuthenticatedIdentity, Authority, AuthorityConfig, AuthorizationTemplate, AuthzState,
    CapabilityRegistry, ConfiguredPolicyBundle, EvidenceConfig, ExecutedAction, KeyRing,
    LocalAuthenticator, LocalPrincipalMapping, LocalRole, PartyType, PolicyBundle,
    PolicyBundleManifest, PolicyCatalog, PolicyEffect, PolicyIssuer, PolicyIssuerType, PolicyRule,
    Principal, Risk, SubmittedIntent, Switchboard, Value, ADAPTER_MATERIAL_FIELDS,
    EXACT_MATCH_POLICY_CONTENT_TYPE, POLICY_PRECEDENCE,
};

struct TempDb(PathBuf);

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        let _ = std::fs::remove_file(format!("{}-wal", self.0.display()));
        let _ = std::fs::remove_file(format!("{}-shm", self.0.display()));
    }
}

fn hash(ch: char) -> String {
    format!("sha256:{}", ch.to_string().repeat(64))
}

fn identity(principal_id: &str, role: LocalRole) -> AuthenticatedIdentity {
    let authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: principal_id.into(),
        party_type: PartyType::Machine,
        roles: vec![role],
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
            id: "allow-email".into(),
            action: "send_email".into(),
            effect: PolicyEffect::allow(
                "POLICY_ALLOW",
                AuthorizationTemplate {
                    derived_risk: Risk::High,
                    capability: "mailer.send".into(),
                    resource_scope: vec!["customer:123".into()],
                    risk_reasons: vec!["external_communication".into()],
                    risk_source: "policy:trusted-time-production@1.0.0".into(),
                },
            ),
        }],
        default: PolicyEffect::deny("POLICY_DENY"),
    };
    let content_hash = exact_match_policy_content_hash(&policy).unwrap();
    AuthorityConfig {
        policy: PolicyCatalog::new(vec![ConfiguredPolicyBundle {
            manifest: PolicyBundleManifest {
                policy_bundle_id: "trusted-time-production".into(),
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
        adapters: AdapterRegistry::new(vec![AdapterContract {
            adapter_id: "adapter.mailer".into(),
            adapter_version: "1.0.0".into(),
            authenticated_principal: "adapter.mailer.local".into(),
            authenticated_authority: "authority.local".into(),
            binary_hash: hash('8'),
            capabilities: vec!["mailer.send".into()],
            actions: vec!["send_email".into()],
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
                ("audit-test-v1".into(), vec![0x5a; 32]),
                ("authorization-test-v1".into(), vec![0x5b; 32]),
            ])
            .unwrap(),
        },
        approval_window_ms: 600_000,
        claim_window_ms: 500,
        execution_lease_ms: 30_000,
    }
}

fn intent() -> SubmittedIntent {
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
        request_id: "production-floor-reopen".into(),
        retry_of_receipt_id: None,
    }
}

fn executed() -> ExecutedAction {
    let source = intent();
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

fn wall_time_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

fn temp_db() -> TempDb {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    TempDb(std::env::temp_dir().join(format!(
        "northstar-trusted-time-production-{}-{nonce}.sqlite",
        std::process::id()
    )))
}

fn set_durable_floor(path: &Path, floor_ms: i64) {
    let connection = Connection::open(path).unwrap();
    assert_eq!(
        connection
            .execute(
                "UPDATE tlpx_trusted_time SET last_observed_ms = ?1 WHERE singleton = 1",
                [floor_ms],
            )
            .unwrap(),
        1
    );
}

#[test]
fn reopened_claim_uses_durable_floor_for_expiry() {
    let database = temp_db();
    let authority = Authority::open(&database.0, config()).unwrap();
    let issued = authority
        .evaluate_authenticated(
            &identity("agent.requester", LocalRole::Requester),
            &intent(),
        )
        .unwrap()
        .authorization
        .unwrap();
    drop(authority);

    let durable_floor_ms = issued.claim_expires_at_ms + 1;
    set_durable_floor(&database.0, durable_floor_ms);
    let before_claim_ms = wall_time_ms();
    assert!(
        before_claim_ms < issued.claim_expires_at_ms,
        "test fixture took too long to reach the pre-expiry claim"
    );
    assert!(
        durable_floor_ms - before_claim_ms <= 1_000,
        "durable floor must remain inside the accepted backward-skew bound"
    );

    let reopened = Authority::open(&database.0, config()).unwrap();
    let error = reopened
        .claim_authenticated(
            &issued.authorization_id,
            &identity("runtime.mailer", LocalRole::Executor),
            &executed(),
        )
        .unwrap_err();
    assert_eq!(error.code(), "AUTHORIZATION_EXPIRED");
    assert_eq!(
        reopened.state(&issued.authorization_id).unwrap(),
        Some(AuthzState::Expired)
    );
}
