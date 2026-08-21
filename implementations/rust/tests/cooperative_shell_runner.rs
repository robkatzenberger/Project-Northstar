use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Barrier;
use std::time::{SystemTime, UNIX_EPOCH};
use tlpx::{
    exact_match_policy_content_hash, sha256_file, Adapter, AdapterContract, AdapterRegistry,
    AuthenticatedAdapterSession, AuthenticatedIdentity, Authority, AuthorityConfig,
    AuthorizationTemplate, AuthorizedAction, AuthzState, CapabilityRegistry,
    ConfiguredPolicyBundle, CooperativeShellConfig, CooperativeShellOutcome,
    CooperativeShellRequest, CooperativeShellRunner, Decision, EvidenceConfig, LocalAuthenticator,
    LocalPrincipalMapping, LocalRole, PartyType, PolicyBundle, PolicyBundleManifest, PolicyCatalog,
    PolicyEffect, PolicyIssuer, PolicyIssuerType, PolicyRule, Principal, Risk, ShellExecutable,
    SubmittedIntent, Switchboard, Value, ADAPTER_MATERIAL_FIELDS, EXACT_MATCH_POLICY_CONTENT_TYPE,
    POLICY_PRECEDENCE,
};

const ADAPTER_HASH: &str =
    "sha256:8888888888888888888888888888888888888888888888888888888888888888";

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
        principal_id: "adapter.shell.local".into(),
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

fn authority_config(targets: &[String]) -> AuthorityConfig {
    let template = AuthorizationTemplate {
        derived_risk: Risk::High,
        capability: "shell.command".into(),
        resource_scope: targets.to_vec(),
        risk_reasons: vec!["protected_shell_command".into()],
        risk_source: "policy:cooperative-shell-runner@1.0.0".into(),
    };
    let policy = PolicyBundle {
        rules: vec![PolicyRule {
            id: "allow-shell-prototype".into(),
            action: "shell.exec".into(),
            effect: PolicyEffect::allow("POLICY_ALLOW", template),
        }],
        default: PolicyEffect::deny("POLICY_DENY"),
    };
    let content_hash = exact_match_policy_content_hash(&policy).unwrap();
    AuthorityConfig {
        policy: PolicyCatalog::new(vec![ConfiguredPolicyBundle {
            manifest: PolicyBundleManifest {
                policy_bundle_id: "cooperative-shell-runner".into(),
                policy_bundle_version: "1.0.0".into(),
                issuer: PolicyIssuer {
                    id: "security.platform".into(),
                    kind: PolicyIssuerType::Human,
                },
                content_type: EXACT_MATCH_POLICY_CONTENT_TYPE.into(),
                content_hash,
                activated_at: "2020-01-01T00:00:00.000Z".into(),
                retired_at: None,
                environment: "test".into(),
                tenant: "local_test".into(),
                precedence: POLICY_PRECEDENCE
                    .iter()
                    .map(|value| (*value).into())
                    .collect(),
                default_decision: Decision::Deny,
                supersedes: None,
            },
            policy,
        }]),
        switchboard: Switchboard::new(vec![
            Principal {
                id: "agent.requester".into(),
                active: true,
                allowed_actions: vec!["shell.exec".into()],
            },
            Principal {
                id: "runtime.shell".into(),
                active: true,
                allowed_actions: vec!["shell.exec".into()],
            },
        ])
        .unwrap(),
        capabilities: CapabilityRegistry::new(vec![(
            "shell.command".into(),
            vec!["adapter.shell".into()],
        )])
        .unwrap(),
        adapters: AdapterRegistry::new(vec![AdapterContract {
            adapter_id: "adapter.shell".into(),
            adapter_version: "1.0.0".into(),
            authenticated_principal: "adapter.shell.local".into(),
            authenticated_authority: "authority.local".into(),
            binary_hash: ADAPTER_HASH.into(),
            capabilities: vec!["shell.command".into()],
            actions: vec!["shell.exec".into()],
            material_fields: ADAPTER_MATERIAL_FIELDS
                .iter()
                .map(|field| (*field).into())
                .collect(),
        }])
        .unwrap(),
        evidence: EvidenceConfig {
            evaluator_id: "authority.local".into(),
            router_id: "switchboard.local".into(),
            requester_type: PartyType::Machine,
            seal_key_id: "cooperative-shell-runner-test".into(),
            seal_key: vec![0x71; 32],
        },
        approval_window_ms: 60_000,
        claim_window_ms: 60_000,
        execution_lease_ms: 2_000,
    }
}

