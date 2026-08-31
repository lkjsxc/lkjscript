use crate::error::DevError;
use crate::process;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const MAXIMUM_AUTHORITY_FILES: usize = 1_100_000;
const MAXIMUM_AUTHORITY_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const MAXIMUM_HEAD_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthorityObservation {
    pub(crate) files: u64,
    pub(crate) bytes: u64,
    pub(crate) head_sha256: String,
    pub(crate) inventory_sha256: String,
}

pub(crate) fn observe_graph_authority(
    application: &Path,
) -> Result<AuthorityObservation, DevError> {
    let head = process::read_bounded(&application.join("HEAD"), MAXIMUM_HEAD_BYTES)?;
    let mut paths = vec![("HEAD".to_owned(), application.join("HEAD"))];
    collect_authority_directory(application, "packs", &mut paths)?;
    collect_authority_directory(application, "PACKAGE-TRANSPORTS", &mut paths)?;
    paths.sort_by(|left, right| left.0.cmp(&right.0));
    if paths.len() > MAXIMUM_AUTHORITY_FILES {
        return Err(DevError::corrupt(
            "Graph authority inventory exceeded the maintained file-count bound",
        ));
    }

    let mut hasher = Sha256::new();
    hasher.update(b"lkjscript.graph-authority-inventory.v1");
    let mut total_bytes = 0_u64;
    for (relative, path) in &paths {
        let metadata = fs::symlink_metadata(path).map_err(|error| {
            DevError::infrastructure(format!(
                "inspect Graph authority input '{relative}': {error}"
            ))
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(DevError::corrupt(format!(
                "Graph authority input '{relative}' is not a regular file"
            )));
        }
        total_bytes = total_bytes
            .checked_add(metadata.len())
            .ok_or_else(|| DevError::corrupt("Graph authority inventory byte count overflowed"))?;
        if total_bytes > MAXIMUM_AUTHORITY_BYTES {
            return Err(DevError::corrupt(
                "Graph authority inventory exceeded the maintained byte bound",
            ));
        }
        let bytes = process::read_bounded(path, metadata.len())?;
        if u64::try_from(bytes.len()).ok() != Some(metadata.len()) {
            return Err(DevError::corrupt(
                "Graph authority input changed while its inventory was captured",
            ));
        }
        let relative_bytes = relative.as_bytes();
        hasher.update((relative_bytes.len() as u64).to_be_bytes());
        hasher.update(relative_bytes);
        hasher.update(metadata.len().to_be_bytes());
        hasher.update(&bytes);
    }
    Ok(AuthorityObservation {
        files: paths.len() as u64,
        bytes: total_bytes,
        head_sha256: sha256_hex(&head),
        inventory_sha256: lower_hex(&hasher.finalize()),
    })
}

fn collect_authority_directory(
    application: &Path,
    relative: &str,
    output: &mut Vec<(String, PathBuf)>,
) -> Result<(), DevError> {
    let directory = application.join(relative);
    let metadata = fs::symlink_metadata(&directory).map_err(|error| {
        DevError::infrastructure(format!(
            "inspect Graph authority directory '{relative}': {error}"
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(DevError::corrupt(format!(
            "Graph authority directory '{relative}' is not a real directory"
        )));
    }
    let mut entries = fs::read_dir(&directory)
        .map_err(|error| {
            DevError::infrastructure(format!(
                "read Graph authority directory '{relative}': {error}"
            ))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            DevError::infrastructure(format!(
                "read Graph authority entry under '{relative}': {error}"
            ))
        })?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let name = entry.file_name().into_string().map_err(|_| {
            DevError::corrupt("Graph authority inventory contains a non-UTF-8 path")
        })?;
        if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains('\0') {
            return Err(DevError::corrupt(
                "Graph authority inventory contains a noncanonical path",
            ));
        }
        let child = format!("{relative}/{name}");
        let metadata = fs::symlink_metadata(entry.path()).map_err(|error| {
            DevError::infrastructure(format!("inspect Graph authority entry '{child}': {error}"))
        })?;
        if metadata.file_type().is_symlink() {
            return Err(DevError::corrupt(format!(
                "Graph authority entry '{child}' is a symbolic link"
            )));
        }
        if metadata.is_dir() {
            collect_authority_directory(application, &child, output)?;
        } else if metadata.is_file() {
            output.push((child, entry.path()));
            if output.len() > MAXIMUM_AUTHORITY_FILES {
                return Err(DevError::corrupt(
                    "Graph authority inventory exceeded the maintained file-count bound",
                ));
            }
        } else {
            return Err(DevError::corrupt(format!(
                "Graph authority entry '{child}' is not a regular file or directory"
            )));
        }
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    lower_hex(&Sha256::digest(bytes))
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
    for &byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_files_are_excluded_from_graph_authority_inventory() {
        let temporary = tempfile::tempdir().expect("authority fixture");
        fs::create_dir(temporary.path().join("catalog")).expect("catalog");
        fs::create_dir(temporary.path().join("packs")).expect("packs");
        fs::create_dir(temporary.path().join("PACKAGE-TRANSPORTS")).expect("transports");
        fs::write(temporary.path().join("HEAD"), b"head\n").expect("HEAD");
        fs::write(temporary.path().join("catalog/current.lkjc"), b"catalog\n")
            .expect("catalog head");
        fs::write(temporary.path().join("packs/one"), b"pack\n").expect("pack");
        let before = observe_graph_authority(temporary.path()).expect("before");
        fs::create_dir(temporary.path().join("derived")).expect("derived");
        fs::write(temporary.path().join("derived/cache"), b"cache\n").expect("cache");
        fs::write(
            temporary.path().join("catalog/current.lkjc"),
            b"rebuilt catalog\n",
        )
        .expect("rebuilt catalog");
        assert_eq!(
            observe_graph_authority(temporary.path()).expect("after derived"),
            before
        );
        fs::write(temporary.path().join("HEAD"), b"changed\n").expect("changed HEAD");
        assert_ne!(
            observe_graph_authority(temporary.path()).expect("after authority"),
            before
        );
    }
}
