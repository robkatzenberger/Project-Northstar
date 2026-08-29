use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct Fixture {
    root: PathBuf,
    config: PathBuf,
    socket: PathBuf,
    marker: PathBuf,
}

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn fixture(max_connections: usize, claim_window_ms: i64, claim_delay_ms: u64) -> Fixture {
    let root = PathBuf::from("/tmp").join(format!(
        "nsp-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let state = root.join("state");
    let endpoint = root.join("endpoint");
    let protected = root.join("protected");
    for directory in [&root, &state, &endpoint, &protected] {
        fs::create_dir_all(directory).unwrap();
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let key = state.join("seal.key");
    let mut role_keys = Vec::with_capacity(64);
    for byte in [0x39, 0x3a] {
        role_keys.extend_from_slice(&[byte; 32]);
    }
    fs::write(&key, role_keys).unwrap();
    fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
    let config = state.join("pep.conf");
    let socket = endpoint.join("pep.sock");
    let marker = protected.join("only-marker");
    let database = state.join("authority.sqlite");
    fs::write(
        &config,
        format!(
            "database={}\nsocket={}\nrole_keys={}\nprotected_marker={}\nagent_uid={}\nagent_gid={}\nmax_connections={}\nclaim_window_ms={}\nclaim_delay_ms={}\n",
            database.display(),
            socket.display(),
            key.display(),
            marker.display(),
            nix::unistd::Uid::effective().as_raw(),
            nix::unistd::Gid::effective().as_raw(),
            max_connections,
            claim_window_ms,
            claim_delay_ms
        ),
    )
    .unwrap();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    Fixture {
        root,
        config,
        socket,
        marker,
    }
}

fn spawn_server(fixture: &Fixture) -> Child {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tlpx-run"))
        .arg("serve")
        .arg(&fixture.config)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    for _ in 0..200 {
        if fixture.socket.exists() {
            return child;
        }
        if let Some(status) = child.try_wait().unwrap() {
            let mut stderr = String::new();
            std::io::Read::read_to_string(child.stderr.as_mut().unwrap(), &mut stderr).unwrap();
            panic!("PEP exited before binding ({status}): {stderr}");
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("PEP socket did not appear");
}

fn request(socket: &Path, line: &str) -> String {
    let mut stream = UnixStream::connect(socket).unwrap();
    writeln!(stream, "{line}").unwrap();
    let mut response = String::new();
    BufReader::new(stream).read_line(&mut response).unwrap();
    response.trim().into()
}

#[test]
fn bounded_service_denies_mutation_replay_and_expired_claim() {
    let active = fixture(5, 5_000, 0);
    let mut server = spawn_server(&active);

    assert!(request(&active.socket, "EXECUTE\tdenied-1\tDELETE_MARKER")
        .starts_with("DENY SWITCHBOARD_ACTION_DENIED "));
    assert_eq!(
        request(
            &active.socket,
            "EXECUTE\tforged-1\tCREATE_MARKER\tprincipal=restricted.pep"
        ),
        "DENY PEP_REQUEST_INVALID"
    );
    assert!(
        request(&active.socket, "EXECUTE\tallowed-1\tCREATE_MARKER").starts_with("OK COMPLETED ")
    );
    assert!(active.marker.is_file());
    assert_eq!(
        request(&active.socket, "EXECUTE\tallowed-1\tCREATE_MARKER"),
        "DENY PEP_PROTECTED_TARGET_EXISTS"
    );
    assert_eq!(
        request(&active.socket, "EXECUTE\tmutated-2\tCREATE_MARKER"),
        "DENY PEP_PROTECTED_TARGET_EXISTS"
    );
    assert!(server.wait().unwrap().success());
    assert!(!active.socket.exists());

    let verified = Command::new(env!("CARGO_BIN_EXE_tlpx-run"))
        .arg("verify")
        .arg(&active.config)
        .output()
        .unwrap();
    assert!(verified.status.success(), "{:?}", verified.stderr);
    let evidence = String::from_utf8(verified.stdout).unwrap();
    assert_eq!(evidence.lines().count(), 5);
    assert!(evidence.contains("\"decision\":\"DENY\""));
    assert!(evidence.contains("\"state\":\"COMPLETED\""));
    let evidence_path = active.root.join("evidence.jsonl");
    fs::write(&evidence_path, evidence).unwrap();
    let javascript_validator = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../javascript/scripts/validate-pep-evidence.mjs");
    let schema_validation = Command::new("node")
        .arg(javascript_validator)
        .arg(evidence_path)
        .output()
        .unwrap();
    assert!(
        schema_validation.status.success(),
        "{}{}",
        String::from_utf8_lossy(&schema_validation.stdout),
        String::from_utf8_lossy(&schema_validation.stderr)
    );

    let expired = fixture(1, 10, 40);
    let mut server = spawn_server(&expired);
    assert_eq!(
        request(&expired.socket, "EXECUTE\texpired-1\tCREATE_MARKER"),
        "DENY AUTHORIZATION_EXPIRED"
    );
    assert!(server.wait().unwrap().success());
    assert!(!expired.marker.exists());
}

#[test]
fn private_configuration_permissions_fail_closed() {
    let fixture = fixture(1, 5_000, 0);
    fs::set_permissions(&fixture.config, fs::Permissions::from_mode(0o644)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_tlpx-run"))
        .arg("serve")
        .arg(&fixture.config)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("PEP_CONFIG_INVALID"));
    assert!(!fixture.marker.exists());
}
