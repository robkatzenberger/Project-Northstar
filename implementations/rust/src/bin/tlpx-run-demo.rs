//! Cooperative slice 3.8 `tlpx-run-demo` demonstration.
//!
//! This binary intentionally protects one operation: creating one new marker
//! with `/usr/bin/touch`. It runs under the caller's UID and is not the 3.9
//! separate-identity forced-mediation acceptance environment.

use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use tlpx::{
    exact_match_policy_content_hash, sha256_file, Adapter, AdapterContract, AdapterRegistry,
    AuthenticatedAdapterSession, AuthenticatedIdentity, Authority, AuthorityConfig,
    AuthorizationTemplate, AuthorizedAction, CapabilityRegistry, ConfiguredPolicyBundle,
    CooperativeShellConfig, CooperativeShellRequest, CooperativeShellRunner, Decision,
    EvidenceConfig, KeyRing, LocalAuthenticator, LocalPrincipalMapping, LocalRole, PartyType,
    PolicyBundle, PolicyBundleManifest, PolicyCatalog, PolicyEffect, PolicyIssuer,
    PolicyIssuerType, PolicyRule, Principal, Risk, ShellExecutable, SubmittedIntent, Switchboard,
    Value, ADAPTER_MATERIAL_FIELDS, EXACT_MATCH_POLICY_CONTENT_TYPE, POLICY_PRECEDENCE,
};

fn main() -> tlpx::Result<()> {
    let mut args = std::env::args().skip(1);
    let database = required(&mut args, "DATABASE")?;
    let output = PathBuf::from(required(&mut args, "NEW_MARKER_PATH")?);
    let request_id = required(&mut args, "REQUEST_ID")?;
    if args.next().is_some() {
        return Err(usage());
    }
    let marker = normalize_new_marker(&output)?;
    let executable = Path::new("/usr/bin/touch").canonicalize().map_err(|_| {
        tlpx::Error::coded(
            "SHELL_RUNNER_CONFIG_INVALID",
            "prototype requires /usr/bin/touch",
        )
    })?;
    let working_directory = marker
        .parent()
        .ok_or_else(|| tlpx::Error::coded("SHELL_RUNNER_REQUEST_INVALID", "marker has no parent"))?
        .to_path_buf();
    let adapter_binary_hash = sha256_file(std::env::current_exe().map_err(|_| {
        tlpx::Error::coded(
            "SHELL_RUNNER_CONFIG_INVALID",
            "current executable path is unavailable",
        )
    })?)?;
    let authority = Authority::open(
        database,
        prototype_authority_config(&executable, &adapter_binary_hash)?,
    )?;
    let intent = prototype_intent(&executable, &working_directory, &marker, request_id);
    let issued = authority
        .evaluate_authenticated(
            &identity("prototype.requester", LocalRole::Requester)?,
            &intent,
        )?
        .authorization
        .ok_or_else(|| {
            tlpx::Error::coded("AUTHORIZATION_DENIED", "prototype was not authorized")
        })?;
    let authorized = authorized_action(&intent, &issued);
    let runner_config = CooperativeShellConfig::new(
        vec![ShellExecutable::pinned(
            &executable,
            sha256_file(&executable)?,
        )?],
        vec![working_directory],
        vec![],
        4_096,
        1_000,
    )?;
    let runner = CooperativeShellRunner::new(&authority, runner_config);
    let executor = identity("prototype.executor", LocalRole::Executor)?;
    let adapter = adapter_session()?;
    let outcome = runner.execute(CooperativeShellRequest {
        authorization: &issued,
        authorized_action: &authorized,
        executor: &executor,
        adapter: &adapter,
        adapter_binary_hash: &adapter_binary_hash,
    })?;
    println!(
        "execution={} state={} marker={}",
        outcome.receipt.execution_id,
        outcome.receipt.state.as_str(),
        marker.display()
    );
    Ok(())
}

fn required(args: &mut impl Iterator<Item = String>, _name: &str) -> tlpx::Result<String> {
    args.next().ok_or_else(usage)
}

