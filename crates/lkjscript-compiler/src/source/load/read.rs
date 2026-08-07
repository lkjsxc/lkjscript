use std::fs::{File, Metadata};
use std::io::Read;
use std::path::Path;

use crate::source::{SourceDiagnostic, SourceOrigin, SourceResult};

// Private allocation/read tuning only. Neither value is an admission policy.
const INITIAL_READ_CAPACITY: usize = 64 * 1024;
const READ_CHUNK_BYTES: usize = 64 * 1024;

pub(super) fn read_source(
    file: &mut File,
    metadata: &Metadata,
    canonical: &Path,
    origin: &SourceOrigin,
    completed_source_bytes: &mut u64,
) -> SourceResult<Vec<u8>> {
    let expected_bytes = metadata.len();
    let source_bytes = read_source_bytes(file, expected_bytes, canonical, origin)?;
    let final_metadata = file.metadata().map_err(|error| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("reinspect opened source {}: {error}", canonical.display()),
        )
    })?;
    let actual_bytes = u64::try_from(source_bytes.len()).map_err(|_| {
        SourceDiagnostic::host(
            origin.clone(),
            format!(
                "source byte length is not representable: {}",
                canonical.display()
            ),
        )
    })?;
    if !final_metadata.is_file()
        || final_metadata.len() != expected_bytes
        || actual_bytes != expected_bytes
    {
        return Err(SourceDiagnostic::loading(
            origin.clone(),
            format!(
                concat!(
                    "source size changed while reading {}: metadata={expected_bytes}; ",
                    "read={actual_bytes}; final-metadata={}"
                ),
                canonical.display(),
                final_metadata.len(),
                expected_bytes = expected_bytes,
                actual_bytes = actual_bytes,
            ),
        ));
    }

    *completed_source_bytes = completed_source_bytes
        .checked_add(actual_bytes)
        .ok_or_else(|| {
            SourceDiagnostic::host(
                origin.clone(),
                "aggregate source byte accounting overflowed its u64 representation",
            )
        })?;
    Ok(source_bytes)
}

pub(crate) fn read_source_bytes<R: Read>(
    reader: &mut R,
    expected_bytes: u64,
    path: &Path,
    origin: &SourceOrigin,
) -> SourceResult<Vec<u8>> {
    let initial_capacity = usize::try_from(expected_bytes)
        .unwrap_or(usize::MAX)
        .min(INITIAL_READ_CAPACITY);
    let mut source_bytes = Vec::new();
    source_bytes.try_reserve(initial_capacity).map_err(|_| {
        SourceDiagnostic::host(
            origin.clone(),
            format!("host could not reserve memory while reading source {path:?}"),
        )
    })?;

    let mut actual_bytes = 0_u64;
    let mut chunk = [0_u8; READ_CHUNK_BYTES];
    loop {
        let read = match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                return Err(SourceDiagnostic::loading(
                    origin.clone(),
                    format!("read source {path:?}: {error}"),
                ));
            }
        };
        let read_u64 = u64::try_from(read).map_err(|_| {
            SourceDiagnostic::host(
                origin.clone(),
                format!("source read length is not representable: {path:?}"),
            )
        })?;
        actual_bytes = actual_bytes.checked_add(read_u64).ok_or_else(|| {
            SourceDiagnostic::host(
                origin.clone(),
                format!("source byte length overflow while reading {path:?}"),
            )
        })?;
        source_bytes.try_reserve(read).map_err(|_| {
            SourceDiagnostic::host(
                origin.clone(),
                format!("host could not reserve memory while reading source {path:?}"),
            )
        })?;
        source_bytes.extend_from_slice(&chunk[..read]);
    }

    if actual_bytes != expected_bytes {
        return Err(SourceDiagnostic::loading(
            origin.clone(),
            format!(
                "source size changed while reading {path:?}: metadata={expected_bytes}; read={actual_bytes}"
            ),
        ));
    }
    Ok(source_bytes)
}
