use std::fs::{File, Metadata};
use std::io::Read;
use std::path::Path;

use crate::source::{SourceDiagnostic, SourceFoundationBudget, SourceOrigin, SourceResult};

pub(super) fn read_bounded_source(
    file: &mut File,
    metadata: &Metadata,
    canonical: &Path,
    origin: &SourceOrigin,
    budget: &mut SourceFoundationBudget,
) -> SourceResult<Vec<u8>> {
    let expected_bytes = metadata.len();
    budget.check_metadata(origin, expected_bytes)?;
    let source_bytes = read_bounded_bytes(file, expected_bytes, canonical, origin, budget)?;
    let final_metadata = file.metadata().map_err(|error| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("reinspect opened source {}: {error}", canonical.display()),
        )
    })?;
    let actual_bytes = u64::try_from(source_bytes.len()).map_err(|_| {
        SourceDiagnostic::loading(
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
                actual_bytes = actual_bytes
            ),
        ));
    }
    budget.record_read(origin, actual_bytes)?;
    Ok(source_bytes)
}

pub(crate) fn read_bounded_bytes<R: Read>(
    reader: &mut R,
    expected_bytes: u64,
    path: &Path,
    origin: &SourceOrigin,
    budget: &SourceFoundationBudget,
) -> SourceResult<Vec<u8>> {
    let allowance = budget.remaining_read_allowance(origin)?;
    let read_limit = allowance.checked_add(1).ok_or_else(|| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("bounded source read limit overflow: {path:?}"),
        )
    })?;
    let mut source_bytes = Vec::new();
    reader
        .take(read_limit)
        .read_to_end(&mut source_bytes)
        .map_err(|error| {
            SourceDiagnostic::loading(origin.clone(), format!("read source {path:?}: {error}"))
        })?;
    let actual_bytes = u64::try_from(source_bytes.len()).map_err(|_| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("source byte length is not representable: {path:?}"),
        )
    })?;
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
