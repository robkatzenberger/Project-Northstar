//! Bounded slice 3.8 cooperative shell-command runner prototype.
//!
//! Commands are executed directly with an argv vector: this module adds no
//! shell interpolation, inherits no environment, and records no output content
//! in audit evidence. The executable allowlist is still trusted configuration;
//! an activated interpreter can interpret its own arguments.

use crate::adapter::AuthenticatedAdapterSession;
use crate::authority::{
    Authority, ExecutionReceipt, ExecutionResultEvidence, ExecutionState, IssuedAuthorization,
};
use crate::error::{Error, Result};
use crate::hash::assert_hash_string;
use crate::jcs::Value;
use crate::local_auth::AuthenticatedIdentity;
use crate::types::{AuthorizedAction, ExecutedAction};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const SHELL_ACTION: &str = "shell.exec";
const MAX_ARGUMENTS: usize = 64;
const MAX_ARGUMENT_BYTES: usize = 4_096;
const MAX_ENVIRONMENT_ENTRIES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellExecutable {
    canonical_path: PathBuf,
    binary_hash: String,
}

impl ShellExecutable {
    pub fn pinned(path: impl AsRef<Path>, binary_hash: impl Into<String>) -> Result<Self> {
        let canonical_path = canonical_existing_file(path.as_ref(), "executable")?;
        reject_shell_interpreter(&canonical_path)?;
        let binary_hash = binary_hash.into();
        assert_hash_string(&binary_hash).map_err(|_| {
            Error::coded(
                "SHELL_RUNNER_CONFIG_INVALID",
                "protected executable digest must be canonical sha256",
            )
        })?;
        open_verified_executable(&canonical_path, &binary_hash).map_err(|_| {
            runner_config_error(
                "protected executable provenance or digest does not match the startup file",
            )
        })?;
        Ok(Self {
            canonical_path,
            binary_hash,
        })
    }

    pub fn path(&self) -> &Path {
        &self.canonical_path
    }

    pub fn binary_hash(&self) -> &str {
        &self.binary_hash
    }
}

#[derive(Debug, Clone)]
pub struct CooperativeShellConfig {
    executables: BTreeMap<PathBuf, ShellExecutable>,
    working_directories: BTreeSet<PathBuf>,
    allowed_environment: BTreeSet<String>,
    max_output_bytes: usize,
    max_duration_ms: i64,
}

impl CooperativeShellConfig {
    pub fn new(
        executables: Vec<ShellExecutable>,
        working_directories: Vec<PathBuf>,
        allowed_environment: Vec<String>,
        max_output_bytes: usize,
        max_duration_ms: i64,
    ) -> Result<Self> {
        if executables.is_empty() || working_directories.is_empty() {
            return Err(runner_config_error(
                "executable and working-directory allowlists must not be empty",
            ));
        }
        if max_output_bytes == 0 || max_output_bytes > 1_048_576 {
            return Err(runner_config_error(
                "max output must be within 1..=1048576 bytes per stream",
            ));
        }
        if max_duration_ms <= 0 || max_duration_ms > 3_600_000 {
            return Err(runner_config_error(
                "max duration must be within 1..=3600000 ms",
            ));
        }
        let mut by_path = BTreeMap::new();
        for executable in executables {
            if by_path
                .insert(executable.canonical_path.clone(), executable)
                .is_some()
            {
                return Err(runner_config_error("duplicate protected executable"));
            }
        }
        let mut directories = BTreeSet::new();
        for directory in working_directories {
            let canonical = directory.canonicalize().map_err(|_| {
                runner_config_error(
                    "configured working directory must exist and be canonicalizable",
                )
            })?;
            if !canonical.is_dir() || !canonical.is_absolute() || !directories.insert(canonical) {
                return Err(runner_config_error(
                    "working directories must be unique absolute directories",
                ));
            }
        }
        let environment: BTreeSet<_> = allowed_environment.iter().cloned().collect();
        if environment.len() != allowed_environment.len()
            || environment
                .iter()
                .any(|name| name.is_empty() || name.contains('=') || name.contains('\0'))
        {
            return Err(runner_config_error(
                "environment allowlist names must be unique and valid",
            ));
        }
        Ok(Self {
            executables: by_path,
            working_directories: directories,
            allowed_environment: environment,
            max_output_bytes,
            max_duration_ms,
        })
    }
}

pub struct CooperativeShellRunner<'a> {
    authority: &'a Authority,
    config: CooperativeShellConfig,
}

