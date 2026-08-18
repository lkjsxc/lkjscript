//! Shared hostile-file input and no-overwrite immutable-artifact publication.

use crate::error::{ErrorCode, LkError, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

pub(crate) const MAXIMUM_ARTIFACT_PATH_BYTES: usize = 4_096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublicationFault {
    None,
    BeforeWrite,
    AfterWrite,
    AfterFileSync,
    AfterLink,
    AfterTemporaryRemoval,
    AfterDirectorySync,
}

pub(crate) fn read_file(path: &Path, label: &str, maximum_bytes: usize) -> Result<Vec<u8>> {
    validate_existing_regular_path(path, label, maximum_bytes)?;
    let file = File::open(path).map_err(|error| {
        LkError::new(
            ErrorCode::Io,
            format!("cannot open {label} {}: {error}", path.display()),
        )
    })?;
    let limit = u64::try_from(maximum_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::new();
    file.take(limit).read_to_end(&mut bytes).map_err(|error| {
        LkError::new(
            ErrorCode::Io,
            format!("cannot read {label} {}: {error}", path.display()),
        )
    })?;
    if bytes.len() > maximum_bytes {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{label} exceeds the artifact byte policy"),
        ));
    }
    Ok(bytes)
}

pub(crate) fn publish(
    path: &Path,
    bytes: &[u8],
    label: &str,
    temporary_prefix: &str,
    maximum_bytes: usize,
) -> Result<()> {
    publish_with_fault(
        path,
        bytes,
        label,
        temporary_prefix,
        maximum_bytes,
        PublicationFault::None,
    )
}

pub(crate) fn publish_with_fault(
    path: &Path,
    bytes: &[u8],
    label: &str,
    temporary_prefix: &str,
    maximum_bytes: usize,
    fault: PublicationFault,
) -> Result<()> {
    if bytes.len() > maximum_bytes {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{label} publication bytes exceed policy"),
        ));
    }
    validate_canonical_absolute_path(path, label)?;
    validate_parent_chain(path, label)?;
    match fs::symlink_metadata(path) {
        Ok(_) => {
            return Err(LkError::new(
                ErrorCode::Io,
                format!("{label} destination already exists"),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let parent = path.parent().ok_or_else(|| {
        LkError::new(
            ErrorCode::Io,
            format!("{label} destination has no parent directory"),
        )
    })?;
    let (temporary_path, mut temporary) = create_temporary_file(parent, label, temporary_prefix)?;
    let before_publication = (|| -> Result<()> {
        if fault == PublicationFault::BeforeWrite {
            return Err(injected(label, "before write"));
        }
        temporary.write_all(bytes)?;
        if fault == PublicationFault::AfterWrite {
            return Err(injected(label, "after write"));
        }
        temporary.sync_all()?;
        if fault == PublicationFault::AfterFileSync {
            return Err(injected(label, "after file sync"));
        }
        fs::hard_link(&temporary_path, path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                LkError::new(
                    ErrorCode::Io,
                    format!("{label} destination appeared during publication"),
                )
            } else {
                LkError::new(
                    ErrorCode::Io,
                    format!("cannot publish {label} link: {error}"),
                )
            }
        })?;
        Ok(())
    })();
    if let Err(error) = before_publication {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    if fault == PublicationFault::AfterLink {
        let cleanup = fs::remove_file(&temporary_path);
        return Err(unknown(
            label,
            match cleanup {
                Ok(()) => format!("injected failure after {label} link publication"),
                Err(error) => format!(
                    "injected failure after {label} link publication; temporary cleanup also failed: {error}"
                ),
            },
        ));
    }
    if let Err(error) = fs::remove_file(&temporary_path) {
        return Err(unknown(
            label,
            format!("{label} was linked but temporary cleanup failed: {error}"),
        ));
    }
    if fault == PublicationFault::AfterTemporaryRemoval {
        return Err(unknown(
            label,
            format!("injected failure before {label} directory sync"),
        ));
    }
    let directory = File::open(parent).map_err(|error| {
        unknown(
            label,
            format!("{label} was linked but parent open failed: {error}"),
        )
    })?;
    directory.sync_all().map_err(|error| {
        unknown(
            label,
            format!("{label} was linked but parent sync failed: {error}"),
        )
    })?;
    if fault == PublicationFault::AfterDirectorySync {
        return Err(unknown(
            label,
            format!("injected failure after {label} directory sync"),
        ));
    }
    Ok(())
}

fn validate_canonical_absolute_path(path: &Path, label: &str) -> Result<()> {
    let bytes = path.as_os_str().as_bytes();
    if bytes.len() > MAXIMUM_ARTIFACT_PATH_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{label} path exceeds its byte policy"),
        ));
    }
    if !path.is_absolute() || bytes.first() != Some(&b'/') {
        return Err(LkError::new(
            ErrorCode::Io,
            format!("{label} paths must be absolute"),
        ));
    }
    if bytes[1..]
        .split(|byte| *byte == b'/')
        .any(|component| component.is_empty() || component == b"." || component == b"..")
    {
        return Err(LkError::new(
            ErrorCode::Io,
            format!("{label} paths must use one separator and no empty or dot components"),
        ));
    }
    Ok(())
}

fn validate_existing_regular_path(path: &Path, label: &str, maximum_bytes: usize) -> Result<()> {
    validate_canonical_absolute_path(path, label)?;
    validate_parent_chain(path, label)?;
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        LkError::new(
            ErrorCode::Io,
            format!("cannot inspect {label} {}: {error}", path.display()),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LkError::new(
            ErrorCode::Io,
            format!("{label} must be a regular non-symlink file"),
        ));
    }
    if metadata.len() > u64::try_from(maximum_bytes).unwrap_or(u64::MAX) {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{label} exceeds the artifact byte policy"),
        ));
    }
    Ok(())
}

fn validate_parent_chain(path: &Path, label: &str) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        LkError::new(
            ErrorCode::Io,
            format!("{label} path has no parent directory"),
        )
    })?;
    let mut current = PathBuf::from("/");
    for component in parent.components() {
        match component {
            Component::RootDir => continue,
            Component::Normal(part) => current.push(part),
            _ => {
                return Err(LkError::new(
                    ErrorCode::Io,
                    format!("{label} parent path is not canonical"),
                ));
            }
        }
        let metadata = fs::symlink_metadata(&current).map_err(|error| {
            LkError::new(
                ErrorCode::Io,
                format!(
                    "cannot inspect {label} parent {}: {error}",
                    current.display()
                ),
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(LkError::new(
                ErrorCode::Io,
                format!("{label} parent components must be non-symlink directories"),
            ));
        }
    }
    Ok(())
}

fn create_temporary_file(
    parent: &Path,
    label: &str,
    temporary_prefix: &str,
) -> Result<(PathBuf, File)> {
    for _ in 0..16 {
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).map_err(|error| {
            LkError::new(
                ErrorCode::Io,
                format!("cannot generate {label} temporary file name: {error}"),
            )
        })?;
        let path = parent.join(format!("{temporary_prefix}{}.tmp", hex(&random)));
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(file) => {
                file.set_permissions(fs::Permissions::from_mode(0o600))?;
                return Ok((path, file));
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(LkError::new(
        ErrorCode::Io,
        format!("cannot allocate a unique {label} temporary file"),
    ))
}

fn injected(label: &str, edge: &str) -> LkError {
    LkError::new(
        ErrorCode::Io,
        format!("injected {label} publication failure {edge}"),
    )
}

fn unknown(label: &str, message: String) -> LkError {
    let _ = label;
    LkError::new(ErrorCode::ArtifactPublicationOutcomeUnknown, message)
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}
