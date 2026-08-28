//! Slice 3.9 restricted-agent local PEP service.
//!
//! The service exposes one Unix-socket operation and derives the caller from
//! kernel peer credentials. Its only protected capability is creating one
//! configured marker with `/usr/bin/touch`; callers cannot supply a path,
//! executable, argv, environment, or principal.

use crate::adapter::{AdapterContract, AdapterRegistry, AuthenticatedAdapterSession};
use crate::authority::{Authority, AuthorityConfig, IssuedAuthorization};
use crate::error::{Error, Result};
use crate::evidence::{EvidenceConfig, EvidenceReconciliation, PartyType, SealedEvidence};
use crate::jcs::Value;
use crate::keys::KeyRing;
use crate::local_auth::{LocalAuthenticator, LocalPrincipalMapping, LocalRole};
use crate::policy::{
    AuthorizationTemplate, CapabilityRegistry, Decision, PolicyBundle, PolicyEffect, PolicyRule,
    Principal, Switchboard,
};
use crate::policy_manifest::{
    exact_match_policy_content_hash, ConfiguredPolicyBundle, PolicyBundleManifest, PolicyCatalog,
    PolicyIssuer, PolicyIssuerType, EXACT_MATCH_POLICY_CONTENT_TYPE, POLICY_PRECEDENCE,
};
use crate::shell_runner::{
    sha256_file, CooperativeShellConfig, CooperativeShellRequest, CooperativeShellRunner,
    ShellExecutable,
};
use crate::types::{Adapter, AuthorizedAction, Risk, SubmittedIntent};
use crate::ADAPTER_MATERIAL_FIELDS;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

const TOUCH_PATH: &str = "/usr/bin/touch";
const REQUEST_ACTION: &str = "shell.exec";
const REQUEST_OPERATION: &str = "CREATE_MARKER";
const DENIED_ACTION: &str = "pep.unsupported";
const ADAPTER_ID: &str = "adapter.restricted-marker";
const ADAPTER_VERSION: &str = "1.0.0";
const AGENT_PRINCIPAL: &str = "restricted.agent";
const ADAPTER_PRINCIPAL: &str = "restricted.pep.adapter";
const AUTHORITY_PRINCIPAL: &str = "restricted.pep.authority";
const ENVIRONMENT: &str = "restricted_pep";
const TENANT: &str = "local_acceptance";
const MAX_PROTOCOL_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestrictedPepConfig {
    pub database: PathBuf,
    pub socket: PathBuf,
    pub role_keys: PathBuf,
    pub protected_marker: PathBuf,
    pub agent_uid: u32,
    pub agent_gid: u32,
    pub max_connections: usize,
    pub claim_window_ms: i64,
    pub claim_delay_ms: u64,
}

