#![cfg(not(feature = "deterministic-time"))]

use rusqlite::Connection;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tlpx::{
    exact_match_policy_content_hash, Adapter, AdapterContract, AdapterRegistry, ApprovalOutcome,
    ApprovalPresentation, ApprovalState, AuthenticatedAdapterSession, AuthenticatedIdentity,
    Authority, AuthorityConfig, AuthorizationTemplate, AuthzState, CapabilityRegistry,
    ConfiguredPolicyBundle, EvidenceConfig, ExecutedAction, ExecutionResultEvidence,
    ExecutionStart, ExecutionState, KeyRing, LocalAuthenticator, LocalPrincipalMapping, LocalRole,
    PartyType, PolicyBundle, PolicyBundleManifest, PolicyCatalog, PolicyEffect, PolicyIssuer,
    PolicyIssuerType, PolicyRule, Principal, Risk, SubmittedIntent, Switchboard, Value,
    ADAPTER_MATERIAL_FIELDS, EXACT_MATCH_POLICY_CONTENT_TYPE, POLICY_PRECEDENCE,
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

fn adapter_session() -> AuthenticatedAdapterSession {
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
    config_with_review(false)
}

fn config_with_review(require_review: bool) -> AuthorityConfig {
    let template = AuthorizationTemplate {
        derived_risk: Risk::High,
        capability: "mailer.send".into(),
        resource_scope: vec!["customer:123".into()],
        risk_reasons: vec!["external_communication".into()],
        risk_source: "policy:trusted-time-production@1.0.0".into(),
    };
    let effect = if require_review {
        PolicyEffect::require_approval(
            "POLICY_REQUIRE_APPROVAL",
            template,
            vec!["ops.mailer".into()],
        )
    } else {
        PolicyEffect::allow("POLICY_ALLOW", template)
    };
    let policy = PolicyBundle {
        rules: vec![PolicyRule {
            id: "allow-email".into(),
            action: "send_email".into(),
            effect,
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
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    TempDb(std::env::temp_dir().join(format!(
        "northstar-trusted-time-production-{}-{nonce}-{}.sqlite",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
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

#[test]
fn claim_samples_production_time_after_waiting_for_write_transaction() {
    let database = temp_db();
    let mut authority_config = config();
    authority_config.claim_window_ms = 1_500;
    let authority = Authority::open(&database.0, authority_config).unwrap();
    let issued = authority
        .evaluate_authenticated(
            &identity("agent.requester", LocalRole::Requester),
            &intent(),
        )
        .unwrap()
        .authorization
        .unwrap();

    let blocker = Connection::open(&database.0).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();

    let remaining_ms = issued.claim_expires_at_ms - wall_time_ms();
    assert!(
        remaining_ms >= 1_000,
        "test fixture must acquire the blocking transaction well before claim expiry"
    );

    let authorization_id = issued.authorization_id.clone();
    let (call_started_tx, call_started_rx) = mpsc::channel();
    let claim_thread = thread::spawn(move || {
        call_started_tx.send(wall_time_ms()).unwrap();
        let result = authority.claim_authenticated(
            &authorization_id,
            &identity("runtime.mailer", LocalRole::Executor),
            &executed(),
        );
        (result, authority)
    });

    let call_started_at_ms = call_started_rx.recv().unwrap();
    assert!(
        call_started_at_ms < issued.claim_expires_at_ms,
        "claim call must begin before its deadline"
    );
    thread::sleep(Duration::from_millis(100));
    assert!(
        wall_time_ms() < issued.claim_expires_at_ms,
        "claim must have time to block on the write transaction before expiry"
    );

    let release_at_ms = issued.claim_expires_at_ms.saturating_add(100);
    let wait_ms = release_at_ms.saturating_sub(wall_time_ms()).max(0);
    thread::sleep(Duration::from_millis(wait_ms as u64));
    assert!(wall_time_ms() >= issued.claim_expires_at_ms);
    blocker.execute_batch("COMMIT").unwrap();

    let (claim_result, authority) = claim_thread.join().unwrap();
    let error = claim_result.unwrap_err();
    assert_eq!(error.code(), "AUTHORIZATION_EXPIRED");
    assert_eq!(
        authority.state(&issued.authorization_id).unwrap(),
        Some(AuthzState::Expired)
    );
}

#[test]
fn execution_start_samples_production_time_after_waiting_for_write_transaction() {
    let database = temp_db();
    let mut authority_config = config();
    authority_config.claim_window_ms = 3_000;
    authority_config.execution_lease_ms = 2_000;
    let authority = Authority::open(&database.0, authority_config).unwrap();
    let issued = authority
        .evaluate_authenticated(
            &identity("agent.requester", LocalRole::Requester),
            &intent(),
        )
        .unwrap()
        .authorization
        .unwrap();
    let claim = authority
        .claim_authenticated(
            &issued.authorization_id,
            &identity("runtime.mailer", LocalRole::Executor),
            &executed(),
        )
        .unwrap();

    let blocker = Connection::open(&database.0).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    let remaining_ms = claim.lease_expires_at_ms - wall_time_ms();
    assert!(
        remaining_ms >= 1_500,
        "test fixture must acquire the blocking transaction well before lease expiry"
    );

    let idempotency_key = issued.idempotency_key.clone();
    let claim_id = claim.claim_id.clone();
    let (call_started_tx, call_started_rx) = mpsc::channel();
    let start_thread = thread::spawn(move || {
        call_started_tx.send(wall_time_ms()).unwrap();
        let result = authority.begin_execution_authenticated(
            &claim_id,
            &idempotency_key,
            &executed(),
            &identity("runtime.mailer", LocalRole::Executor),
            &adapter_session(),
            &hash('8'),
        );
        (result, authority)
    });

    let call_started_at_ms = call_started_rx.recv().unwrap();
    assert!(
        call_started_at_ms < claim.lease_expires_at_ms,
        "execution-start call must begin before its lease deadline"
    );
    thread::sleep(Duration::from_millis(100));
    assert!(
        wall_time_ms() < claim.lease_expires_at_ms,
        "execution start must have time to block before lease expiry"
    );

    let release_at_ms = claim.lease_expires_at_ms.saturating_add(100);
    let wait_ms = release_at_ms.saturating_sub(wall_time_ms()).max(0);
    thread::sleep(Duration::from_millis(wait_ms as u64));
    assert!(wall_time_ms() >= claim.lease_expires_at_ms);
    blocker.execute_batch("COMMIT").unwrap();

    let (start_result, _authority) = start_thread.join().unwrap();
    match start_result.unwrap() {
        ExecutionStart::NotStarted(lease) => {
            assert_eq!(lease.state, ExecutionState::LeaseExpired);
        }
        ExecutionStart::Started(_) => panic!("expired execution lease granted spawn permission"),
    }
}

#[test]
fn explicit_time_is_validated_at_ingress_but_sampled_after_lock_wait() {
    let database = temp_db();
    let mut authority_config = config();
    authority_config.claim_window_ms = 3_000;
    let authority = Authority::open(&database.0, authority_config).unwrap();
    let issued = authority
        .evaluate_authenticated(
            &identity("agent.requester", LocalRole::Requester),
            &intent(),
        )
        .unwrap()
        .authorization
        .unwrap();

    let untrusted_future = wall_time_ms().saturating_add(10_000);
    let error = authority
        .claim_authenticated_at(
            &issued.authorization_id,
            &identity("runtime.mailer", LocalRole::Executor),
            &executed(),
            untrusted_future,
        )
        .unwrap_err();
    assert_eq!(error.code(), "TRUSTED_TIME_INVALID");

    let database_path = database.0.clone();
    let (locked_tx, locked_rx) = mpsc::channel();
    let (released_tx, released_rx) = mpsc::channel();
    let blocker_thread = thread::spawn(move || {
        let blocker = Connection::open(database_path).unwrap();
        blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
        locked_tx.send(()).unwrap();
        thread::sleep(Duration::from_millis(1_200));
        let released_at_ms = wall_time_ms();
        blocker.execute_batch("COMMIT").unwrap();
        released_tx.send(released_at_ms).unwrap();
    });

    locked_rx.recv().unwrap();
    let claim = authority
        .claim_authenticated_at(
            &issued.authorization_id,
            &identity("runtime.mailer", LocalRole::Executor),
            &executed(),
            wall_time_ms(),
        )
        .unwrap();
    let released_at_ms = released_rx.recv().unwrap();
    blocker_thread.join().unwrap();
    assert!(released_at_ms < issued.claim_expires_at_ms);
    assert!(
        claim.claimed_at_ms >= released_at_ms,
        "production transition time must be sampled after the lock wait"
    );
}

#[test]
fn runtime_execution_finish_retry_returns_the_durable_receipt() {
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
    let executor = identity("runtime.mailer", LocalRole::Executor);
    let claim = authority
        .claim_authenticated(&issued.authorization_id, &executor, &executed())
        .unwrap();
    let lease = authority
        .begin_execution_authenticated(
            &claim.claim_id,
            &issued.idempotency_key,
            &executed(),
            &executor,
            &adapter_session(),
            &hash('8'),
        )
        .unwrap()
        .into_started()
        .unwrap();
    let result = ExecutionResultEvidence {
        result_summary: Some("protected operation completed".into()),
        result_hash: None,
        external_evidence_reference: None,
    };
    let receipt = authority
        .finish_execution_authenticated(
            &lease.execution_id,
            &executor,
            ExecutionState::Completed,
            result.clone(),
            None,
        )
        .unwrap();

    thread::sleep(Duration::from_millis(5));
    let retry = authority
        .finish_execution_authenticated(
            &lease.execution_id,
            &executor,
            ExecutionState::Completed,
            result,
            None,
        )
        .unwrap();
    assert_eq!(retry, receipt);
}

#[test]
fn approval_samples_production_time_after_waiting_for_write_transaction() {
    let database = temp_db();
    let mut authority_config = config_with_review(true);
    authority_config.approval_window_ms = 1_500;
    let authority = Authority::open(&database.0, authority_config).unwrap();
    let pending = authority
        .evaluate_authenticated(
            &identity("agent.requester", LocalRole::Requester),
            &intent(),
        )
        .unwrap();
    assert!(pending.authorization.is_none());

    // Same-UID fixture: exercises the authenticated human role and approval
    // route, not OS identity separation or an actual approval interface.
    let authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: "human.operator".into(),
        party_type: PartyType::Human,
        roles: vec![LocalRole::Operator],
        approval_routes: vec!["ops.mailer".into()],
    }])
    .unwrap();
    let (server, _client) = UnixStream::pair().unwrap();
    let operator = authenticator.authenticate_stream(&server).unwrap();
    let view = authority
        .pending_approval_authenticated(&pending.receipt_id, &operator)
        .unwrap();
    let presentation = ApprovalPresentation {
        authorized_action_hash: view.authorized_action_hash,
        renderer_id: "approval.test".into(),
        renderer_version: "1.0.0".into(),
    };
    let blocker = Connection::open(&database.0).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(view.approval_expires_at_ms - wall_time_ms() >= 1_000);

    let receipt_id = pending.receipt_id.clone();
    let (call_started_tx, call_started_rx) = mpsc::channel();
    let approval_thread = thread::spawn(move || {
        call_started_tx.send(wall_time_ms()).unwrap();
        let result = authority.resolve_pending_authenticated(
            &receipt_id,
            &operator,
            ApprovalOutcome::Approve,
            presentation,
        );
        (result, authority)
    });
    assert!(call_started_rx.recv().unwrap() < view.approval_expires_at_ms);
    thread::sleep(Duration::from_millis(100));
    assert!(wall_time_ms() < view.approval_expires_at_ms);
    let wait_ms = (view.approval_expires_at_ms + 100 - wall_time_ms()).max(0);
    thread::sleep(Duration::from_millis(wait_ms as u64));
    assert!(wall_time_ms() >= view.approval_expires_at_ms);
    blocker.execute_batch("COMMIT").unwrap();

    let (result, authority) = approval_thread.join().unwrap();
    assert_eq!(result.unwrap_err().code(), "APPROVAL_EXPIRED");
    assert_eq!(
        authority.approval_state(&pending.receipt_id).unwrap(),
        Some(ApprovalState::Expired)
    );
    let authorization_count: i64 = blocker
        .query_row("SELECT COUNT(*) FROM tlpx_authorizations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(
        authorization_count, 0,
        "expired approval must issue nothing"
    );
    authority.reconcile_evidence().unwrap();
}

#[test]
fn runtime_reconciliation_retry_returns_the_durable_receipt() {
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
    let executor = identity("runtime.mailer", LocalRole::Executor);
    let reconciler = identity("runtime.reconciler", LocalRole::Reconciler);
    let claim = authority
        .claim_authenticated(&issued.authorization_id, &executor, &executed())
        .unwrap();
    let lease = authority
        .begin_execution_authenticated(
            &claim.claim_id,
            &issued.idempotency_key,
            &executed(),
            &executor,
            &adapter_session(),
            &hash('8'),
        )
        .unwrap()
        .into_started()
        .unwrap();
    authority
        .mark_execution_outcome_unknown_authenticated(&lease.execution_id, &executor)
        .unwrap();
    authority
        .require_reconciliation_authenticated_at(&lease.execution_id, &reconciler, wall_time_ms())
        .unwrap();
    let result = ExecutionResultEvidence {
        result_summary: Some("protected operation confirmed by reconciler".into()),
        result_hash: None,
        external_evidence_reference: None,
    };
    let receipt = authority
        .reconcile_execution_authenticated_at(
            &lease.execution_id,
            &reconciler,
            ExecutionState::CompletedConfirmed,
            result.clone(),
            wall_time_ms(),
        )
        .unwrap();
    let evidence_before = authority.pending_evidence(100).unwrap();
    thread::sleep(Duration::from_millis(5));
    let retry = authority
        .reconcile_execution_authenticated_at(
            &lease.execution_id,
            &reconciler,
            ExecutionState::CompletedConfirmed,
            result.clone(),
            wall_time_ms(),
        )
        .unwrap();
    assert_eq!(retry, receipt);

    let mut changed_result = result;
    changed_result.result_summary = Some("different observation".into());
    let error = authority
        .reconcile_execution_authenticated_at(
            &lease.execution_id,
            &reconciler,
            ExecutionState::CompletedConfirmed,
            changed_result,
            wall_time_ms(),
        )
        .unwrap_err();
    assert_eq!(error.code(), "IDEMPOTENCY_CONFLICT");
    assert_eq!(authority.pending_evidence(100).unwrap(), evidence_before);
    assert_eq!(
        authority.execution_state(&lease.execution_id).unwrap(),
        Some(ExecutionState::CompletedConfirmed)
    );
    assert_eq!(
        authority.state(&issued.authorization_id).unwrap(),
        Some(AuthzState::Claimed)
    );
    authority.reconcile_evidence().unwrap();
}
