//! Exact reuse is opt-in for content-addressed build outputs, not ordinary --output.

use super::super::{deployment_error, deployment_io};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::owned_output::{
    OwnedOutputReceipt, inspect_create_new, publish_create_new, publish_private_create_new,
};
use rustix::fs::{Mode, OFlags};
use std::fs::File;
use std::io::Read;
use std::path::Path;

fn open_regular(path: &Path) -> Result<File, Diagnostic> {
    let observed = std::fs::symlink_metadata(path)
        .map_err(|error| deployment_io("deployment_read", path, error))?;
    if !observed.is_file() || observed.file_type().is_symlink() {
        return Err(deployment_error(
            "deployment_input_kind",
            "build input is not an ordinary non-symlink file",
        ));
    }
    let descriptor = rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| deployment_io("deployment_read", path, error.into()))?;
    let file = File::from(descriptor);
    let metadata = file
        .metadata()
        .map_err(|error| deployment_io("deployment_read", path, error))?;
    if !metadata.is_file() {
        return Err(deployment_error(
            "deployment_input_kind",
            "build input is not an ordinary non-symlink file",
        ));
    }
    Ok(file)
}

fn read_file(file: &mut File, path: &Path, maximum: Option<usize>) -> Result<Vec<u8>, Diagnostic> {
    let length = file
        .metadata()
        .map_err(|error| deployment_io("deployment_read", path, error))?
        .len();
    if maximum.is_some_and(|maximum| u128::from(length) > maximum as u128) {
        return Err(input_limit());
    }
    let mut bytes = Vec::new();
    let mut remaining = maximum;
    let mut chunk = [0_u8; 65_536];
    loop {
        let count = match file.read(&mut chunk) {
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(deployment_io("deployment_read", path, error)),
        };
        if count == 0 {
            return Ok(bytes);
        }
        if let Some(available) = remaining {
            remaining = Some(available.checked_sub(count).ok_or_else(input_limit)?);
        }
        bytes.try_reserve(count).map_err(|_| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "deployment_input_allocation",
                "cannot allocate build input",
            )
        })?;
        bytes.extend_from_slice(&chunk[..count]);
    }
}

fn input_limit() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "deployment_input_limit",
        "build input exceeds its configured byte allowance",
    )
}

pub(super) fn read_regular(path: &Path, maximum: usize) -> Result<Vec<u8>, Diagnostic> {
    read_file(&mut open_regular(path)?, path, Some(maximum))
}

fn verify_exact(path: &Path, expected: &[u8], owner_only: bool) -> Result<File, Diagnostic> {
    let mut file = open_regular(path)?;
    if owner_only {
        verify_private_access(&file, path)?;
    }
    let observed = read_file(&mut file, path, Some(expected.len())).map_err(|error| {
        if error.code == "deployment_input_limit" {
            conflict(path)
        } else {
            error
        }
    })?;
    if observed != expected {
        return Err(conflict(path));
    }
    Ok(file)
}

fn verify_private_access(file: &File, path: &Path) -> Result<(), Diagnostic> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = file
            .metadata()
            .map_err(|error| deployment_io("deployment_read", path, error))?;
        if metadata.mode() & 0o077 == 0 && metadata.uid() == rustix::process::geteuid().as_raw() {
            return Ok(());
        }
    }
    #[cfg(not(unix))]
    let _ = (file, path);
    Err(deployment_error(
        "deployment_build_permissions",
        "existing deployment snapshot is not owner-only for the current user; no permissions or ownership are changed",
    ))
}

fn conflict(path: &Path) -> Diagnostic {
    deployment_error(
        "deployment_build_conflict",
        format!(
            "build output '{}' already contains different bytes; nothing is overwritten",
            path.display()
        ),
    )
}

pub(super) fn inspect_exact(
    path: &Path,
    bytes: &[u8],
    maximum: Option<usize>,
) -> Result<(), Diagnostic> {
    inspect_with_access(path, bytes, maximum, false)
}

pub(super) fn inspect_private_exact(
    path: &Path,
    bytes: &[u8],
    maximum: usize,
) -> Result<(), Diagnostic> {
    inspect_with_access(path, bytes, Some(maximum), true)
}

fn inspect_with_access(
    path: &Path,
    bytes: &[u8],
    maximum: Option<usize>,
    owner_only: bool,
) -> Result<(), Diagnostic> {
    if maximum.is_some_and(|maximum| bytes.len() > maximum) {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "output_byte_limit",
            "build output exceeds its byte bound",
        ));
    }
    match inspect_create_new(path) {
        Ok(_) => Ok(()),
        Err(error) if error.code == "output_conflict" => {
            verify_exact(path, bytes, owner_only).map(drop)
        }
        Err(error) => Err(error),
    }
}

pub(super) fn publish_exact(
    path: &Path,
    bytes: &[u8],
    maximum: Option<usize>,
) -> Result<OwnedOutputReceipt, Diagnostic> {
    publish_with_access(path, bytes, maximum, false)
}

pub(super) fn publish_private_exact(
    path: &Path,
    bytes: &[u8],
    maximum: usize,
) -> Result<OwnedOutputReceipt, Diagnostic> {
    publish_with_access(path, bytes, Some(maximum), true)
}

fn publish_with_access(
    path: &Path,
    bytes: &[u8],
    maximum: Option<usize>,
    owner_only: bool,
) -> Result<OwnedOutputReceipt, Diagnostic> {
    let result = if owner_only {
        publish_private_create_new(path, bytes, maximum, "immutable private build output")
    } else {
        publish_create_new(path, bytes, maximum, "immutable build output")
    };
    match result {
        Ok(receipt) => Ok(receipt),
        Err(error) if error.code == "output_conflict" && error.notes.is_empty() => {
            // Do not hide a failed owned-stage cleanup behind successful exact reuse.
            // Recheck after create-new failure; another identical builder may have won.
            let file = verify_exact(path, bytes, owner_only)?;
            let synchronized = file
                .sync_all()
                .and_then(|()| {
                    let parent = path.parent().unwrap_or_else(|| Path::new("."));
                    File::open(parent)?.sync_all()
                })
                .is_ok();
            Ok(OwnedOutputReceipt {
                path: path.to_owned(),
                bytes: bytes.len() as u64,
                visibility: "reused-exact",
                durability: if synchronized {
                    "synchronized"
                } else {
                    "uncertain"
                },
                stage_cleanup: "none-retained",
            })
        }
        Err(error) => Err(error),
    }
}