#[derive(Debug)]
pub struct CooperativeShellRequest<'a> {
    pub authorization: &'a IssuedAuthorization,
    pub authorized_action: &'a AuthorizedAction,
    pub executor: &'a AuthenticatedIdentity,
    pub adapter: &'a AuthenticatedAdapterSession,
    pub adapter_binary_hash: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooperativeShellOutcome {
    pub receipt: ExecutionReceipt,
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub timed_out: bool,
}

impl<'a> CooperativeShellRunner<'a> {
    pub fn new(authority: &'a Authority, config: CooperativeShellConfig) -> Self {
        Self { authority, config }
    }

    pub fn execute(&self, request: CooperativeShellRequest<'_>) -> Result<CooperativeShellOutcome> {
        let plan = self.validate_request(&request)?;
        let started_at_ms = self.authority.transition_time_candidate_ms()?;
        let executed = request.authorized_action.binding();
        let executed = ExecutedAction {
            executing_principal: executed.executing_principal,
            action: executed.action,
            target: executed.target,
            arguments: executed.arguments,
            environment: executed.environment,
            tenant: executed.tenant,
            payload_hash: executed.payload_hash,
            artifact_hash: executed.artifact_hash,
            adapter: executed.adapter,
        };
        let claim = self.authority.claim_authenticated_at(
            &request.authorization.authorization_id,
            request.executor,
            &executed,
            started_at_ms,
        )?;
        let lease = self
            .authority
            .begin_execution_authenticated_at(
                &claim.claim_id,
                &request.authorization.idempotency_key,
                &executed,
                request.executor,
                request.adapter,
                request.adapter_binary_hash,
                started_at_ms,
            )?
            .into_started()?;

        if plan.executable.verify_current_path().is_err() {
            return self.finish_without_child(
                &lease.execution_id,
                request.executor,
                started_at_ms,
                "protected executable changed before spawn",
            );
        }

        let timer = Instant::now();
        let mut command = Command::new(plan.executable.path());
        command
            .args(&plan.argv)
            .current_dir(&plan.working_directory)
            .env_clear()
            .envs(&plan.environment)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(_) => {
                return self.finish_without_child(
                    &lease.execution_id,
                    request.executor,
                    started_at_ms,
                    "protected command could not be spawned",
                );
            }
        };
        let Some(stdout) = child.stdout.take() else {
            kill_process_group(&mut child);
            let _ = child.wait();
            return self.mark_unknown_after_process(
                &lease.execution_id,
                request.executor,
                started_at_ms,
                timer.elapsed(),
                "protected command stdout pipe is missing",
            );
        };
        let Some(stderr) = child.stderr.take() else {
            kill_process_group(&mut child);
            let _ = child.wait();
            return self.mark_unknown_after_process(
                &lease.execution_id,
                request.executor,
                started_at_ms,
                timer.elapsed(),
                "protected command stderr pipe is missing",
            );
        };
        let output_limit = self.config.max_output_bytes;
        let stdout_reader = thread::spawn(move || drain_bounded(stdout, output_limit));
        let stderr_reader = thread::spawn(move || drain_bounded(stderr, output_limit));

