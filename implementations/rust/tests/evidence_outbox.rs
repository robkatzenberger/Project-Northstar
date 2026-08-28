use rusqlite::Connection;
use serde_json::Value as JsonValue;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tlpx::{
    exact_match_policy_content_hash, Adapter, AdapterContract, AdapterRegistry,
    AuthenticatedAdapterSession, AuthenticatedIdentity, Authority, AuthorityConfig,
    AuthorizationTemplate, AuthzState, CapabilityRegistry, ConfiguredPolicyBundle, EvidenceConfig,
    ExecutedAction, ExecutionResultEvidence, ExecutionState, KeyPurpose, KeyRing,
    LocalAuthenticator, LocalPrincipalMapping, LocalRole, PartyType, PolicyBundle,
    PolicyBundleManifest, PolicyCatalog, PolicyEffect, PolicyIssuer, PolicyIssuerType, PolicyRule,
    Principal, RevocationReason, RevocationScope, Risk, RoleKey, SubmittedIntent, Switchboard,
    Value, ADAPTER_MATERIAL_FIELDS, EXACT_MATCH_POLICY_CONTENT_TYPE, POLICY_PRECEDENCE,
};

const NOW: i64 = 1_800_000_000_000;

fn hash(ch: char) -> String {
    format!("sha256:{}", ch.to_string().repeat(64))
}