fn usage() -> tlpx::Error {
    tlpx::Error::coded(
        "SHELL_RUNNER_REQUEST_INVALID",
        "usage: tlpx-run-demo DATABASE NEW_MARKER_PATH REQUEST_ID",
    )
}

fn normalize_new_marker(path: &Path) -> tlpx::Result<PathBuf> {
    if !path.is_absolute() || path.exists() {
        return Err(tlpx::Error::coded(
            "SHELL_RUNNER_REQUEST_INVALID",
            "marker path must be absolute and must not already exist",
        ));
    }
    let file_name = path.file_name().ok_or_else(|| {
        tlpx::Error::coded("SHELL_RUNNER_REQUEST_INVALID", "marker filename is missing")
    })?;
    if file_name.is_empty() {
        return Err(tlpx::Error::coded(
            "SHELL_RUNNER_REQUEST_INVALID",
            "marker filename is empty",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| {
            tlpx::Error::coded("SHELL_RUNNER_REQUEST_INVALID", "marker parent is missing")
        })?
        .canonicalize()
        .map_err(|_| {
            tlpx::Error::coded(
                "SHELL_RUNNER_REQUEST_INVALID",
                "marker parent must already exist",
            )
        })?;
    Ok(parent.join(file_name))
}

fn prototype_intent(
    executable: &Path,
    working_directory: &Path,
    marker: &Path,
    request_id: String,
) -> SubmittedIntent {
    SubmittedIntent {
        requesting_principal: "prototype.requester".into(),
        executing_principal: "prototype.executor".into(),
        action: "shell.exec".into(),
        intent_class: "prototype_marker".into(),
        target: executable.to_string_lossy().into_owned(),
        arguments: Value::Object(vec![
            (
                "argv".into(),
                Value::Array(vec![Value::String(marker.to_string_lossy().into_owned())]),
            ),
            (
                "cwd".into(),
                Value::String(working_directory.to_string_lossy().into_owned()),
            ),
            ("env".into(), Value::Object(vec![])),
            ("timeout_ms".into(), Value::Int(500)),
        ]),
        environment: "prototype".into(),
        tenant: "local".into(),
        declared_risk: Risk::High,
        data_classes: vec![],
        requested_capability: "shell.command".into(),
        resource_scope: vec![executable.to_string_lossy().into_owned()],
        payload_hash: None,
        artifact_hash: None,
        adapter: Adapter {
            id: "adapter.shell".into(),
            version: "1.0.0".into(),
        },
        request_id,
        retry_of_receipt_id: None,
    }
}

fn authorized_action(
    intent: &SubmittedIntent,
    issued: &tlpx::IssuedAuthorization,
) -> AuthorizedAction {
    AuthorizedAction {
        requesting_principal: intent.requesting_principal.clone(),
        executing_principal: intent.executing_principal.clone(),
        action: intent.action.clone(),
        target: intent.target.clone(),
        arguments: intent.arguments.clone(),
        environment: intent.environment.clone(),
        tenant: intent.tenant.clone(),
        derived_risk: Risk::High,
        effective_risk: Risk::High,
        risk_reasons: vec!["prototype_protected_command".into()],
        risk_source: "policy:tlpx-run-prototype@1.0.0".into(),
        data_classes: intent.data_classes.clone(),
        capability: issued.capability.clone(),
        resource_scope: issued.resource_scope.clone(),
        payload_hash: intent.payload_hash.clone(),
        artifact_hash: intent.artifact_hash.clone(),
        policy_bundle_hash: issued.policy_bundle_hash.clone(),
        adapter: intent.adapter.clone(),
    }
}

fn prototype_authority_config(
    executable: &Path,
    adapter_binary_hash: &str,
) -> tlpx::Result<AuthorityConfig> {
    let target = executable.to_string_lossy().into_owned();
    let template = AuthorizationTemplate {
        derived_risk: Risk::High,
        capability: "shell.command".into(),
        resource_scope: vec![target],
        risk_reasons: vec!["prototype_protected_command".into()],
        risk_source: "policy:tlpx-run-prototype@1.0.0".into(),
    };
    let policy = PolicyBundle {
        rules: vec![PolicyRule {
            id: "prototype-touch".into(),
            action: "shell.exec".into(),
            effect: PolicyEffect::allow("POLICY_ALLOW", template),
        }],
        default: PolicyEffect::deny("POLICY_DENY"),
    };
    let content_hash = exact_match_policy_content_hash(&policy)?;
    Ok(AuthorityConfig {
        policy: PolicyCatalog::new(vec![ConfiguredPolicyBundle {
            manifest: PolicyBundleManifest {
                policy_bundle_id: "tlpx-run-prototype".into(),
                policy_bundle_version: "1.0.0".into(),
                issuer: PolicyIssuer {
                    id: "prototype.local".into(),
                    kind: PolicyIssuerType::Human,
                },
                content_type: EXACT_MATCH_POLICY_CONTENT_TYPE.into(),
                content_hash,
                activated_at: "2020-01-01T00:00:00.000Z".into(),
                retired_at: None,
                environment: "prototype".into(),
                tenant: "local".into(),
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
                id: "prototype.requester".into(),
                active: true,
                allowed_actions: vec!["shell.exec".into()],
            },
            Principal {
                id: "prototype.executor".into(),
                active: true,
                allowed_actions: vec!["shell.exec".into()],
            },
        ])?,
        capabilities: CapabilityRegistry::new(vec![(
            "shell.command".into(),
            vec!["adapter.shell".into()],
        )])?,
        adapters: AdapterRegistry::new(vec![AdapterContract {
            adapter_id: "adapter.shell".into(),
            adapter_version: "1.0.0".into(),
            authenticated_principal: "prototype.adapter".into(),
            authenticated_authority: "prototype.authority".into(),
            binary_hash: adapter_binary_hash.into(),
            capabilities: vec!["shell.command".into()],
            actions: vec!["shell.exec".into()],
            material_fields: ADAPTER_MATERIAL_FIELDS
                .iter()
                .map(|field| (*field).into())
                .collect(),
        }])?,
        evidence: EvidenceConfig {
            evaluator_id: "prototype.authority".into(),
            router_id: "prototype.switchboard".into(),
            requester_type: PartyType::Machine,
            // Prototype only: fixed role-separated bytes are public and insecure.
            keys: KeyRing::active_profile([
                ("insecure-demo-audit-v1".into(), vec![0x73; 32]),
                ("insecure-demo-authorization-v1".into(), vec![0x74; 32]),
                ("insecure-demo-service-v1".into(), vec![0x75; 32]),
                ("insecure-demo-operator-v1".into(), vec![0x76; 32]),
                ("insecure-demo-tenant-v1".into(), vec![0x77; 32]),
            ])?,
        },
        approval_window_ms: 60_000,
        claim_window_ms: 5_000,
        execution_lease_ms: 5_000,
    })
}

fn identity(principal_id: &str, role: LocalRole) -> tlpx::Result<AuthenticatedIdentity> {
    let authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: principal_id.into(),
        party_type: PartyType::Machine,
        roles: vec![role],
        approval_routes: vec![],
    }])?;
    let (server, client) = UnixStream::pair()
        .map_err(|error| tlpx::Error::authority(format!("local socket pair: {error}")))?;
    let identity = authenticator.authenticate_stream(&server)?;
    drop(client);
    Ok(identity)
}

fn adapter_session() -> tlpx::Result<AuthenticatedAdapterSession> {
    let authority_authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: "prototype.adapter".into(),
        party_type: PartyType::Machine,
        roles: vec![LocalRole::Adapter],
        approval_routes: vec![],
    }])?;
    let adapter_authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: nix::unistd::Uid::effective().as_raw(),
        gid: nix::unistd::Gid::effective().as_raw(),
        principal_id: "prototype.authority".into(),
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