        let deadline = Duration::from_millis(plan.timeout_ms as u64);
        let mut timed_out = false;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    // The direct child may have left descendants holding the
                    // output pipes. The activated command owns its process
                    // group; close that whole group before joining readers.
                    kill_process_group(&mut child);
                    break status;
                }
                Ok(None) if timer.elapsed() < deadline => {
                    thread::sleep(Duration::from_millis(5));
                }
                Ok(None) => {
                    timed_out = true;
                    kill_process_group(&mut child);
                    match child.wait() {
                        Ok(status) => break status,
                        Err(_) => {
                            return self.mark_unknown_after_process(
                                &lease.execution_id,
                                request.executor,
                                started_at_ms,
                                timer.elapsed(),
                                "timed-out protected command could not be reaped",
                            );
                        }
                    }
                }
                Err(_) => {
                    kill_process_group(&mut child);
                    let _ = child.wait();
                    return self.mark_unknown_after_process(
                        &lease.execution_id,
                        request.executor,
                        started_at_ms,
                        timer.elapsed(),
                        "protected command status could not be observed",
                    );
                }
            }
        };
        let (stdout, stdout_truncated, stdout_capture_failed) = match join_output(stdout_reader) {
            Ok((output, truncated)) => (output, truncated, false),
            Err(_) => (Vec::new(), true, true),
        };
        let (stderr, stderr_truncated, stderr_capture_failed) = match join_output(stderr_reader) {
            Ok((output, truncated)) => (output, truncated, false),
            Err(_) => (Vec::new(), true, true),
        };
        let state = if status.success() && !timed_out {
            ExecutionState::Completed
        } else {
            ExecutionState::Failed
        };
        let summary = result_summary(
            status,
            timed_out,
            stdout.len(),
            stderr.len(),
            stdout_capture_failed || stderr_capture_failed,
        );
        let ended_at_ms = monotonic_end(started_at_ms, timer.elapsed());
        let receipt = self.finish_or_mark_unknown(
            &lease.execution_id,
            request.executor,
            state,
            summary,
            ended_at_ms,
        )?;
        Ok(CooperativeShellOutcome {
            receipt,
            exit_code: status.code(),
            stdout,
            stderr,
            stdout_truncated,
            stderr_truncated,
            timed_out,
        })
    }

    fn validate_request(&self, request: &CooperativeShellRequest<'_>) -> Result<ShellPlan> {
        request
            .authorized_action
            .validate()
            .map_err(|_| pep_request("invalid Authorized Action"))?;
        let full_hash = request
            .authorized_action
            .authorized_action_hash()
            .map_err(|_| pep_request("Authorized Action hash could not be computed"))?;
        let binding_hash = request
            .authorized_action
            .binding_hash()
            .map_err(|_| pep_request("Action Binding hash could not be computed"))?;
        let issued = request.authorization;
        if full_hash != issued.authorized_action_hash
            || binding_hash != issued.action_binding_hash
            || request.authorized_action.requesting_principal != issued.requesting_principal
            || request.authorized_action.executing_principal != issued.executing_principal
            || request.authorized_action.action != issued.action
            || request.authorized_action.target != issued.target
            || request.authorized_action.environment != issued.environment
            || request.authorized_action.tenant != issued.tenant
            || request.authorized_action.capability != issued.capability
            || request.authorized_action.resource_scope != issued.resource_scope
            || request.authorized_action.policy_bundle_hash != issued.policy_bundle_hash
            || request.authorized_action.adapter.id != issued.adapter_id
            || request.authorized_action.adapter.version != issued.adapter_version
        {
            return Err(pep_request(
                "complete Authorized Action does not match the issuance",
            ));
        }
        if request.authorized_action.action != SHELL_ACTION {
            return Err(Error::coded(
                "SHELL_RUNNER_COMMAND_DENIED",
                "only shell.exec is supported",
            ));
        }
        let executable = PathBuf::from(&request.authorized_action.target);
        if !executable.is_absolute() {
            return Err(Error::coded(
                "SHELL_RUNNER_COMMAND_DENIED",
                "executable must be absolute",
            ));
        }
        let canonical = executable.canonicalize().map_err(|_| {
            Error::coded("SHELL_RUNNER_COMMAND_DENIED", "executable is unavailable")
        })?;
        let pinned = self.config.executables.get(&canonical).ok_or_else(|| {
            Error::coded("SHELL_RUNNER_COMMAND_DENIED", "executable is not activated")
        })?;
        let executable = open_verified_executable(&canonical, &pinned.binary_hash)?;
        let plan = parse_plan(&request.authorized_action.arguments)?;
        let working_directory = plan.working_directory.canonicalize().map_err(|_| {
            Error::coded(
                "SHELL_RUNNER_COMMAND_DENIED",
                "working directory is unavailable",
            )
        })?;
        if !self.config.working_directories.contains(&working_directory) {
            return Err(Error::coded(
                "SHELL_RUNNER_COMMAND_DENIED",
                "working directory is not activated",
            ));
        }
        if plan.timeout_ms > self.config.max_duration_ms
            || plan.timeout_ms >= issued.execution_lease_ms
        {
            return Err(Error::coded(
                "SHELL_RUNNER_COMMAND_DENIED",
                "requested duration exceeds the PEP or execution lease",
            ));
        }
        if plan.environment.len() > MAX_ENVIRONMENT_ENTRIES
            || plan
                .environment
                .keys()
                .any(|name| !self.config.allowed_environment.contains(name))
        {
            return Err(Error::coded(
                "SHELL_RUNNER_COMMAND_DENIED",
                "environment entry is not activated",
            ));
        }
        Ok(ShellPlan {
            executable,
            argv: plan.argv,
            working_directory,
            environment: plan.environment,
            timeout_ms: plan.timeout_ms,
        })
    }

    fn finish_without_child(
        &self,
        execution_id: &str,
        executor: &AuthenticatedIdentity,
        started_at_ms: i64,
        summary: &str,
    ) -> Result<CooperativeShellOutcome> {
        let receipt = self.finish_or_mark_unknown(
            execution_id,
            executor,
            ExecutionState::Failed,
            summary.into(),
            started_at_ms,
        )?;
        Ok(CooperativeShellOutcome {
            receipt,
            exit_code: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            timed_out: false,
        })
    }

    fn finish_or_mark_unknown(
        &self,
        execution_id: &str,
        executor: &AuthenticatedIdentity,
        state: ExecutionState,
        summary: String,
        ended_at_ms: i64,
    ) -> Result<ExecutionReceipt> {
        match self.authority.finish_execution_authenticated_at(
            execution_id,
            executor,
            state,
            ExecutionResultEvidence {
                result_summary: Some(summary),
                result_hash: None,
                external_evidence_reference: None,
            },
            None,
            ended_at_ms,
        ) {
            Ok(receipt) => Ok(receipt),
            Err(error) => {
                let _ = self
                    .authority
                    .mark_execution_outcome_unknown_authenticated_at(
                        execution_id,
                        executor,
                        ended_at_ms,
                    );
                Err(error)
            }
        }
    }

    fn mark_unknown_after_process(
        &self,
        execution_id: &str,
        executor: &AuthenticatedIdentity,
        started_at_ms: i64,
        elapsed: Duration,
        message: &str,
    ) -> Result<CooperativeShellOutcome> {
        let observed_at_ms = monotonic_end(started_at_ms, elapsed);
        let _ = self
            .authority
            .mark_execution_outcome_unknown_authenticated_at(
                execution_id,
                executor,
                observed_at_ms,
            );
        Err(Error::coded("SHELL_RUNNER_IO_FAILED", message))
    }
}