fn role_identity(principal_id: &str, role: LocalRole) -> AuthenticatedIdentity {
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

fn executor_identity() -> AuthenticatedIdentity {
    role_identity("runtime.mailer", LocalRole::Executor)
}

/// Test-only compatibility seam. The crate's public API accepts authenticated
/// identities; deterministic test calls cannot reach raw principal-string
/// authority methods.
trait AuthenticatedTestAuthority {
    fn evaluate_and_issue_at(
        &self,
        requester: &str,
        intent: &SubmittedIntent,
        evaluated_at_ms: i64,
    ) -> tlpx::Result<tlpx::EvaluationOutcome>;
    fn claim_at(
        &self,
        authorization_id: &str,
        executor: &str,
        executed: &ExecutedAction,
        claimed_at_ms: i64,
    ) -> tlpx::Result<tlpx::ClaimRecord>;
}

impl AuthenticatedTestAuthority for Authority {
    fn evaluate_and_issue_at(
        &self,
        requester: &str,
        intent: &SubmittedIntent,
        evaluated_at_ms: i64,
    ) -> tlpx::Result<tlpx::EvaluationOutcome> {
        self.evaluate_authenticated_at(
            &role_identity(requester, LocalRole::Requester),
            intent,
            evaluated_at_ms,
        )
    }

    fn claim_at(
        &self,
        authorization_id: &str,
        executor: &str,
        executed: &ExecutedAction,
        claimed_at_ms: i64,
    ) -> tlpx::Result<tlpx::ClaimRecord> {
        self.claim_authenticated_at(
            authorization_id,
            &role_identity(executor, LocalRole::Executor),
            executed,
            claimed_at_ms,
        )
    }
}

fn adapter_identity() -> AuthenticatedAdapterSession {
    let authority_authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: "adapter.mailer.local".into(),
        party_type: PartyType::Machine,
        roles: vec![LocalRole::Adapter],
        approval_routes: vec![],
    }])
    .unwrap();
    let adapter_authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: "authority.local".into(),
        party_type: PartyType::Machine,
        roles: vec![LocalRole::Authority],
        approval_routes: vec![],
    }])
    .unwrap();
    let (authority_side, adapter_side) = UnixStream::pair().unwrap();
    AuthenticatedAdapterSession::authenticate_local(
        &authority_authenticator,
        &adapter_authenticator,
        &authority_side,
        &adapter_side,
    )
    .unwrap()
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
                    risk_source: "policy:evidence-policy@1.0.0".into(),
                },
            ),
        }],
        default: PolicyEffect::deny("POLICY_DENY"),
    };
    let content_hash = exact_match_policy_content_hash(&policy).unwrap();
    AuthorityConfig {
        policy: PolicyCatalog::new(vec![ConfiguredPolicyBundle {
            manifest: PolicyBundleManifest {
                policy_bundle_id: "evidence-policy".into(),
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
            keys: KeyRing::active_profile([
                ("audit-test-v1".into(), vec![0x5a; 32]),
                ("authorization-test-v1".into(), vec![0x5b; 32]),
                ("service-test-v1".into(), vec![0x5c; 32]),
                ("operator-test-v1".into(), vec![0x5d; 32]),
                ("tenant-test-v1".into(), vec![0x5e; 32]),
            ])
            .unwrap(),
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

fn temp_db(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "northstar-evidence-{label}-{}-{nonce}.sqlite",
        std::process::id()
    ))
}

fn clean_db(path: &Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}

fn json(record: &str) -> JsonValue {
    serde_json::from_str(record).unwrap()
}

#[test]
fn decision_authorization_and_claim_are_canonical_sealed_and_ordered() {
    let authority = Authority::in_memory(config()).unwrap();
    let outcome = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-evidence"), NOW)
        .unwrap();
    let issued = outcome.authorization.unwrap();

    let rows = authority.pending_evidence(10).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].record_type, "tlpx.decision");
    assert_eq!(rows[0].authority_sequence, outcome.sequence);
    assert_eq!(rows[0].ordinal, 0);
    assert_eq!(rows[1].record_type, "tlpx.authorization");
    assert_eq!(rows[1].authority_sequence, outcome.sequence);
    assert_eq!(rows[1].ordinal, 1);
    assert_eq!(
        rows[1].previous_chain_hash.as_deref(),
        Some(rows[0].chain_hash.as_str())
    );
    assert!(rows
        .iter()
        .all(|row| row.record_hash.starts_with("sha256:")));
    assert!(rows.iter().all(|row| row.chain_hash.starts_with("sha256:")));
    assert!(rows.iter().all(|row| row.seal.starts_with("hmac-sha256:")));

    let decision = json(&rows[0].record_json);
    assert_eq!(decision["standard_version"], "0.2.0");
    assert_eq!(decision["decision"], "ALLOW");
    assert_eq!(decision["policy_id"], "allow-email");
    assert_eq!(decision["parties"]["requester"]["type"], "machine");
    assert_eq!(decision["parties"]["evaluator"]["id"], "authority.local");
    assert!(decision.get("seal").is_none());
    assert!(decision.get("record_hash").is_none());

    let authorization = json(&rows[1].record_json);
    assert_eq!(authorization["target"], "customer:123");
    assert_eq!(authorization["issued_at"], "2027-01-15T08:00:00.000Z");
    assert_eq!(authorization["execution_lease_seconds"], 30);
    assert_eq!(authorization["state"], "AUTHORIZED_UNCLAIMED");

    let claim = authority
        .claim_at(
            &issued.authorization_id,
            "runtime.mailer",
            &executed(),
            NOW + 1_000,
        )
        .unwrap();
    let rows = authority.pending_evidence(10).unwrap();
    assert_eq!(rows.len(), 3);
    let claim_row = &rows[2];
    assert_eq!(claim_row.record_type, "tlpx.authorization_claim");
    assert_eq!(claim_row.authority_sequence, claim.sequence);
    let claim_json = json(&claim_row.record_json);
    assert_eq!(claim_json["adapter"]["id"], "adapter.mailer");
    assert_eq!(claim_json["state"], "CLAIMED");

    let reconciliation = authority.reconcile_evidence().unwrap();
    assert_eq!(reconciliation.total, 3);
    assert_eq!(reconciliation.pending, 3);
    assert_eq!(reconciliation.exported, 0);
    assert_eq!(
        reconciliation.last_chain_hash.as_deref(),
        Some(claim_row.chain_hash.as_str())
    );
}

#[test]
fn requester_party_type_is_explicit_embedding_metadata() {
    let mut human_config = config();
    human_config.evidence.requester_type = PartyType::Human;
    let authority = Authority::in_memory(human_config).unwrap();
    authority
        .evaluate_and_issue_at("agent.requester", &intent("req-human-type"), NOW)
        .unwrap();

    let rows = authority.pending_evidence(10).unwrap();
    let decision = json(&rows[0].record_json);
    assert_eq!(decision["parties"]["requester"]["type"], "human");
}

#[test]
fn evaluation_error_is_schema_shaped_sealed_and_restart_durable() {
    let path = temp_db("error-restart");
    {
        let authority = Authority::open(&path, config()).unwrap();
        let error = authority
            .evaluate_and_issue_at("agent.impostor", &intent("req-error"), NOW)
            .unwrap_err();
        assert_eq!(error.code(), "AUTHENTICATION_FAILED");
        let rows = authority.pending_evidence(10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].record_type, "tlpx.evaluation_error");
        let record = json(&rows[0].record_json);
        assert_eq!(record["authenticated_requester"], "agent.impostor");
        assert_eq!(record["request_id"], "req-error");
        assert_eq!(record["retryability"], "AFTER_CONDITION");
        assert!(record["required_condition"].is_string());
        assert!(record.get("authorization_id").is_none());
    }
    {
        let authority = Authority::open(&path, config()).unwrap();
        let reconciliation = authority.reconcile_evidence().unwrap();
        assert_eq!(reconciliation.total, 1);
        assert_eq!(reconciliation.pending, 1);
    }
    clean_db(&path);
}

