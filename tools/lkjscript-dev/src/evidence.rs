use crate::error::DevError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const DIGEST_DOMAIN: &str = "lkjscript.dev.verification-evidence.v1";
const MAXIMUM_EVIDENCE_BYTES: usize = 128 * 1024 * 1024;
static STAGE_ORDINAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct VerificationDigest(String);

impl VerificationDigest {
    pub(crate) fn of(bytes: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new_derive_key(DIGEST_DOMAIN);
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
        Self(format!("verification_{}", hasher.finalize().to_hex()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VerificationDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileProof {
    pub(crate) path: String,
    pub(crate) kind: FileKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) mode: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) digest: Option<VerificationDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) link_target: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FileKind {
    Missing,
    File,
    Directory,
    Symlink,
    Unsupported,
}

#[derive(Clone, Debug)]
pub(crate) struct PublishedEvidence {
    pub(crate) path: PathBuf,
    pub(crate) bytes: u64,
    pub(crate) digest: VerificationDigest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AtomicCheckpoint {
    StageCreated,
    BytesWritten,
    FileSynchronized,
    Published,
    DirectorySynchronized,
}

pub(crate) fn encode_json<T: Serialize>(value: &T) -> Result<Vec<u8>, DevError> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| DevError::infrastructure(format!("encode evidence: {error}")))?;
    bytes.push(b'\n');
    if bytes.len() > MAXIMUM_EVIDENCE_BYTES {
        return Err(DevError::infrastructure(format!(
            "evidence exceeds {MAXIMUM_EVIDENCE_BYTES} bytes"
        )));
    }
    Ok(bytes)
}

pub(crate) fn publish_json<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<PublishedEvidence, DevError> {
    let bytes = encode_json(value)?;
    publish(path, &bytes)
}

pub(crate) fn publish(path: &Path, bytes: &[u8]) -> Result<PublishedEvidence, DevError> {
    publish_with_checkpoints(path, bytes, &mut |_| Ok(()))
}

pub(crate) fn publish_with_checkpoints(
    path: &Path,
    bytes: &[u8],
    checkpoint: &mut dyn FnMut(AtomicCheckpoint) -> Result<(), DevError>,
) -> Result<PublishedEvidence, DevError> {
    if bytes.len() > MAXIMUM_EVIDENCE_BYTES {
        return Err(DevError::infrastructure(format!(
            "evidence exceeds {MAXIMUM_EVIDENCE_BYTES} bytes"
        )));
    }
    let parent = path.parent().ok_or_else(|| {
        DevError::infrastructure(format!("evidence path '{}' has no parent", path.display()))
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        DevError::infrastructure(format!(
            "create evidence directory '{}': {error}",
            parent.display()
        ))
    })?;
    ensure_directory(parent)?;
    if let Ok(metadata) = fs::symlink_metadata(path)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err(DevError::infrastructure(format!(
            "evidence destination '{}' is not a regular file",
            path.display()
        )));
    }

    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| DevError::infrastructure("evidence file name is not portable UTF-8"))?;
    let stage = parent.join(format!(".{file_name}.stage-{}", unique_suffix()?));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&stage).map_err(|error| {
            DevError::infrastructure(format!(
                "create evidence stage '{}': {error}",
                stage.display()
            ))
        })?;
        checkpoint(AtomicCheckpoint::StageCreated)?;
        file.write_all(bytes).map_err(|error| {
            DevError::infrastructure(format!(
                "write evidence stage '{}': {error}",
                stage.display()
            ))
        })?;
        checkpoint(AtomicCheckpoint::BytesWritten)?;
        file.sync_all().map_err(|error| {
            DevError::infrastructure(format!(
                "synchronize evidence stage '{}': {error}",
                stage.display()
            ))
        })?;
        checkpoint(AtomicCheckpoint::FileSynchronized)?;
        drop(file);
        fs::rename(&stage, path).map_err(|error| {
            DevError::infrastructure(format!("publish evidence '{}': {error}", path.display()))
        })?;
        checkpoint(AtomicCheckpoint::Published)?;
        synchronize_directory(parent)?;
        checkpoint(AtomicCheckpoint::DirectorySynchronized)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&stage);
    }
    result?;
    Ok(PublishedEvidence {
        path: path.to_path_buf(),
        bytes: bytes.len() as u64,
        digest: VerificationDigest::of(bytes),
    })
}