fn plan(argv: Vec<String>, cwd: &Path, env: Vec<(&str, &str)>, timeout_ms: i64) -> Value {
    Value::Object(vec![
        (
            "argv".into(),
            Value::Array(argv.into_iter().map(Value::String).collect()),
        ),
        (
            "cwd".into(),
            Value::String(cwd.to_string_lossy().into_owned()),
        ),
        (
            "env".into(),
            Value::Object(
                env.into_iter()
                    .map(|(name, value)| (name.into(), Value::String(value.into())))
                    .collect(),
            ),
        ),
        ("timeout_ms".into(), Value::Int(timeout_ms)),
    ])
}

fn issue(
    authority: &Authority,
    target: &Path,
    arguments: Value,
    request_id: &str,
) -> (tlpx::IssuedAuthorization, AuthorizedAction) {
    let intent = SubmittedIntent {
        requesting_principal: "agent.requester".into(),
        executing_principal: "runtime.shell".into(),
        action: "shell.exec".into(),
        intent_class: "protected_command".into(),
        target: target.to_string_lossy().into_owned(),
        arguments: arguments.clone(),
        environment: "test".into(),
        tenant: "local_test".into(),
        declared_risk: Risk::High,
        data_classes: vec![],
        requested_capability: "shell.command".into(),
        resource_scope: vec![target.to_string_lossy().into_owned()],
        payload_hash: None,
        artifact_hash: None,
        adapter: Adapter {
            id: "adapter.shell".into(),
            version: "1.0.0".into(),
        },
        request_id: request_id.into(),
        retry_of_receipt_id: None,
    };
    let issued = authority
        .evaluate_authenticated(&identity("agent.requester", LocalRole::Requester), &intent)
        .unwrap()
        .authorization
        .unwrap();
    let authorized = AuthorizedAction {
        requesting_principal: intent.requesting_principal,
        executing_principal: intent.executing_principal,
        action: intent.action,
        target: intent.target,
        arguments,
        environment: intent.environment,
        tenant: intent.tenant,
        derived_risk: Risk::High,
        effective_risk: Risk::High,
        risk_reasons: vec!["protected_shell_command".into()],
        risk_source: "policy:cooperative-shell-runner@1.0.0".into(),
        data_classes: vec![],
        capability: "shell.command".into(),
        resource_scope: authority_targets(&issued),
        payload_hash: None,
        artifact_hash: None,
        policy_bundle_hash: issued.policy_bundle_hash.clone(),
        adapter: intent.adapter,
    };
    assert_eq!(
        authorized.authorized_action_hash().unwrap(),
        issued.authorized_action_hash
    );
    (issued, authorized)
}

fn authority_targets(issued: &tlpx::IssuedAuthorization) -> Vec<String> {
    issued.resource_scope.clone()
}

fn executable(path: &str) -> (PathBuf, ShellExecutable) {
    let canonical = Path::new(path).canonicalize().unwrap();
    let pinned = ShellExecutable::pinned(&canonical, sha256_file(&canonical).unwrap()).unwrap();
    (canonical, pinned)
}

fn runner_config(executables: Vec<ShellExecutable>, output_limit: usize) -> CooperativeShellConfig {
    CooperativeShellConfig::new(
        executables,
        vec![std::env::temp_dir()],
        vec!["DEMO_FLAG".into()],
        output_limit,
        1_000,
    )
    .unwrap()
}

fn temp_marker(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("northstar-{label}-{nonce}"))
}

