//! Durable append-only file export for the sealed local evidence outbox.
//!
//! A row is acknowledged in SQLite only after its canonical export envelope
//! has been appended and synced. On restart, a complete sink row that is ahead
//! of its database acknowledgement is verified and acknowledged without being
//! appended a second time.

use crate::authority::Authority;
use crate::error::{Error, Result};
use crate::evidence::SealedEvidence;
use crate::jcs::{canonicalize, parse, Canonical, Value};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

const EXPORT_FORMAT: &str = "tlpx.local-audit-export";
const EXPORT_FORMAT_VERSION: &str = "1";
const MAX_SINK_BYTES: u64 = 64 * 1024 * 1024;
const MAX_LINE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditExportSummary {
    pub recovered: usize,
    pub appended: usize,
    pub total_in_sink: usize,
    pub last_chain_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileAuditExporter {
    path: PathBuf,
}

impl FileAuditExporter {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Exports at most `limit` new rows after first recovering any complete,
    /// already-synced prefix left ahead of its database acknowledgement.
    pub fn export_pending_at(
        &self,
        authority: &Authority,
        limit: usize,
        exported_at_ms: i64,
    ) -> Result<AuditExportSummary> {
        if !(1..=1_000).contains(&limit) {
            return Err(export_error("audit export limit must be 1..=1000"));
        }
        if exported_at_ms < 0 {
            return Err(export_error("audit export timestamp must not be negative"));
        }

        let parent = protected_parent(&self.path)?;
        let _lock = ExportLock::acquire(&self.path)?;
        let rows = authority.evidence_snapshot()?;
        let existed = self.path.exists();
        let mut sink = secure_sink(&self.path)?;
        if !existed {
            File::open(&parent)
                .and_then(|directory| directory.sync_all())
                .map_err(|error| export_io("sync audit export directory", error))?;
        }

        let lines = read_lines(&mut sink)?;
        if lines.len() > rows.len() {
            return Err(export_error(
                "audit sink contains rows not present in the authority outbox",
            ));
        }

        let mut recovered = 0_usize;
        for (index, line) in lines.iter().enumerate() {
            let row = &rows[index];
            let expected = export_envelope(row)?;
            if line != expected.as_str() {
                return Err(export_error(
                    "audit sink is not an exact ordered prefix of the sealed outbox",
                ));
            }
            if row.exported_at_ms.is_none() {
                authority.mark_evidence_exported_at(
                    row.outbox_id,
                    &row.chain_hash,
                    exported_at_ms,
                )?;
                recovered += 1;
            }
        }

        if rows
            .iter()
            .skip(lines.len())
            .any(|row| row.exported_at_ms.is_some())
        {
            return Err(export_error(
                "authority acknowledges evidence missing from the audit sink",
            ));
        }

        sink.seek(SeekFrom::End(0))
            .map_err(|error| export_io("seek audit sink", error))?;
        let mut appended = 0_usize;
        for row in rows.iter().skip(lines.len()).take(limit) {
            let envelope = export_envelope(row)?;
            if envelope.as_str().len() > MAX_LINE_BYTES {
                return Err(export_error(
                    "audit export row exceeds the bounded line size",
                ));
            }
            sink.write_all(envelope.as_str().as_bytes())
                .and_then(|()| sink.write_all(b"\n"))
                .and_then(|()| sink.sync_all())
                .map_err(|error| export_io("append and sync audit evidence", error))?;
            authority.mark_evidence_exported_at(row.outbox_id, &row.chain_hash, exported_at_ms)?;
            appended += 1;
        }

        let total_in_sink = lines.len() + appended;
        let last_chain_hash = total_in_sink
            .checked_sub(1)
            .and_then(|index| rows.get(index))
            .map(|row| row.chain_hash.clone());
        Ok(AuditExportSummary {
            recovered,
            appended,
            total_in_sink,
            last_chain_hash,
        })
    }
}

struct ExportLock {
    path: PathBuf,
}

impl ExportLock {
    fn acquire(sink_path: &Path) -> Result<Self> {
        let file_name = sink_path
            .file_name()
            .ok_or_else(|| export_error("audit sink path must name a file"))?;
        let mut lock_name = file_name.to_os_string();
        lock_name.push(".lock");
        let path = sink_path.with_file_name(lock_name);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(nix::libc::O_NOFOLLOW)
            .open(&path)
            .map_err(|error| export_io("acquire exclusive audit export lock", error))?;
        validate_file(&file, "audit export lock")?;
        file.sync_all()
            .map_err(|error| export_io("sync audit export lock", error))?;
        Ok(Self { path })
    }
}

impl Drop for ExportLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn protected_parent(path: &Path) -> Result<PathBuf> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let metadata = fs::symlink_metadata(parent)
        .map_err(|error| export_io("inspect audit export directory", error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(export_error(
            "audit export parent must be a real directory, not a symlink",
        ));
    }
    if metadata.uid() != nix::unistd::Uid::effective().as_raw() {
        return Err(export_error(
            "audit export directory must be owned by the effective user",
        ));
    }
    if metadata.permissions().mode() & 0o022 != 0 {
        return Err(export_error(
            "audit export directory must not be writable by group or others",
        ));
    }
    fs::canonicalize(parent).map_err(|error| export_io("resolve audit export directory", error))
}

fn secure_sink(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .mode(0o600)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(path)
        .map_err(|error| export_io("open audit sink", error))?;
    validate_file(&file, "audit sink")?;
    Ok(file)
}

fn validate_file(file: &File, label: &str) -> Result<()> {
    let metadata = file
        .metadata()
        .map_err(|error| export_io(&format!("inspect {label}"), error))?;
    if !metadata.is_file() {
        return Err(export_error(format!("{label} must be a regular file")));
    }
    if metadata.uid() != nix::unistd::Uid::effective().as_raw() {
        return Err(export_error(format!(
            "{label} must be owned by the effective user"
        )));
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(export_error(format!(
            "{label} must not be accessible by group or others"
        )));
    }
    Ok(())
}

fn read_lines(file: &mut File) -> Result<Vec<String>> {
    let length = file
        .metadata()
        .map_err(|error| export_io("inspect audit sink length", error))?
        .len();
    if length > MAX_SINK_BYTES {
        return Err(export_error("audit sink exceeds the bounded local size"));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| export_io("rewind audit sink", error))?;
    let mut bytes = Vec::with_capacity(length as usize);
    file.read_to_end(&mut bytes)
        .map_err(|error| export_io("read audit sink", error))?;
    if !bytes.is_empty() && !bytes.ends_with(b"\n") {
        return Err(export_error(
            "audit sink ends with an incomplete row; operator recovery is required",
        ));
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| export_error("audit sink must contain valid UTF-8"))?;
    text.split_terminator('\n')
        .map(|line| {
            if line.is_empty() {
                return Err(export_error("audit sink contains an empty row"));
            }
            if line.len() > MAX_LINE_BYTES {
                return Err(export_error("audit sink row exceeds the bounded line size"));
            }
            let parsed = parse(line)
                .map_err(|error| export_error(format!("audit sink row is invalid: {error}")))?;
            let canonical = canonicalize(&parsed).map_err(|error| {
                export_error(format!("audit sink row cannot be canonicalized: {error}"))
            })?;
            if canonical.as_str() != line {
                return Err(export_error("audit sink row is not canonical JCS"));
            }
            Ok(line.to_owned())
        })
        .collect()
}

fn export_envelope(row: &SealedEvidence) -> Result<Canonical> {
    let record = parse(&row.record_json)?;
    canonicalize(&Value::Object(vec![
        string("format", EXPORT_FORMAT),
        string("format_version", EXPORT_FORMAT_VERSION),
        ("outbox_id".into(), Value::Int(row.outbox_id)),
        (
            "authority_sequence".into(),
            Value::Int(row.authority_sequence),
        ),
        ("ordinal".into(), Value::Int(row.ordinal)),
        string("record_type", &row.record_type),
        string("source_id", &row.source_id),
        ("record".into(), record),
        string("record_hash", &row.record_hash),
        (
            "previous_chain_hash".into(),
            row.previous_chain_hash
                .as_ref()
                .map_or(Value::Null, |value| Value::String(value.clone())),
        ),
        string("chain_hash", &row.chain_hash),
        string("seal_algorithm", &row.seal_algorithm),
        string("seal_key_id", &row.seal_key_id),
        string("seal", &row.seal),
    ]))
}

fn string(name: &str, value: &str) -> (String, Value) {
    (name.into(), Value::String(value.into()))
}

fn export_error(message: impl Into<String>) -> Error {
    Error::coded("AUDIT_EXPORT_FAILED", message)
}

fn export_io(action: &str, error: std::io::Error) -> Error {
    export_error(format!("{action}: {error}"))
}
