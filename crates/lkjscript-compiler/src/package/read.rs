use std::fs::{File, Metadata};
use std::io::Read;
use std::path::Path;

use lkjscript_core::{Error, Result};

// Private allocation and syscall tuning, never an admission policy.
const INITIAL_READ_CAPACITY: usize = 64 * 1024;
const READ_CHUNK_BYTES: usize = 64 * 1024;

pub(super) fn unchanged_file(path: &Path, label: &str) -> Result<Vec<u8>> {
    let mut file = File::open(path)
        .map_err(|error| Error::host(format!("open {label} {}: {error}", path.display())))?;
    let initial = file.metadata().map_err(|error| {
        Error::host(format!(
            "inspect opened {label} {}: {error}",
            path.display()
        ))
    })?;
    if !initial.is_file() {
        return Err(Error::host(format!(
            "{label} is not a regular file: {}",
            path.display()
        )));
    }

    let bytes = bytes(&mut file, initial.len(), path, label)?;
    let opened_final = file.metadata().map_err(|error| {
        Error::host(format!(
            "reinspect opened {label} {}: {error}",
            path.display()
        ))
    })?;
    let path_final = std::fs::symlink_metadata(path).map_err(|error| {
        Error::host(format!(
            "reinspect {label} path {}: {error}",
            path.display()
        ))
    })?;
    let actual = u64::try_from(bytes.len()).map_err(|_| {
        Error::host(format!(
            "{label} byte length is not representable on this host: {}",
            path.display()
        ))
    })?;
    if changed(&initial, &opened_final, &path_final, actual) {
        return Err(Error::host(format!(
            concat!(
                "{} changed while reading {}: initial-bytes={}; ",
                "read-bytes={}; final-bytes={}"
            ),
            label,
            path.display(),
            initial.len(),
            actual,
            opened_final.len()
        )));
    }
    Ok(bytes)
}

pub(super) fn bytes<R: Read>(
    reader: &mut R,
    expected_bytes: u64,
    path: &Path,
    label: &str,
) -> Result<Vec<u8>> {
    let initial_capacity = usize::try_from(expected_bytes)
        .unwrap_or(usize::MAX)
        .min(INITIAL_READ_CAPACITY);
    let mut output = Vec::new();
    output.try_reserve(initial_capacity).map_err(|_| {
        Error::host(format!(
            "host could not reserve memory while reading {label} {}",
            path.display()
        ))
    })?;

    let mut actual_bytes = 0_u64;
    let mut chunk = [0_u8; READ_CHUNK_BYTES];
    loop {
        let read = match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                return Err(Error::host(format!(
                    "read {label} {}: {error}",
                    path.display()
                )));
            }
        };
        let read_u64 = u64::try_from(read).map_err(|_| {
            Error::host(format!(
                "{label} read length is not representable on this host: {}",
                path.display()
            ))
        })?;
        actual_bytes = actual_bytes.checked_add(read_u64).ok_or_else(|| {
            Error::host(format!(
                "{label} byte length overflow while reading {}",
                path.display()
            ))
        })?;
        output.try_reserve(read).map_err(|_| {
            Error::host(format!(
                "host could not reserve memory while reading {label} {}",
                path.display()
            ))
        })?;
        output.extend_from_slice(&chunk[..read]);
    }

    if actual_bytes != expected_bytes {
        return Err(Error::host(format!(
            "{label} size changed while reading {}: metadata={expected_bytes}; read={actual_bytes}",
            path.display()
        )));
    }
    Ok(output)
}

fn changed(
    initial: &Metadata,
    opened_final: &Metadata,
    path_final: &Metadata,
    actual: u64,
) -> bool {
    !opened_final.is_file()
        || !path_final.is_file()
        || actual != initial.len()
        || metadata_changed(initial, opened_final)
        || !same_file(initial, path_final)
}

#[cfg(unix)]
fn same_file(left: &Metadata, right: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_file(left: &Metadata, right: &Metadata) -> bool {
    left.len() == right.len()
        && left.created().ok() == right.created().ok()
        && left.modified().ok() == right.modified().ok()
}

#[cfg(unix)]
fn metadata_changed(initial: &Metadata, final_metadata: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    initial.len() != final_metadata.len()
        || initial.mtime() != final_metadata.mtime()
        || initial.mtime_nsec() != final_metadata.mtime_nsec()
        || initial.ctime() != final_metadata.ctime()
        || initial.ctime_nsec() != final_metadata.ctime_nsec()
}

#[cfg(not(unix))]
fn metadata_changed(initial: &Metadata, final_metadata: &Metadata) -> bool {
    initial.len() != final_metadata.len()
        || initial.modified().ok() != final_metadata.modified().ok()
        || initial.created().ok() != final_metadata.created().ok()
}

#[cfg(all(test, unix))]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn replacement_identity_is_rejected_even_at_the_same_length() {
        let nonce = NEXT.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "lkjscript-package-reader-replacement-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("lkjscript.package.json");
        let replacement = directory.join("replacement");
        fs::write(&path, b"old").unwrap();
        let opened = File::open(&path).unwrap();
        let initial = opened.metadata().unwrap();
        fs::write(&replacement, b"new").unwrap();
        fs::rename(&replacement, &path).unwrap();
        let opened_final = opened.metadata().unwrap();
        let path_final = fs::symlink_metadata(&path).unwrap();

        assert!(changed(&initial, &opened_final, &path_final, 3));
        fs::remove_dir_all(directory).unwrap();
    }
}