#[test]
fn export_ack_is_idempotent_and_restart_durable() {
    let path = temp_db("export-ack");
    let first_id;
    let first_hash;
    {
        let authority = Authority::open(&path, config()).unwrap();
        authority
            .evaluate_and_issue_at("agent.requester", &intent("req-export"), NOW)
            .unwrap();
        let mut rows = authority.pending_evidence(10).unwrap();
        let second = rows.pop().unwrap();
        let first = rows.pop().unwrap();
        first_id = first.outbox_id;
        first_hash = first.chain_hash;
        let order_error = authority
            .mark_evidence_exported_at(second.outbox_id, &second.chain_hash, NOW + 1_000)
            .unwrap_err();
        assert!(order_error.message().contains("preserve outbox order"));
        assert!(authority
            .mark_evidence_exported_at(first_id, &first_hash, NOW + 2_000)
            .unwrap());
        assert!(!authority
            .mark_evidence_exported_at(first_id, &first_hash, NOW + 3_000)
            .unwrap());
        assert!(authority
            .mark_evidence_exported_at(first_id, &hash('f'), NOW + 3_000)
            .is_err());
    }
    {
        let authority = Authority::open(&path, config()).unwrap();
        let pending = authority.pending_evidence(10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_ne!(pending[0].outbox_id, first_id);
        let reconciliation = authority.reconcile_evidence().unwrap();
        assert_eq!(reconciliation.total, 2);
        assert_eq!(reconciliation.exported, 1);
        assert_eq!(reconciliation.pending, 1);
    }
    clean_db(&path);
}

#[test]
fn outbox_failure_rolls_back_evaluation_slot_and_sequence() {
    let path = temp_db("evaluation-rollback");
    {
        let authority = Authority::open(&path, config()).unwrap();
        drop(authority);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TRIGGER fail_outbox BEFORE INSERT ON tlpx_evidence_outbox
                 BEGIN SELECT RAISE(FAIL, 'injected outbox failure'); END;",
            )
            .unwrap();
    }
    {
        let authority = Authority::open(&path, config()).unwrap();
        let error = authority
            .evaluate_and_issue_at("agent.requester", &intent("req-outbox-fail"), NOW)
            .unwrap_err();
        assert_eq!(error.code(), "AUTHORITY_INTERNAL_ERROR");
        assert!(error.receipt_id().is_none());
        assert_eq!(authority.evaluation_event_count().unwrap(), 0);
        assert!(authority.pending_evidence(10).unwrap().is_empty());
    }
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("DROP TRIGGER fail_outbox;")
            .unwrap();
        let authority = Authority::open(&path, config()).unwrap();
        let outcome = authority
            .evaluate_and_issue_at("agent.requester", &intent("req-outbox-fail"), NOW)
            .unwrap();
        assert_eq!(outcome.sequence, 1);
        assert_eq!(authority.pending_evidence(10).unwrap().len(), 2);
    }
    clean_db(&path);
}