#[derive(Debug)]
struct ParsedPlan {
    argv: Vec<String>,
    working_directory: PathBuf,
    environment: BTreeMap<String, String>,
    timeout_ms: i64,
}

#[derive(Debug)]
struct ShellPlan {
    // Keeps the verified inode open through spawn and re-checks that the
    // protected pathname still resolves to it immediately before execution.
    executable: VerifiedExecutable,
    argv: Vec<String>,
    working_directory: PathBuf,
    environment: BTreeMap<String, String>,
    timeout_ms: i64,
}

fn parse_plan(arguments: &Value) -> Result<ParsedPlan> {
    let Value::Object(fields) = arguments else {
        return Err(pep_request("shell arguments must be an object"));
    };
    let names: BTreeSet<_> = fields.iter().map(|(name, _)| name.as_str()).collect();
    if names != BTreeSet::from(["argv", "cwd", "env", "timeout_ms"]) {
        return Err(pep_request(
            "shell arguments must contain exactly argv/cwd/env/timeout_ms",
        ));
    }
    let get = |name: &str| {
        fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
            .ok_or_else(|| pep_request("shell argument field is missing"))
    };
    let Value::Array(argv_values) = get("argv")? else {
        return Err(pep_request("argv must be an array"));
    };
    if argv_values.len() > MAX_ARGUMENTS {
        return Err(pep_request("argv has too many entries"));
    }
    let mut argv = Vec::with_capacity(argv_values.len());
    for value in argv_values {
        let Value::String(value) = value else {
            return Err(pep_request("argv entries must be strings"));
        };
        validate_bounded_string(value, "argv entry")?;
        argv.push(value.clone());
    }
    let Value::String(cwd) = get("cwd")? else {
        return Err(pep_request("cwd must be a string"));
    };
    validate_bounded_string(cwd, "cwd")?;
    let Value::Object(environment_values) = get("env")? else {
        return Err(pep_request("env must be an object"));
    };
    let mut environment = BTreeMap::new();
    for (name, value) in environment_values {
        let Value::String(value) = value else {
            return Err(pep_request("environment values must be strings"));
        };
        validate_bounded_string(name, "environment name")?;
        validate_bounded_string(value, "environment value")?;
        if name.contains('=') || environment.insert(name.clone(), value.clone()).is_some() {
            return Err(pep_request("environment name is invalid or duplicated"));
        }
    }
    let Value::Int(timeout_ms) = get("timeout_ms")? else {
        return Err(pep_request("timeout_ms must be an integer"));
    };
    if *timeout_ms <= 0 {
        return Err(pep_request("timeout_ms must be positive"));
    }
    Ok(ParsedPlan {
        argv,
        working_directory: PathBuf::from(cwd),
        environment,
        timeout_ms: *timeout_ms,
    })
}