impl RestrictedPepConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        validate_owned_file(path, "PEP configuration", true)?;
        let text = fs::read_to_string(path)
            .map_err(|_| pep_config("PEP configuration could not be read"))?;
        let mut values = BTreeMap::new();
        for (index, line) in text.lines().enumerate() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((name, value)) = line.split_once('=') else {
                return Err(pep_config(format!(
                    "PEP configuration line {} is malformed",
                    index + 1
                )));
            };
            if name.is_empty() || value.is_empty() || values.insert(name, value).is_some() {
                return Err(pep_config(
                    "PEP configuration contains an empty or duplicate field",
                ));
            }
        }
        let expected = BTreeSet::from([
            "agent_gid",
            "agent_uid",
            "claim_delay_ms",
            "claim_window_ms",
            "database",
            "max_connections",
            "protected_marker",
            "role_keys",
            "socket",
        ]);
        if values.keys().copied().collect::<BTreeSet<_>>() != expected {
            return Err(pep_config(
                "PEP configuration fields are missing, unknown, or misspelled",
            ));
        }
        let config = Self {
            database: absolute_path(value(&values, "database")?, "database")?,
            socket: absolute_path(value(&values, "socket")?, "socket")?,
            role_keys: absolute_path(value(&values, "role_keys")?, "role_keys")?,
            protected_marker: absolute_path(
                value(&values, "protected_marker")?,
                "protected_marker",
            )?,
            agent_uid: parse_number(value(&values, "agent_uid")?, "agent_uid")?,
            agent_gid: parse_number(value(&values, "agent_gid")?, "agent_gid")?,
            max_connections: parse_number(value(&values, "max_connections")?, "max_connections")?,
            claim_window_ms: parse_number(value(&values, "claim_window_ms")?, "claim_window_ms")?,
            claim_delay_ms: parse_number(value(&values, "claim_delay_ms")?, "claim_delay_ms")?,
        };
        if config.max_connections == 0 || config.max_connections > 64 {
            return Err(pep_config("max_connections must be within 1..=64"));
        }
        if config.claim_window_ms <= 0 || config.claim_window_ms > 60_000 {
            return Err(pep_config("claim_window_ms must be within 1..=60000"));
        }
        if config.claim_delay_ms > 60_000 {
            return Err(pep_config("claim_delay_ms must be within 0..=60000"));
        }
        config.validate_paths()?;
        Ok(config)
    }

    fn validate_paths(&self) -> Result<()> {
        validate_owned_directory(parent(&self.database, "database")?, "database parent")?;
        validate_owned_directory(parent(&self.socket, "socket")?, "socket parent")?;
        validate_owned_directory(
            parent(&self.protected_marker, "protected marker")?,
            "protected marker parent",
        )?;
        validate_owned_file(&self.role_keys, "PEP role-key bundle", true)?;
        if self.socket.exists() {
            return Err(pep_config(
                "PEP socket path already exists; refusing to replace an endpoint",
            ));
        }
        if let Ok(metadata) = fs::symlink_metadata(&self.protected_marker) {
            if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
                return Err(pep_config(
                    "existing protected marker must be a non-symlink regular file",
                ));
            }
        }
        if self.database.exists() {
            validate_owned_file(&self.database, "authority database", false)?;
        }
        validate_current_executable()?;
        Ok(())
    }
}

pub fn serve_restricted_pep(config_path: impl AsRef<Path>) -> Result<()> {
    let config = RestrictedPepConfig::load(config_path)?;
    let executable = touch_executable()?;
    let adapter_binary_hash = current_binary_hash()?;
    let role_keys = read_role_keys(&config.role_keys)?;
    let authority = Authority::open(
        &config.database,
        authority_config(
            &executable,
            &adapter_binary_hash,
            role_keys,
            config.claim_window_ms,
        )?,
    )?;
    fs::set_permissions(&config.database, fs::Permissions::from_mode(0o600))
        .map_err(|_| pep_config("authority database permissions could not be restricted"))?;
    let runner = CooperativeShellRunner::new(
        &authority,
        CooperativeShellConfig::new(
            vec![ShellExecutable::pinned(
                &executable,
                sha256_file(&executable)?,
            )?],
            vec![parent(&config.protected_marker, "protected marker")?.to_path_buf()],
            vec![],
            4_096,
            1_000,
        )?,
    );
    let authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid: config.agent_uid,
        gid: config.agent_gid,
        principal_id: AGENT_PRINCIPAL.into(),
        party_type: PartyType::Machine,
        roles: vec![LocalRole::Requester, LocalRole::Executor],
        approval_routes: vec![],
    }])?;
    let adapter = internal_adapter_session()?;
    let listener = UnixListener::bind(&config.socket).map_err(|error| {
        Error::coded(
            "PEP_ENDPOINT_UNAVAILABLE",
            format!("PEP socket could not be bound: {error}"),
        )
    })?;
    let _socket_guard = SocketGuard(config.socket.clone());
    // Endpoint access is intentionally broad enough for an unprivileged
    // caller to connect. Authorization comes from exact kernel UID/GID peer
    // credentials, while the non-writable parent prevents endpoint replacement.
    fs::set_permissions(&config.socket, fs::Permissions::from_mode(0o666))
        .map_err(|_| pep_config("PEP socket permissions could not be restricted"))?;

    for stream in listener.incoming().take(config.max_connections) {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(_) => continue,
        };
        let response = handle_connection(
            &mut stream,
            &config,
            &authenticator,
            &adapter,
            &adapter_binary_hash,
            &authority,
            &runner,
            &executable,
        );
        let _ = write_response(&mut stream, &response);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_connection(
    stream: &mut UnixStream,
    config: &RestrictedPepConfig,
    authenticator: &LocalAuthenticator,
    adapter: &AuthenticatedAdapterSession,
    adapter_binary_hash: &str,
    authority: &Authority,
    runner: &CooperativeShellRunner<'_>,
    executable: &Path,
) -> Result<String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| pep_request("PEP request timeout could not be applied"))?;
    let identity = authenticator.authenticate_stream(stream)?;
    let request = read_request(stream)?;
    let action = if request.operation == REQUEST_OPERATION {
        if config.protected_marker.exists() {
            return Err(Error::coded(
                "PEP_PROTECTED_TARGET_EXISTS",
                "protected marker already exists and cannot be reopened",
            ));
        }
        REQUEST_ACTION
    } else {
        DENIED_ACTION
    };
    let intent = submitted_intent(
        identity.principal_id(),
        executable,
        &config.protected_marker,
        &request.request_id,
        action,
    );
    let outcome = authority.evaluate_authenticated(&identity, &intent)?;
    let Some(issued) = outcome.authorization.as_ref() else {
        return Ok(format!(
            "DENY {} {}",
            outcome.reason_code, outcome.receipt_id
        ));
    };
    if config.claim_delay_ms > 0 {
        thread::sleep(Duration::from_millis(config.claim_delay_ms));
    }
    let authorized = authorized_action(&intent, issued);
    let outcome = runner.execute(CooperativeShellRequest {
        authorization: issued,
        authorized_action: &authorized,
        executor: &identity,
        adapter,
        adapter_binary_hash,
    })?;
    Ok(format!(
        "OK {} {}",
        outcome.receipt.state.as_str(),
        outcome.receipt.receipt_id
    ))
}