#[test]
fn claim_outbox_failure_does_not_consume_authorization() {
    let path = temp_db("claim-rollback");
    let authorization_id;
    {
        let authority = Authority::open(&path, config()).unwrap();
        authorization_id = authority
            .evaluate_and_issue_at("agent.requester", &intent("req-claim-fail"), NOW)
            .unwrap()
            .authorization
            .unwrap()
            .authorization_id;
    }
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TRIGGER fail_claim_outbox
                 BEFORE INSERT ON tlpx_evidence_outbox
                 WHEN NEW.record_type = 'tlpx.authorization_claim'
                 BEGIN SELECT RAISE(FAIL, 'injected claim outbox failure'); END;",
            )
            .unwrap();
        let authority = Authority::open(&path, config()).unwrap();
        let error = authority
            .claim_at(
                &authorization_id,
                "runtime.mailer",
                &executed(),
                NOW + 1_000,
            )
            .unwrap_err();
        assert_eq!(error.code(), "AUTHORITY_INTERNAL_ERROR");
        assert_eq!(
            authority.state(&authorization_id).unwrap(),
            Some(AuthzState::AuthorizedUnclaimed)
        );
        assert_eq!(authority.pending_evidence(10).unwrap().len(), 2);
        drop(authority);
        connection
            .execute_batch("DROP TRIGGER fail_claim_outbox;")
            .unwrap();
    }
    {
        let authority = Authority::open(&path, config()).unwrap();
        authority
            .claim_at(
                &authorization_id,
                "runtime.mailer",
                &executed(),
                NOW + 1_000,
            )
            .unwrap();
        assert_eq!(authority.pending_evidence(10).unwrap().len(), 3);
        authority.reconcile_evidence().unwrap();
    }
    clean_db(&path);
}

#[test]
fn execution_outbox_failure_does_not_create_a_ghost_terminal_receipt() {
    let path = temp_db("execution-rollback");
    let (execution_id, ended_at_ms) = {
        let authority = Authority::open(&path, config()).unwrap();
        let issued = authority
            .evaluate_and_issue_at("agent.requester", &intent("req-execution-fail"), NOW)
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
            .unwrap()
            .into_started()
            .unwrap();
        (lease.execution_id, NOW + 3)
    };
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER fail_execution_outbox
             BEFORE INSERT ON tlpx_evidence_outbox
             WHEN NEW.record_type = 'tlpx.execution'
             BEGIN SELECT RAISE(FAIL, 'injected execution outbox failure'); END;",
        )
        .unwrap();
    let authority = Authority::open(&path, config()).unwrap();
    let error = authority
        .finish_execution_authenticated_at(
            &execution_id,
            &executor_identity(),
            ExecutionState::Completed,
            ExecutionResultEvidence {
                result_summary: Some("completed".into()),
                result_hash: None,
                external_evidence_reference: None,
            },
            None,
            ended_at_ms,
        )
        .unwrap_err();
    assert_eq!(error.code(), "AUTHORITY_INTERNAL_ERROR");
    assert_eq!(
        authority.execution_state(&execution_id).unwrap(),
        Some(ExecutionState::Started)
    );
    assert_eq!(authority.pending_evidence(10).unwrap().len(), 3);
    drop(authority);
    connection
        .execute_batch("DROP TRIGGER fail_execution_outbox;")
        .unwrap();
    drop(connection);
    let authority = Authority::open(&path, config()).unwrap();
    authority
        .finish_execution_authenticated_at(
            &execution_id,
            &executor_identity(),
            ExecutionState::Completed,
            ExecutionResultEvidence {
                result_summary: Some("completed".into()),
                result_hash: None,
                external_evidence_reference: None,
            },
            None,
            ended_at_ms,
        )
        .unwrap();
    assert_eq!(authority.pending_evidence(10).unwrap().len(), 4);
    authority.reconcile_evidence().unwrap();
    clean_db(&path);
}

