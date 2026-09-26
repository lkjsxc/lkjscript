//! Exact reuse is opt-in for content-addressed build outputs, not ordinary --output.

use super::super::{deployment_error, deployment_io};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::owned_output::{OwnedOutputReceipt, inspect_create_new, publish_create_new};
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

fn read_file(file: &mut File, path: &Path, maximum: usize) -> Result<Vec<u8>, Diagnostic> {
    let maximum = u64::try_from(maximum)
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| deployment_error("deployment_input_limit", "input bound overflow"))?;
    let length = file
        .metadata()
        .map_err(|error| deployment_io("deployment_read", path, error))?
        .len();
    if length >= maximum {
        return Err(deployment_error(
            "deployment_input_limit",
            "build input exceeds its byte bound",
        ));
    }
    let mut bytes = Vec::new();
    file.take(maximum)
        .read_to_end(&mut bytes)
        .map_err(|error| deployment_io("deployment_read", path, error))?;
    if u64::try_from(bytes.len()).map_or(true, |length| length >= maximum) {
        return Err(deployment_error(
            "deployment_input_limit",
            "build input exceeds its byte bound",
        ));
    }
    Ok(bytes)
}

pub(super) fn read_regular(path: &Path, maximum: usize) -> Result<Vec<u8>, Diagnostic> {
    read_file(&mut open_regular(path)?, path, maximum)
}

fn verify_exact(path: &Path, expected: &[u8]) -> Result<File, Diagnostic> {
    let mut file = open_regular(path)?;
    let observed = read_file(&mut file, path, expected.len()).map_err(|error| {
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

fn conflict(path: &Path) -> Diagnostic {
    deployment_error(
        "deployment_build_conflict",
        format!(
            "build output '{}' already contains different bytes; nothing is overwritten",
            path.display()
        ),
    )
}

pub(super) fn inspect_exact(path: &Path, bytes: &[u8], maximum: usize) -> Result<(), Diagnostic> {
    if bytes.len() > maximum {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "output_byte_limit",
            "build output exceeds its byte bound",
        ));
    }
    match inspect_create_new(path) {
        Ok(_) => Ok(()),
        Err(error) if error.code == "output_conflict" => verify_exact(path, bytes).map(drop),
        Err(error) => Err(error),
    }
}

pub(super) fn publish_exact(
    path: &Path,
    bytes: &[u8],
    maximum: usize,
) -> Result<OwnedOutputReceipt, Diagnostic> {
    match publish_create_new(path, bytes, maximum, "immutable build output") {
        Ok(receipt) => Ok(receipt),
        Err(error) if error.code == "output_conflict" && error.notes.is_empty() => {
            // Do not hide a failed owned-stage cleanup behind successful exact reuse.
            // Recheck after create-new failure; another identical builder may have won.
            let file = verify_exact(path, bytes)?;
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