pub(crate) fn proof(path: &Path, label: String) -> Result<FileProof, DevError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(FileProof {
                path: label,
                kind: FileKind::Missing,
                mode: None,
                bytes: None,
                digest: None,
                link_target: None,
            });
        }
        Err(error) => {
            return Err(DevError::infrastructure(format!(
                "inspect '{}': {error}",
                path.display()
            )));
        }
    };
    #[cfg(unix)]
    let mode = Some(metadata.permissions().mode() & 0o7777);
    #[cfg(not(unix))]
    let mode = None;
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(path).map_err(|error| {
            DevError::infrastructure(format!("read symlink '{}': {error}", path.display()))
        })?;
        let target = target.to_string_lossy().into_owned();
        return Ok(FileProof {
            path: label,
            kind: FileKind::Symlink,
            mode,
            bytes: Some(target.len() as u64),
            digest: Some(VerificationDigest::of(target.as_bytes())),
            link_target: Some(target),
        });
    }
    if metadata.is_file() {
        let (digest, bytes) = digest_file(path)?;
        return Ok(FileProof {
            path: label,
            kind: FileKind::File,
            mode,
            bytes: Some(bytes),
            digest: Some(digest),
            link_target: None,
        });
    }
    Ok(FileProof {
        path: label,
        kind: if metadata.is_dir() {
            FileKind::Directory
        } else {
            FileKind::Unsupported
        },
        mode,
        bytes: None,
        digest: None,
        link_target: None,
    })
}

pub(crate) fn digest_file(path: &Path) -> Result<(VerificationDigest, u64), DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!("inspect file '{}': {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::infrastructure(format!(
            "'{}' is not a regular non-symlink file",
            path.display()
        )));
    }
    let mut file = File::open(path).map_err(|error| {
        DevError::infrastructure(format!("open file '{}': {error}", path.display()))
    })?;
    let mut hasher = blake3::Hasher::new_derive_key(DIGEST_DOMAIN);
    hasher.update(&metadata.len().to_be_bytes());
    let mut buffer = [0_u8; 64 * 1024];
    let mut observed = 0_u64;
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            DevError::infrastructure(format!("read file '{}': {error}", path.display()))
        })?;
        if read == 0 {
            break;
        }
        observed = observed.checked_add(read as u64).ok_or_else(|| {
            DevError::infrastructure(format!("file '{}' length overflow", path.display()))
        })?;
        hasher.update(&buffer[..read]);
    }
    if observed != metadata.len() {
        return Err(DevError::infrastructure(format!(
            "file '{}' changed during digest",
            path.display()
        )));
    }
    Ok((
        VerificationDigest(format!("verification_{}", hasher.finalize().to_hex())),
        observed,
    ))
}

pub(crate) fn synchronize_file(path: &Path) -> Result<(), DevError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| {
            DevError::infrastructure(format!("synchronize '{}': {error}", path.display()))
        })
}

pub(crate) fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

fn ensure_directory(path: &Path) -> Result<(), DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!("inspect directory '{}': {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(DevError::infrastructure(format!(
            "'{}' is not a regular non-symlink directory",
            path.display()
        )));
    }
    Ok(())
}

fn synchronize_directory(path: &Path) -> Result<(), DevError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            DevError::infrastructure(format!(
                "synchronize directory '{}': {error}",
                path.display()
            ))
        })
}

fn unique_suffix() -> Result<String, DevError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| DevError::infrastructure(format!("system clock before epoch: {error}")))?;
    let ordinal = STAGE_ORDINAL.fetch_add(1, Ordering::Relaxed);
    Ok(format!(
        "{}-{}-{ordinal}",
        std::process::id(),
        now.as_nanos()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_publication_replaces_only_with_complete_bytes() {
        let temporary = tempfile::tempdir().expect("temporary evidence directory");
        let destination = temporary.path().join("receipt.json");
        publish(&destination, b"old\n").expect("publish initial evidence");
        let mut failed = false;
        let result = publish_with_checkpoints(&destination, b"new\n", &mut |checkpoint| {
            if checkpoint == AtomicCheckpoint::BytesWritten && !failed {
                failed = true;
                return Err(DevError::infrastructure("injected write boundary"));
            }
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(
            fs::read(&destination).expect("read retained evidence"),
            b"old\n"
        );
        let published = publish(&destination, b"new\n").expect("replace evidence");
        assert_eq!(published.bytes, 4);
        assert_eq!(fs::read(destination).expect("read new evidence"), b"new\n");
    }

    #[test]
    fn proof_rejects_digesting_symlink_as_file() {
        let temporary = tempfile::tempdir().expect("temporary evidence directory");
        let source = temporary.path().join("source");
        fs::write(&source, b"bytes").expect("write source");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&source, temporary.path().join("link"))
                .expect("create link");
            assert!(digest_file(&temporary.path().join("link")).is_err());
        }
    }
}