#[test]
fn reconciliation_detects_payload_tampering() {
    let path = temp_db("tamper");
    {
        let authority = Authority::open(&path, config()).unwrap();
        authority
            .evaluate_and_issue_at("agent.requester", &intent("req-tamper"), NOW)
            .unwrap();
    }
    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE tlpx_evidence_outbox SET record_json = '{}'
             WHERE outbox_id = (SELECT MIN(outbox_id) FROM tlpx_evidence_outbox)",
            [],
        )
        .unwrap();
    drop(connection);
    let authority = Authority::open(&path, config()).unwrap();
    let evaluation_error = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-after-tamper"), NOW + 1_000)
        .unwrap_err();
    assert!(evaluation_error.message().contains("record hash mismatch"));
    assert_eq!(authority.evaluation_event_count().unwrap(), 1);
    let error = authority.reconcile_evidence().unwrap_err();
    assert!(error.message().contains("record hash mismatch"));
    clean_db(&path);
}

#[test]
fn reconciliation_detects_envelope_source_swapping() {
    let path = temp_db("source-swap");
    {
        let authority = Authority::open(&path, config()).unwrap();
        authority
            .evaluate_and_issue_at("agent.requester", &intent("req-source-a"), NOW)
            .unwrap();
        authority
            .evaluate_and_issue_at("agent.requester", &intent("req-source-b"), NOW + 1_000)
            .unwrap();
    }
    let connection = Connection::open(&path).unwrap();
    let mut statement = connection
        .prepare(
            "SELECT outbox_id, source_id FROM tlpx_evidence_outbox
             WHERE record_type = 'tlpx.decision' ORDER BY outbox_id",
        )
        .unwrap();
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    drop(statement);
    assert_eq!(rows.len(), 2);
    connection
        .execute(
            "UPDATE tlpx_evidence_outbox SET source_id = 'temporary-source-swap'
             WHERE outbox_id = ?1",
            [rows[0].0],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE tlpx_evidence_outbox SET source_id = ?1 WHERE outbox_id = ?2",
            (&rows[0].1, rows[1].0),
        )
        .unwrap();
    connection
        .execute(
            "UPDATE tlpx_evidence_outbox SET source_id = ?1 WHERE outbox_id = ?2",
            (&rows[1].1, rows[0].0),
        )
        .unwrap();
    drop(connection);

    let authority = Authority::open(&path, config()).unwrap();
    let error = authority.reconcile_evidence().unwrap_err();
    assert!(error.message().contains("envelope source mismatch"));
    clean_db(&path);
}

#[test]
fn idempotent_replay_does_not_duplicate_evidence() {
    let authority = Authority::in_memory(config()).unwrap();
    authority
        .evaluate_and_issue_at("agent.requester", &intent("req-replay"), NOW)
        .unwrap();
    authority
        .evaluate_and_issue_at("agent.requester", &intent("req-replay"), NOW + 1_000)
        .unwrap();
    assert_eq!(authority.pending_evidence(10).unwrap().len(), 2);
    assert_eq!(authority.reconcile_evidence().unwrap().total, 2);
}

#[test]
fn weak_or_missing_sealing_configuration_fails_closed() {
    let error = KeyRing::active_profile([
        ("audit-test-v1".into(), vec![]),
        ("authorization-test-v1".into(), vec![0x5b; 32]),
        ("service-test-v1".into(), vec![0x5c; 32]),
        ("operator-test-v1".into(), vec![0x5d; 32]),
        ("tenant-test-v1".into(), vec![0x5e; 32]),
    ])
    .err()
    .unwrap();
    assert_eq!(error.code(), "KEY_CONFIGURATION_INVALID");
    assert!(error.message().contains("at least 32 bytes"));
}