fn validate_bounded_string(value: &str, name: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_ARGUMENT_BYTES || value.contains('\0') {
        return Err(pep_request(format!(
            "{name} is empty, oversized, or contains NUL"
        )));
    }
    Ok(())
}

fn canonical_existing_file(path: &Path, name: &str) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .map_err(|_| runner_config_error(format!("{name} must exist and be canonicalizable")))?;
    if !canonical.is_absolute() || !canonical.is_file() {
        return Err(runner_config_error(format!(
            "{name} must be an absolute regular file"
        )));
    }
    Ok(canonical)
}

fn reject_shell_interpreter(path: &Path) -> Result<()> {
    let forbidden = ["sh", "bash", "dash", "zsh", "fish", "ksh", "csh", "tcsh"];
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if forbidden.contains(&name) {
        return Err(runner_config_error(
            "shell interpreters cannot be protected executables",
        ));
    }
    Ok(())
}

pub fn sha256_file(path: impl AsRef<Path>) -> Result<String> {
    let mut file = File::open(path.as_ref())
        .map_err(|_| Error::coded("SHELL_RUNNER_EXECUTABLE_CHANGED", "file cannot be opened"))?;
    sha256_reader(&mut file)
}

#[derive(Debug)]
struct VerifiedExecutable {
    path: PathBuf,
    _file: File,
    device: u64,
    inode: u64,
}

impl VerifiedExecutable {
    fn path(&self) -> &Path {
        &self.path
    }

    fn verify_current_path(&self) -> Result<()> {
        verify_executable_provenance(&self.path)?;
        let metadata = self.path.metadata().map_err(|_| executable_changed())?;
        if metadata.dev() != self.device || metadata.ino() != self.inode {
            return Err(executable_changed());
        }
        Ok(())
    }
}

fn open_verified_executable(path: &Path, expected_hash: &str) -> Result<VerifiedExecutable> {
    verify_executable_provenance(path)?;
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| {
            Error::coded(
                "SHELL_RUNNER_EXECUTABLE_CHANGED",
                "protected executable cannot be opened without following a link",
            )
        })?;
    let metadata = file.metadata().map_err(|_| executable_changed())?;
    if !metadata.is_file() {
        return Err(Error::coded(
            "SHELL_RUNNER_EXECUTABLE_CHANGED",
            "protected executable is not a regular file",
        ));
    }
    if sha256_reader(&mut file)? != expected_hash {
        return Err(Error::coded(
            "SHELL_RUNNER_EXECUTABLE_CHANGED",
            "protected executable descriptor digest does not match the activated digest",
        ));
    }
    let current = path.metadata().map_err(|_| executable_changed())?;
    if current.dev() != metadata.dev() || current.ino() != metadata.ino() {
        return Err(executable_changed());
    }
    Ok(VerifiedExecutable {
        path: path.to_path_buf(),
        _file: file,
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

fn verify_executable_provenance(path: &Path) -> Result<()> {
    let effective_uid = nix::unistd::Uid::effective().as_raw();
    let executable = path.metadata().map_err(|_| executable_changed())?;
    if !executable.is_file()
        || (executable.uid() != 0 && executable.uid() != effective_uid)
        || executable.mode() & 0o022 != 0
    {
        return Err(Error::coded(
            "SHELL_RUNNER_EXECUTABLE_CHANGED",
            "protected executable must be root- or PEP-owned and not group/world writable",
        ));
    }
    let mut child_uid = executable.uid();
    let mut ancestor = path.parent();
    while let Some(directory) = ancestor {
        let metadata = directory.metadata().map_err(|_| executable_changed())?;
        if !metadata.is_dir() {
            return Err(executable_changed());
        }
        if metadata.mode() & 0o022 != 0 {
            let sticky = metadata.mode() & 0o1000 != 0;
            let trusted_owner = metadata.uid() == 0 || metadata.uid() == effective_uid;
            let trusted_child = child_uid == 0 || child_uid == effective_uid;
            if !sticky || !trusted_owner || !trusted_child {
                return Err(Error::coded(
                    "SHELL_RUNNER_EXECUTABLE_CHANGED",
                    "protected executable has an untrusted writable directory ancestor",
                ));
            }
        }
        child_uid = metadata.uid();
        ancestor = directory.parent();
    }
    Ok(())
}

fn executable_changed() -> Error {
    Error::coded(
        "SHELL_RUNNER_EXECUTABLE_CHANGED",
        "protected executable identity or provenance changed",
    )
}

fn sha256_reader(file: &mut File) -> Result<String> {
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 16_384];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| Error::coded("SHELL_RUNNER_EXECUTABLE_CHANGED", "file cannot be read"))?;
        if read == 0 {
            break;
        }
        digest
            .write_all(&buffer[..read])
            .map_err(|_| Error::coded("SHELL_RUNNER_EXECUTABLE_CHANGED", "digest update failed"))?;
    }
    Ok(format!("sha256:{:x}", digest.finalize()))
}

