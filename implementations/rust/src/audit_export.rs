//! Durable append-only file export for the sealed local evidence outbox.
//!
//! A row is acknowledged in SQLite only after its canonical export envelope
//! has been appended and synced. On restart, a complete sink row that is ahead
//! of its database acknowledgement is verified and acknowledged without being
//! appended a second time.

use crate::authority::Authority;
use crate::authority::OperationalSnapshot;
use crate::error::{Error, Result};
pub use crate::evidence::MAX_AUDIT_SINK_BYTES;
use crate::evidence::{export_envelope, MAX_AUDIT_LINE_BYTES};
use crate::jcs::{canonicalize, parse};
use nix::fcntl::{Flock, FlockArg};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditExportSummary {
    pub recovered: usize,
    pub appended: usize,
    pub total_in_sink: usize,
    pub last_chain_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditSinkReconciliation {
    pub rows_in_sink: usize,
    pub acknowledged_rows: usize,
    pub pending_rows: usize,
    pub recovery_rows: usize,
    pub last_chain_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationalReadinessSnapshot {
    pub authority: OperationalSnapshot,
    pub audit_sink: AuditSinkReconciliation,
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

    /// Verifies the on-disk sink against the complete sealed outbox without
    /// appending or acknowledging rows. Database-only health checks are not a
    /// substitute for this at-rest reconciliation.
    pub fn verify_at_rest(&self, authority: &Authority) -> Result<AuditSinkReconciliation> {
        let rows = authority.evidence_snapshot()?;
        let acknowledged_rows = rows
            .iter()
            .take_while(|row| row.exported_at_ms.is_some())
            .count();
        if !self.path.exists() {
            if acknowledged_rows != 0 {
                return Err(export_error(
                    "authority acknowledges evidence missing from the audit sink",
                ));
            }
            return Ok(AuditSinkReconciliation {
                rows_in_sink: 0,
                acknowledged_rows: 0,
                pending_rows: rows.len(),
                recovery_rows: 0,
                last_chain_hash: None,
            });
        }

        let _parent = protected_parent(&self.path)?;
        let _lock = ExportLock::acquire(&self.path)?;
        let mut sink = secure_existing_sink(&self.path)?;
        let lines = read_lines(&mut sink)?;
        verify_sink_prefix(&rows, &lines)?;
        if acknowledged_rows > lines.len() {
            return Err(export_error(
                "authority acknowledges evidence missing from the audit sink",
            ));
        }
        let recovery_rows = lines.len().saturating_sub(acknowledged_rows);
        Ok(AuditSinkReconciliation {
            rows_in_sink: lines.len(),
            acknowledged_rows,
            pending_rows: rows.len().saturating_sub(acknowledged_rows),
            recovery_rows,
            last_chain_hash: lines
                .len()
                .checked_sub(1)
                .and_then(|index| rows.get(index))
                .map(|row| row.chain_hash.clone()),
        })
    }

    /// Combined readiness view. This is the deploy/restore health boundary;
    /// `Authority::operational_snapshot` intentionally covers SQLite only.
    pub fn operational_readiness(
        &self,
        authority: &Authority,
    ) -> Result<OperationalReadinessSnapshot> {
        let authority_snapshot = authority.operational_snapshot()?;
        let audit_sink = self.verify_at_rest(authority)?;
        Ok(OperationalReadinessSnapshot {
            authority: authority_snapshot,
            audit_sink,
        })
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
        verify_sink_prefix(&rows, &lines)?;

        let mut recovered = 0_usize;
        for (index, _) in lines.iter().enumerate() {
            let row = &rows[index];
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
        let mut sink_bytes = sink
            .metadata()
            .map_err(|error| export_io("inspect audit sink capacity", error))?
            .len();
        let mut appended = 0_usize;
        for row in rows.iter().skip(lines.len()).take(limit) {
            let envelope = export_envelope(row)?;
            if envelope.as_str().len() > MAX_AUDIT_LINE_BYTES {
                return Err(export_error(
                    "audit export row exceeds the bounded line size",
                ));
            }
            let row_bytes = u64::try_from(envelope.as_str().len() + 1)
                .map_err(|_| export_error("audit export row length overflow"))?;
            let projected = projected_sink_bytes(sink_bytes, row_bytes)?;
            sink.write_all(envelope.as_str().as_bytes())
                .and_then(|()| sink.write_all(b"\n"))
                .and_then(|()| sink.sync_all())
                .map_err(|error| export_io("append and sync audit evidence", error))?;
            sink_bytes = projected;
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
    _file: Flock<File>,
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
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(nix::libc::O_NOFOLLOW)
            .open(&path)
            .map_err(|error| export_io("open audit export lock", error))?;
        validate_file(&file, "audit export lock")?;
        file.sync_all()
            .map_err(|error| export_io("sync audit export lock", error))?;
        let file = Flock::lock(file, FlockArg::LockExclusiveNonblock)
            .map_err(|(_, error)| export_error(format!("acquire audit export lock: {error}")))?;
        let descriptor = file
            .metadata()
            .map_err(|error| export_io("inspect held audit export lock", error))?;
        let named = fs::symlink_metadata(&path)
            .map_err(|error| export_io("inspect named audit export lock", error))?;
        if descriptor.dev() != named.dev() || descriptor.ino() != named.ino() {
            return Err(export_error(
                "audit export lock path changed while the lock was acquired",
            ));
        }
        Ok(Self { _file: file })
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

fn secure_existing_sink(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(path)
        .map_err(|error| export_io("open existing audit sink", error))?;
    validate_file(&file, "audit sink")?;
    Ok(file)
}

fn verify_sink_prefix(rows: &[crate::evidence::SealedEvidence], lines: &[String]) -> Result<()> {
    if lines.len() > rows.len() {
        return Err(export_error(
            "audit sink contains rows not present in the authority outbox",
        ));
    }
    for (index, line) in lines.iter().enumerate() {
        let expected = export_envelope(&rows[index])?;
        if line != expected.as_str() {
            return Err(export_error(
                "audit sink is not an exact ordered prefix of the sealed outbox",
            ));
        }
    }
    Ok(())
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
    if length > MAX_AUDIT_SINK_BYTES {
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
            if line.len() > MAX_AUDIT_LINE_BYTES {
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

fn export_error(message: impl Into<String>) -> Error {
    Error::coded("AUDIT_EXPORT_FAILED", message)
}

fn export_io(action: &str, error: std::io::Error) -> Error {
    export_error(format!("{action}: {error}"))
}

fn projected_sink_bytes(current: u64, row_bytes: u64) -> Result<u64> {
    let projected = current
        .checked_add(row_bytes)
        .ok_or_else(|| export_error("audit sink length overflow"))?;
    if projected > MAX_AUDIT_SINK_BYTES {
        return Err(export_error(
            "audit sink capacity would be exceeded; export must stop before append",
        ));
    }
    Ok(projected)
}

#[cfg(test)]
mod tests {
    use super::{projected_sink_bytes, MAX_AUDIT_SINK_BYTES};

    #[test]
    fn capacity_is_checked_before_append() {
        assert_eq!(
            projected_sink_bytes(MAX_AUDIT_SINK_BYTES - 1, 1).unwrap(),
            MAX_AUDIT_SINK_BYTES
        );
        assert!(projected_sink_bytes(MAX_AUDIT_SINK_BYTES, 1).is_err());
        assert!(projected_sink_bytes(u64::MAX, 1).is_err());
    }
}