#[test]
fn cooperative_demo_binary_completes_narrow_marker_smoke_path() {
    let directory = temp_marker("binary-smoke-dir");
    std::fs::create_dir(&directory).unwrap();
    let database = directory.join("authority.sqlite");
    let marker = directory.join("marker");
    let output = Command::new(env!("CARGO_BIN_EXE_tlpx-run-demo"))
        .args([
            database.as_os_str(),
            marker.as_os_str(),
            std::ffi::OsStr::new("binary-smoke-request"),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(marker.exists());
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("state=COMPLETED"));
    std::fs::remove_file(&marker).unwrap();
    let replay = Command::new(env!("CARGO_BIN_EXE_tlpx-run-demo"))
        .args([
            database.as_os_str(),
            marker.as_os_str(),
            std::ffi::OsStr::new("binary-smoke-request"),
        ])
        .output()
        .unwrap();
    assert!(!replay.status.success());
    std::fs::remove_dir_all(directory).unwrap();
}

fn execute(
    runner: &CooperativeShellRunner<'_>,
    issued: &tlpx::IssuedAuthorization,
    authorized: &AuthorizedAction,
) -> tlpx::Result<CooperativeShellOutcome> {
    runner.execute(CooperativeShellRequest {
        authorization: issued,
        authorized_action: authorized,
        executor: &identity("runtime.shell", LocalRole::Executor),
        adapter: &adapter_session(),
        adapter_binary_hash: ADAPTER_HASH,
    })
}

#[test]
fn runner_startup_rejects_shell_interpreters_and_executable_digest_mismatch() {
    let shell_hash = sha256_file("/bin/sh").unwrap();
    let shell = ShellExecutable::pinned("/bin/sh", shell_hash).unwrap_err();
    assert_eq!(shell.code(), "SHELL_RUNNER_CONFIG_INVALID");
    let wrong_digest = ShellExecutable::pinned(
        "/bin/echo",
        "sha256:9999999999999999999999999999999999999999999999999999999999999999",
    )
    .unwrap_err();
    assert_eq!(wrong_digest.code(), "SHELL_RUNNER_CONFIG_INVALID");
}

#[test]
fn protected_marker_executes_once_and_replay_is_blocked() {
    let (touch, pinned) = executable("/usr/bin/touch");
    let authority =
        Authority::in_memory(authority_config(&[touch.to_string_lossy().into()])).unwrap();
    let marker = temp_marker("runner-once");
    let arguments = plan(
        vec![marker.to_string_lossy().into_owned()],
        &std::env::temp_dir(),
        vec![],
        500,
    );
    let (issued, authorized) = issue(&authority, &touch, arguments, "runner-once");
    let config = runner_config(vec![pinned], 1_024);
    let runner = CooperativeShellRunner::new(&authority, config);
    let outcome = execute(&runner, &issued, &authorized).unwrap();
    assert_eq!(outcome.receipt.state, tlpx::ExecutionState::Completed);
    assert!(marker.exists());
    let replay = execute(&runner, &issued, &authorized).unwrap_err();
    assert_eq!(replay.code(), "ALREADY_CLAIMED");
    std::fs::remove_file(marker).unwrap();
}

#[test]
fn action_mutation_and_unactivated_executable_block_before_claim() {
    let (touch, pinned_touch) = executable("/usr/bin/touch");
    let (echo, pinned_echo) = executable("/bin/echo");
    let targets = vec![
        touch.to_string_lossy().into_owned(),
        echo.to_string_lossy().into_owned(),
    ];
    let authority = Authority::in_memory(authority_config(&targets)).unwrap();
    let marker = temp_marker("runner-mutation");
    let arguments = plan(
        vec![marker.to_string_lossy().into_owned()],
        &std::env::temp_dir(),
        vec![],
        500,
    );
    let (issued, mut authorized) = issue(&authority, &touch, arguments, "runner-mutation");
    let original = authorized.arguments.clone();
    authorized.arguments = plan(
        vec![temp_marker("different").to_string_lossy().into_owned()],
        &std::env::temp_dir(),
        vec![],
        500,
    );
    let runner = CooperativeShellRunner::new(&authority, runner_config(vec![pinned_touch], 1_024));
    let error = execute(&runner, &issued, &authorized).unwrap_err();
    assert_eq!(error.code(), "SHELL_RUNNER_REQUEST_INVALID");
    assert_eq!(
        authority.state(&issued.authorization_id).unwrap(),
        Some(AuthzState::AuthorizedUnclaimed)
    );
    assert!(!marker.exists());

    authorized.arguments = original;
    let runner = CooperativeShellRunner::new(&authority, runner_config(vec![pinned_echo], 1_024));
    let error = execute(&runner, &issued, &authorized).unwrap_err();
    assert_eq!(error.code(), "SHELL_RUNNER_COMMAND_DENIED");
    assert_eq!(
        authority.state(&issued.authorization_id).unwrap(),
        Some(AuthzState::AuthorizedUnclaimed)
    );
}

#[test]
fn shell_metacharacters_are_literal_and_output_is_bounded() {
    let (echo, pinned) = executable("/bin/echo");
    let authority =
        Authority::in_memory(authority_config(&[echo.to_string_lossy().into()])).unwrap();
    let marker = temp_marker("no-interpolation");
    let literal = format!("$(/usr/bin/touch {})", marker.to_string_lossy());
    let arguments = plan(vec![literal], &std::env::temp_dir(), vec![], 500);
    let (issued, authorized) = issue(&authority, &echo, arguments, "runner-literal");
    let runner = CooperativeShellRunner::new(&authority, runner_config(vec![pinned], 12));
    let outcome = execute(&runner, &issued, &authorized).unwrap();
    assert_eq!(outcome.receipt.state, tlpx::ExecutionState::Completed);
    assert!(outcome.stdout_truncated);
    assert_eq!(outcome.stdout.len(), 12);
    assert!(!marker.exists());
    assert!(!outcome
        .receipt
        .result
        .result_summary
        .unwrap()
        .contains(&marker.to_string_lossy().to_string()));
}

#[test]
fn environment_is_cleared_and_only_allowlisted_values_are_passed() {
    let (env, pinned) = executable("/usr/bin/printenv");
    let authority =
        Authority::in_memory(authority_config(&[env.to_string_lossy().into()])).unwrap();
    let arguments = plan(
        vec![],
        &std::env::temp_dir(),
        vec![("DEMO_FLAG", "present")],
        500,
    );
    let (issued, authorized) = issue(&authority, &env, arguments, "runner-env");
    let runner = CooperativeShellRunner::new(&authority, runner_config(vec![pinned], 1_024));
    let outcome = execute(&runner, &issued, &authorized).unwrap();
    assert_eq!(
        String::from_utf8(outcome.stdout).unwrap(),
        "DEMO_FLAG=present\n"
    );
}

#[test]
fn duration_limit_kills_command_and_records_failure() {
    let (sleep, pinned) = executable("/bin/sleep");
    let authority =
        Authority::in_memory(authority_config(&[sleep.to_string_lossy().into()])).unwrap();
    let arguments = plan(vec!["1".into()], &std::env::temp_dir(), vec![], 20);
    let (issued, authorized) = issue(&authority, &sleep, arguments, "runner-timeout");
    let runner = CooperativeShellRunner::new(&authority, runner_config(vec![pinned], 1_024));
    let outcome = execute(&runner, &issued, &authorized).unwrap();
    assert!(outcome.timed_out);
    assert_eq!(outcome.receipt.state, tlpx::ExecutionState::Failed);
}

#[test]
fn duration_must_be_strictly_shorter_than_the_execution_lease() {
    let (sleep, pinned) = executable("/bin/sleep");
    let authority =
        Authority::in_memory(authority_config(&[sleep.to_string_lossy().into()])).unwrap();
    let arguments = plan(vec!["0".into()], &std::env::temp_dir(), vec![], 2_000);
    let (issued, authorized) = issue(&authority, &sleep, arguments, "runner-lease-bound");
    let config = CooperativeShellConfig::new(
        vec![pinned],
        vec![std::env::temp_dir()],
        vec![],
        1_024,
        3_000,
    )
    .unwrap();
    let runner = CooperativeShellRunner::new(&authority, config);
    let error = execute(&runner, &issued, &authorized).unwrap_err();
    assert_eq!(error.code(), "SHELL_RUNNER_COMMAND_DENIED");
    assert_eq!(
        authority.state(&issued.authorization_id).unwrap(),
        Some(AuthzState::AuthorizedUnclaimed)
    );
}

#[test]
fn spawn_failure_after_start_records_terminal_failure() {
    let program = temp_marker("runner-non-executable");
    std::fs::write(&program, b"not executable\n").unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o600)).unwrap();
    let pinned = ShellExecutable::pinned(&program, sha256_file(&program).unwrap()).unwrap();
    let canonical = program.canonicalize().unwrap();
    let authority =
        Authority::in_memory(authority_config(&[canonical.to_string_lossy().into()])).unwrap();
    let arguments = plan(vec![], &std::env::temp_dir(), vec![], 500);
    let (issued, authorized) = issue(&authority, &canonical, arguments, "runner-spawn-fail");
    let runner = CooperativeShellRunner::new(&authority, runner_config(vec![pinned], 1_024));
    let outcome = execute(&runner, &issued, &authorized).unwrap();
    assert_eq!(outcome.receipt.state, tlpx::ExecutionState::Failed);
    assert_eq!(outcome.exit_code, None);
    assert_eq!(
        authority.state(&issued.authorization_id).unwrap(),
        Some(AuthzState::Claimed)
    );
    std::fs::remove_file(program).unwrap();
}

#[test]
fn concurrent_execute_has_one_claim_and_one_side_effect() {
    let (touch, pinned) = executable("/usr/bin/touch");
    let authority =
        Authority::in_memory(authority_config(&[touch.to_string_lossy().into()])).unwrap();
    let marker = temp_marker("runner-concurrent");
    let arguments = plan(
        vec![marker.to_string_lossy().into_owned()],
        &std::env::temp_dir(),
        vec![],
        500,
    );
    let (issued, authorized) = issue(&authority, &touch, arguments, "runner-concurrent");
    let runner = CooperativeShellRunner::new(&authority, runner_config(vec![pinned], 1_024));
    let barrier = Barrier::new(3);
    let results = std::thread::scope(|scope| {
        let left = scope.spawn(|| {
            barrier.wait();
            execute(&runner, &issued, &authorized)
        });
        let right = scope.spawn(|| {
            barrier.wait();
            execute(&runner, &issued, &authorized)
        });
        barrier.wait();
        [left.join().unwrap(), right.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .filter(|error| error.code() == "ALREADY_CLAIMED")
            .count(),
        1
    );
    assert!(marker.exists());
    std::fs::remove_file(marker).unwrap();
}