pub fn request_restricted_pep(
    socket: impl AsRef<Path>,
    request_id: &str,
    operation: &str,
) -> Result<String> {
    validate_token(request_id, "request_id")?;
    validate_token(operation, "operation")?;
    raw_restricted_pep_request(socket, &format!("EXECUTE\t{request_id}\t{operation}"))
}

pub fn raw_restricted_pep_request(socket: impl AsRef<Path>, request_line: &str) -> Result<String> {
    if request_line.is_empty()
        || request_line.len() > MAX_PROTOCOL_BYTES
        || request_line.contains('\n')
        || request_line.contains('\r')
        || request_line.contains('\0')
    {
        return Err(pep_request("raw PEP request must be one bounded line"));
    }
    let mut stream = UnixStream::connect(socket.as_ref()).map_err(|error| {
        Error::coded(
            "PEP_ENDPOINT_UNAVAILABLE",
            format!("PEP socket is unavailable: {error}"),
        )
    })?;
    stream
        .write_all(format!("{request_line}\n").as_bytes())
        .map_err(|_| pep_request("PEP request could not be written"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| pep_request("PEP response timeout could not be applied"))?;
    let mut response = String::new();
    BufReader::new(stream)
        .take(4_096)
        .read_to_string(&mut response)
        .map_err(|_| pep_request("PEP response could not be read"))?;
    let response = response.trim_end_matches(['\n', '\r']);
    if response.is_empty() {
        return Err(pep_request("PEP returned an empty response"));
    }
    Ok(response.into())
}

pub fn verify_restricted_pep_evidence(
    config_path: impl AsRef<Path>,
) -> Result<(EvidenceReconciliation, Vec<SealedEvidence>)> {
    let config = RestrictedPepConfig::load(config_path)?;
    let executable = touch_executable()?;
    let adapter_binary_hash = current_binary_hash()?;
    let authority = Authority::open(
        &config.database,
        authority_config(
            &executable,
            &adapter_binary_hash,
            read_role_keys(&config.role_keys)?,
            config.claim_window_ms,
        )?,
    )?;
    let reconciliation = authority.reconcile_evidence()?;
    let evidence = authority.pending_evidence(1_000)?;
    if reconciliation.pending != i64::try_from(evidence.len()).unwrap_or(i64::MAX) {
        return Err(Error::coded(
            "PEP_EVIDENCE_INVALID",
            "pending evidence count does not match reconciled authority state",
        ));
    }
    Ok((reconciliation, evidence))
}

pub fn assert_replacement_bind_denied(socket: impl AsRef<Path>) -> Result<()> {
    let socket = socket.as_ref();
    match UnixListener::bind(socket) {
        Err(_) => Ok(()),
        Ok(listener) => {
            drop(listener);
            let _ = fs::remove_file(socket);
            Err(Error::coded(
                "PEP_REPLACEMENT_POSSIBLE",
                "caller could bind a replacement PEP endpoint",
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WireRequest {
    request_id: String,
    operation: String,
}

fn read_request(stream: &UnixStream) -> Result<WireRequest> {
    let mut line = String::new();
    let read = BufReader::new(stream)
        .take((MAX_PROTOCOL_BYTES + 2) as u64)
        .read_line(&mut line)
        .map_err(|_| pep_request("PEP request could not be read"))?;
    if read == 0 || read > MAX_PROTOCOL_BYTES || !line.ends_with('\n') {
        return Err(pep_request(
            "PEP request must be one bounded newline-terminated line",
        ));
    }
    let line = line.trim_end_matches(['\n', '\r']);
    let fields = line.split('\t').collect::<Vec<_>>();
    if fields.len() != 3 || fields[0] != "EXECUTE" {
        return Err(pep_request(
            "PEP request must contain exactly EXECUTE/request_id/operation",
        ));
    }
    validate_token(fields[1], "request_id")?;
    validate_token(fields[2], "operation")?;
    Ok(WireRequest {
        request_id: fields[1].into(),
        operation: fields[2].into(),
    })
}

fn validate_token(token: &str, name: &str) -> Result<()> {
    if token.is_empty()
        || token.len() > 128
        || !token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(pep_request(format!(
            "{name} must be a bounded ASCII identifier"
        )));
    }
    Ok(())
}

fn write_response(stream: &mut UnixStream, response: &Result<String>) -> std::io::Result<()> {
    match response {
        Ok(response) => writeln!(stream, "{response}"),
        Err(error) => writeln!(stream, "DENY {}", error.code()),
    }
}

fn submitted_intent(
    principal: &str,
    executable: &Path,
    marker: &Path,
    request_id: &str,
    action: &str,
) -> SubmittedIntent {
    let executable = executable.to_string_lossy().into_owned();
    SubmittedIntent {
        requesting_principal: principal.into(),
        executing_principal: principal.into(),
        action: action.into(),
        intent_class: "restricted_marker_creation".into(),
        target: executable.clone(),
        arguments: command_plan(marker),
        environment: ENVIRONMENT.into(),
        tenant: TENANT.into(),
        declared_risk: Risk::High,
        data_classes: vec![],
        requested_capability: "shell.command".into(),
        resource_scope: vec![executable],
        payload_hash: None,
        artifact_hash: None,
        adapter: Adapter {
            id: ADAPTER_ID.into(),
            version: ADAPTER_VERSION.into(),
        },
        request_id: request_id.into(),
        retry_of_receipt_id: None,
    }
}

fn command_plan(marker: &Path) -> Value {
    Value::Object(vec![
        (
            "argv".into(),
            Value::Array(vec![Value::String(marker.to_string_lossy().into_owned())]),
        ),
        (
            "cwd".into(),
            Value::String(
                marker
                    .parent()
                    .unwrap_or_else(|| Path::new("/"))
                    .to_string_lossy()
                    .into_owned(),
            ),
        ),
        ("env".into(), Value::Object(vec![])),
        ("timeout_ms".into(), Value::Int(500)),
    ])
}

fn authorized_action(intent: &SubmittedIntent, issued: &IssuedAuthorization) -> AuthorizedAction {
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
        risk_reasons: vec!["restricted_protected_marker".into()],
        risk_source: "policy:restricted-marker-pep@1.0.0".into(),
        data_classes: vec![],
        capability: issued.capability.clone(),
        resource_scope: issued.resource_scope.clone(),
        payload_hash: None,
        artifact_hash: None,
        policy_bundle_hash: issued.policy_bundle_hash.clone(),
        adapter: intent.adapter.clone(),
    }
}

fn authority_config(
    executable: &Path,
    adapter_binary_hash: &str,
    role_keys: KeyRing,
    claim_window_ms: i64,
) -> Result<AuthorityConfig> {
    let target = executable.to_string_lossy().into_owned();
    let template = AuthorizationTemplate {
        derived_risk: Risk::High,
        capability: "shell.command".into(),
        resource_scope: vec![target],
        risk_reasons: vec!["restricted_protected_marker".into()],
        risk_source: "policy:restricted-marker-pep@1.0.0".into(),
    };
    let policy = PolicyBundle {
        rules: vec![PolicyRule {
            id: "allow-restricted-marker".into(),
            action: REQUEST_ACTION.into(),
            effect: PolicyEffect::allow("POLICY_ALLOW", template),
        }],
        default: PolicyEffect::deny("POLICY_DENY"),
    };
    let content_hash = exact_match_policy_content_hash(&policy)?;
    Ok(AuthorityConfig {
        policy: PolicyCatalog::new(vec![ConfiguredPolicyBundle {
            manifest: PolicyBundleManifest {
                policy_bundle_id: "restricted-marker-pep".into(),
                policy_bundle_version: "1.0.0".into(),
                issuer: PolicyIssuer {
                    id: "security.platform".into(),
                    kind: PolicyIssuerType::Human,
                },
                content_type: EXACT_MATCH_POLICY_CONTENT_TYPE.into(),
                content_hash,
                activated_at: "2020-01-01T00:00:00.000Z".into(),
                retired_at: None,
                environment: ENVIRONMENT.into(),
                tenant: TENANT.into(),
                precedence: POLICY_PRECEDENCE
                    .iter()
                    .map(|field| (*field).into())
                    .collect(),
                default_decision: Decision::Deny,
                supersedes: None,
            },
            policy,
        }]),
        switchboard: Switchboard::new(vec![Principal {
            id: AGENT_PRINCIPAL.into(),
            active: true,
            allowed_actions: vec![REQUEST_ACTION.into()],
        }])?,
        capabilities: CapabilityRegistry::new(vec![(
            "shell.command".into(),
            vec![ADAPTER_ID.into()],
        )])?,
        adapters: AdapterRegistry::new(vec![AdapterContract {
            adapter_id: ADAPTER_ID.into(),
            adapter_version: ADAPTER_VERSION.into(),
            authenticated_principal: ADAPTER_PRINCIPAL.into(),
            authenticated_authority: AUTHORITY_PRINCIPAL.into(),
            binary_hash: adapter_binary_hash.into(),
            capabilities: vec!["shell.command".into()],
            actions: vec![REQUEST_ACTION.into()],
            material_fields: ADAPTER_MATERIAL_FIELDS
                .iter()
                .map(|field| (*field).into())
                .collect(),
        }])?,
        evidence: EvidenceConfig {
            evaluator_id: AUTHORITY_PRINCIPAL.into(),
            router_id: "restricted.pep.switchboard".into(),
            requester_type: PartyType::Machine,
            keys: role_keys,
        },
        approval_window_ms: 60_000,
        claim_window_ms,
        execution_lease_ms: 2_000,
    })
}

fn internal_adapter_session() -> Result<AuthenticatedAdapterSession> {
    let uid = nix::unistd::Uid::effective().as_raw();
    let gid = nix::unistd::Gid::effective().as_raw();
    let authority_authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid,
        gid,
        principal_id: ADAPTER_PRINCIPAL.into(),
        party_type: PartyType::Machine,
        roles: vec![LocalRole::Adapter],
        approval_routes: vec![],
    }])?;
    let adapter_authenticator = LocalAuthenticator::new(vec![LocalPrincipalMapping {
        uid,
        gid,
        principal_id: AUTHORITY_PRINCIPAL.into(),
        party_type: PartyType::Machine,
        roles: vec![LocalRole::Authority],
        approval_routes: vec![],
    }])?;
    let (authority_side, adapter_side) = UnixStream::pair()
        .map_err(|_| Error::authority("internal PEP adapter channel could not be created"))?;
    AuthenticatedAdapterSession::authenticate_local(
        &authority_authenticator,
        &adapter_authenticator,
        &authority_side,
        &adapter_side,
    )
}

fn touch_executable() -> Result<PathBuf> {
    Path::new(TOUCH_PATH)
        .canonicalize()
        .map_err(|_| pep_config("restricted PEP requires /usr/bin/touch"))
}

fn current_binary_hash() -> Result<String> {
    sha256_file(
        std::env::current_exe()
            .map_err(|_| pep_config("PEP executable path could not be determined"))?,
    )
}

fn validate_current_executable() -> Result<()> {
    let path = std::env::current_exe()
        .map_err(|_| pep_config("PEP executable path could not be determined"))?;
    let metadata =
        fs::metadata(path).map_err(|_| pep_config("PEP executable metadata could not be read"))?;
    if !metadata.is_file() || metadata.mode() & 0o022 != 0 {
        return Err(pep_config(
            "PEP executable must be a regular file not writable by group or others",
        ));
    }
    Ok(())
}

fn read_role_keys(path: &Path) -> Result<KeyRing> {
    let bytes = fs::read(path).map_err(|_| pep_config("PEP role-key bundle could not be read"))?;
    if bytes.len() != 160 {
        return Err(pep_config(
            "PEP role-key bundle must contain exactly five independent 32-byte keys",
        ));
    }
    KeyRing::active_profile([
        ("restricted-pep-audit-v1".into(), bytes[0..32].to_vec()),
        (
            "restricted-pep-authorization-v1".into(),
            bytes[32..64].to_vec(),
        ),
        ("restricted-pep-service-v1".into(), bytes[64..96].to_vec()),
        ("restricted-pep-operator-v1".into(), bytes[96..128].to_vec()),
        ("restricted-pep-tenant-v1".into(), bytes[128..160].to_vec()),
    ])
}

fn validate_owned_file(path: &Path, name: &str, private: bool) -> Result<()> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| pep_config(format!("{name} is unavailable")))?;
    let forbidden = if private { 0o077 } else { 0o022 };
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != nix::unistd::Uid::effective().as_raw()
        || metadata.mode() & forbidden != 0
    {
        return Err(pep_config(format!(
            "{name} must be an owned regular file with restricted permissions"
        )));
    }
    Ok(())
}