fn rotated_keys() -> KeyRing {
    KeyRing::new(vec![
        RoleKey::verify_only("audit-test-v1", KeyPurpose::AuditSealing, vec![0x5a; 32]),
        RoleKey::active("audit-test-v2", KeyPurpose::AuditSealing, vec![0x61; 32]),
        RoleKey::verify_only(
            "authorization-test-v1",
            KeyPurpose::AuthorizationSigning,
            vec![0x5b; 32],
        ),
        RoleKey::active(
            "authorization-test-v2",
            KeyPurpose::AuthorizationSigning,
            vec![0x62; 32],
        ),
        RoleKey::active(
            "service-test-v1",
            KeyPurpose::ServiceIdentity,
            vec![0x5c; 32],
        ),
        RoleKey::active(
            "operator-test-v1",
            KeyPurpose::OperatorAuthentication,
            vec![0x5d; 32],
        ),
        RoleKey::active("tenant-test-v1", KeyPurpose::TenantTrust, vec![0x5e; 32]),
    ])
    .unwrap()
}

#[test]
fn role_key_rotation_preserves_history_and_uses_new_keys_for_new_proofs() {
    let path = temp_db("key-rotation");
    let issued = {
        let authority = Authority::open(&path, config()).unwrap();
        authority
            .evaluate_and_issue_at("agent.requester", &intent("req-before-rotation"), NOW)
            .unwrap()
            .authorization
            .unwrap()
    };
    assert_eq!(issued.authorization_signing_key_id, "authorization-test-v1");

    let mut after_rotation = config();
    after_rotation.evidence.keys = rotated_keys();
    let authority = Authority::open(&path, after_rotation).unwrap();
    assert_eq!(authority.reconcile_evidence().unwrap().total, 2);
    authority
        .claim_authenticated_at(
            &issued.authorization_id,
            &role_identity("runtime.mailer", LocalRole::Executor),
            &executed(),
            NOW + 1,
        )
        .unwrap();
    let later = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-after-rotation"), NOW + 2)
        .unwrap()
        .authorization
        .unwrap();
    assert_eq!(later.authorization_signing_key_id, "authorization-test-v2");
    let rows = authority.pending_evidence(20).unwrap();
    assert!(rows.iter().any(|row| row.seal_key_id == "audit-test-v1"));
    assert!(rows.iter().any(|row| row.seal_key_id == "audit-test-v2"));
    authority.reconcile_evidence().unwrap();
    drop(authority);
    clean_db(&path);
}

#[test]
fn signing_key_revocation_blocks_every_later_unclaimed_use() {
    let authority = Authority::in_memory(config()).unwrap();
    let issued = authority
        .evaluate_and_issue_at("agent.requester", &intent("req-key-revoked"), NOW)
        .unwrap()
        .authorization
        .unwrap();
    authority
        .revoke_authenticated_at(
            &role_identity("authority.emergency", LocalRole::EmergencyCanceller),
            RevocationScope::SigningKey,
            &issued.authorization_signing_key_id,
            RevocationReason::SigningKeyCompromised,
            NOW + 1,
        )
        .unwrap();
    let error = authority
        .claim_authenticated_at(
            &issued.authorization_id,
            &role_identity("runtime.mailer", LocalRole::Executor),
            &executed(),
            NOW + 2,
        )
        .unwrap_err();
    assert_eq!(error.code(), "AUTHORIZATION_REVOKED");
}