fn drain_bounded<R: Read>(mut reader: R, limit: usize) -> std::io::Result<(Vec<u8>, bool)> {
    let mut captured = Vec::with_capacity(limit.min(16_384));
    let mut truncated = false;
    let mut buffer = [0_u8; 8_192];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(captured.len());
        let retain = remaining.min(read);
        captured.extend_from_slice(&buffer[..retain]);
        truncated |= retain < read;
    }
    Ok((captured, truncated))
}

fn kill_process_group(child: &mut Child) {
    let process_group = nix::unistd::Pid::from_raw(child.id() as i32);
    if nix::sys::signal::killpg(process_group, nix::sys::signal::Signal::SIGKILL).is_err() {
        let _ = child.kill();
    }
}

fn join_output(
    handle: thread::JoinHandle<std::io::Result<(Vec<u8>, bool)>>,
) -> Result<(Vec<u8>, bool)> {
    handle
        .join()
        .map_err(|_| Error::coded("SHELL_RUNNER_IO_FAILED", "output reader panicked"))?
        .map_err(|_| Error::coded("SHELL_RUNNER_IO_FAILED", "output could not be drained"))
}

fn result_summary(
    status: ExitStatus,
    timed_out: bool,
    stdout_bytes: usize,
    stderr_bytes: usize,
    capture_failed: bool,
) -> String {
    let capture = if capture_failed {
        "bounded output capture failed"
    } else {
        "output withheld"
    };
    if timed_out {
        format!(
            "protected command exceeded its duration; {capture} (stdout {stdout_bytes} bytes, stderr {stderr_bytes} bytes)"
        )
    } else {
        format!(
            "protected command exited with code {}; {capture} (stdout {stdout_bytes} bytes, stderr {stderr_bytes} bytes)",
            status.code().map_or_else(|| "signal".into(), |code| code.to_string())
        )
    }
}

fn monotonic_end(started_at_ms: i64, elapsed: Duration) -> i64 {
    let elapsed_ms = i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX);
    started_at_ms.saturating_add(elapsed_ms)
}

fn runner_config_error(message: impl Into<String>) -> Error {
    Error::coded("SHELL_RUNNER_CONFIG_INVALID", message)
}

fn pep_request(message: impl Into<String>) -> Error {
    Error::coded("SHELL_RUNNER_REQUEST_INVALID", message)
}

#[cfg(test)]
mod executable_provenance_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn private_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("tlpx-{label}-{nonce}"));
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    #[test]
    fn verified_executable_rejects_path_replacement_before_spawn() {
        let directory = private_directory("executable-replacement");
        let executable = directory.join("command");
        let displaced = directory.join("displaced");
        std::fs::write(&executable, b"original bytes").unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o500)).unwrap();
        let hash = sha256_file(&executable).unwrap();
        let verified = open_verified_executable(&executable, &hash).unwrap();

        std::fs::rename(&executable, &displaced).unwrap();
        std::fs::write(&executable, b"replacement bytes").unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o500)).unwrap();

        let error = verified.verify_current_path().unwrap_err();
        assert_eq!(error.code(), "SHELL_RUNNER_EXECUTABLE_CHANGED");
        std::fs::remove_file(&executable).unwrap();
        std::fs::remove_file(&displaced).unwrap();
        std::fs::remove_dir(&directory).unwrap();
    }

    #[test]
    fn activated_executable_rejects_group_writable_provenance() {
        let directory = private_directory("executable-permissions");
        let executable = directory.join("command");
        std::fs::write(&executable, b"bytes").unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o520)).unwrap();
        let hash = sha256_file(&executable).unwrap();

        let error = ShellExecutable::pinned(&executable, hash).unwrap_err();
        assert_eq!(error.code(), "SHELL_RUNNER_CONFIG_INVALID");
        std::fs::remove_file(&executable).unwrap();
        std::fs::remove_dir(&directory).unwrap();
    }
}