fn validate_owned_directory(path: &Path, name: &str) -> Result<()> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| pep_config(format!("{name} is unavailable")))?;
    if !metadata.file_type().is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != nix::unistd::Uid::effective().as_raw()
        || metadata.mode() & 0o022 != 0
    {
        return Err(pep_config(format!(
            "{name} must be an owned directory not writable by group or others"
        )));
    }
    Ok(())
}

fn value<'a>(values: &BTreeMap<&str, &'a str>, name: &str) -> Result<&'a str> {
    values
        .get(name)
        .copied()
        .ok_or_else(|| pep_config(format!("missing PEP configuration field {name}")))
}

fn absolute_path(value: &str, name: &str) -> Result<PathBuf> {
    let path = PathBuf::from(value);
    if !path.is_absolute() || value.contains('\0') {
        return Err(pep_config(format!("{name} must be an absolute path")));
    }
    Ok(path)
}

fn parse_number<T>(value: &str, name: &str) -> Result<T>
where
    T: std::str::FromStr,
{
    value
        .parse()
        .map_err(|_| pep_config(format!("{name} must be a decimal integer")))
}

fn parent<'a>(path: &'a Path, name: &str) -> Result<&'a Path> {
    path.parent()
        .filter(|parent| parent.is_absolute())
        .ok_or_else(|| pep_config(format!("{name} has no absolute parent")))
}

fn pep_config(message: impl Into<String>) -> Error {
    Error::coded("PEP_CONFIG_INVALID", message)
}

fn pep_request(message: impl Into<String>) -> Error {
    Error::coded("PEP_REQUEST_INVALID", message)
}

struct SocketGuard(PathBuf);

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_is_exact_and_bounded() {
        assert!(validate_token("request-1", "request_id").is_ok());
        assert!(validate_token("a/b", "request_id").is_err());
        assert!(validate_token(&"x".repeat(129), "request_id").is_err());
    }
}