#[test]
fn revoked_active_keys_block_new_proofs_and_unknown_key_revocation_is_rejected() {
    let authority = Authority::in_memory(config()).unwrap();
    let revoker = role_identity("authority.emergency", LocalRole::EmergencyCanceller);
    let invalid = authority
        .revoke_authenticated_at(
            &revoker,
            RevocationScope::SigningKey,
            "attacker-invented-key",
            RevocationReason::SigningKeyCompromised,
            NOW,
        )
        .unwrap_err();
    assert_eq!(invalid.code(), "REVOCATION_INVALID");

    authority
        .revoke_authenticated_at(
            &revoker,
            RevocationScope::SigningKey,
            "authorization-test-v1",
            RevocationReason::SigningKeyCompromised,
            NOW + 1,
        )
        .unwrap();
    let issuance = authority
        .evaluate_and_issue_at(
            "agent.requester",
            &intent("req-after-signing-key-revocation"),
            NOW + 2,
        )
        .unwrap_err();
    assert_eq!(issuance.code(), "AUTHORITY_INTERNAL_ERROR");
    assert!(issuance.message().contains("durably revoked"));

    let second = Authority::in_memory(config()).unwrap();
    second
        .evaluate_and_issue_at("agent.requester", &intent("req-audit-key"), NOW)
        .unwrap();
    second
        .revoke_authenticated_at(
            &revoker,
            RevocationScope::SigningKey,
            "audit-test-v1",
            RevocationReason::SigningKeyCompromised,
            NOW + 1,
        )
        .unwrap();
    let reconciliation = second.reconcile_evidence().unwrap_err();
    assert_eq!(reconciliation.code(), "KEY_REVOKED");
}

#[test]
fn stored_authorization_proof_tampering_and_role_substitution_fail_closed() {
    for (label, mutation) in [
        (
            "signature",
            "UPDATE tlpx_authorizations SET authorization_signature = 'hmac-sha256:0000000000000000000000000000000000000000000000000000000000000000'",
        ),
        (
            "role",
            "UPDATE tlpx_authorizations SET authorization_signing_key_id = 'audit-test-v1'",
        ),
    ] {
        let path = temp_db(&format!("authorization-proof-{label}"));
        let issued = {
            let authority = Authority::open(&path, config()).unwrap();
            authority
                .evaluate_and_issue_at("agent.requester", &intent(&format!("req-{label}")), NOW)
                .unwrap()
                .authorization
                .unwrap()
        };
        Connection::open(&path)
            .unwrap()
            .execute(mutation, [])
            .unwrap();
        let authority = Authority::open(&path, config()).unwrap();
        let error = authority
            .claim_authenticated_at(
                &issued.authorization_id,
                &role_identity("runtime.mailer", LocalRole::Executor),
                &executed(),
                NOW + 1,
            )
            .unwrap_err();
        assert_eq!(error.code(), "AUTHORIZATION_PROOF_INVALID");
        drop(authority);
        clean_db(&path);
    }
}

#[test]
fn incompatible_pre_release_database_is_rejected_before_schema_changes() {
    let path = temp_db("legacy-schema");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE tlpx_authorizations (
                   authorization_id TEXT PRIMARY KEY
                 );",
            )
            .unwrap();
    }

    let error = Authority::open(&path, config()).err().unwrap();
    assert!(error
        .message()
        .contains("incompatible pre-release authority database"));
    assert!(error.message().contains("use a fresh database"));

    let connection = Connection::open(&path).unwrap();
    let created_sequence_table: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'tlpx_sequence')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!created_sequence_table);
    clean_db(&path);
}

#[test]
fn legacy_nonnullable_policy_hash_schema_is_rejected() {
    let path = temp_db("legacy-policy-hash-schema");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE tlpx_evaluations (
                   policy_id TEXT,
                   policy_bundle_hash TEXT NOT NULL
                 );",
            )
            .unwrap();
    }

    let error = Authority::open(&path, config()).err().unwrap();
    assert!(error.message().contains("policy_bundle_hash"));
    assert!(error.message().contains("use a fresh database"));

    let connection = Connection::open(&path).unwrap();
    let created_sequence_table: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'tlpx_sequence')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!created_sequence_table);
    clean_db(&path);
}
