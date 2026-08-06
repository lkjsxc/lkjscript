use std::collections::{HashMap, HashSet};

use crate::semantic::codec::error;
use crate::semantic::schema::{ChangedSource, FilePrecondition, ProtocolError, ProtocolErrorCode};
use crate::semantic::transaction::StagedSource;
use crate::source::{SourceOrigin, ValidatedSourceTree};

pub(super) fn changed_sources(
    tree: &ValidatedSourceTree,
    rebuilt: &[(std::path::PathBuf, SourceOrigin, String)],
) -> Result<(Vec<StagedSource>, Vec<ChangedSource>), ProtocolError> {
    let old: HashMap<_, _> = tree
        .files()
        .iter()
        .map(|file| (file.origin.logical_path.as_str(), file))
        .collect();
    let mut sources = Vec::new();
    let mut changes = Vec::new();
    for (path, origin, source) in rebuilt {
        let original = old.get(origin.logical_path.as_str()).ok_or_else(|| {
            error(
                ProtocolErrorCode::ValidationFailed,
                "rebuilt source origin changed",
            )
        })?;
        let new_bytes = source.as_bytes();
        let new_len = checked_len(new_bytes.len(), "rebuilt source")?;
        let new_hash = lkjscript_core::sha256(new_bytes);
        if original.exact_source_sha256 == new_hash && original.exact_source_len == new_len {
            continue;
        }
        let old_bytes = std::fs::read(path).map_err(|failure| {
            error(
                ProtocolErrorCode::PublicationFailed,
                format!(
                    "read publication precondition {}: {failure}",
                    path.display()
                ),
            )
        })?;
        if lkjscript_core::sha256(&old_bytes) != original.exact_source_sha256
            || checked_len(old_bytes.len(), "published source")? != original.exact_source_len
        {
            return Err(error(
                ProtocolErrorCode::PreconditionFailed,
                format!("source {} changed after snapshot", origin.logical_path),
            ));
        }
        changes.push(ChangedSource {
            path: origin.logical_path.clone(),
            old_sha256: crate::semantic::tree::hex(&original.exact_source_sha256),
            new_sha256: crate::semantic::tree::hex(&new_hash),
            bytes: new_len,
        });
        sources.push(StagedSource {
            logical_path: origin.logical_path.clone(),
            host_path: path.clone(),
            old_bytes,
            new_bytes: new_bytes.to_vec(),
        });
    }
    sources.sort_by(|a, b| a.logical_path.cmp(&b.logical_path));
    changes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok((sources, changes))
}

fn checked_len(len: usize, context: &str) -> Result<u64, ProtocolError> {
    u64::try_from(len).map_err(|_| {
        error(
            ProtocolErrorCode::PublicationFailed,
            format!("{context} length exceeds the u64 metadata representation"),
        )
    })
}

pub(super) fn check_preconditions(
    tree: &ValidatedSourceTree,
    changes: &[ChangedSource],
    supplied: &[FilePrecondition],
) -> Result<(), ProtocolError> {
    if changes.is_empty() {
        return Err(error(
            ProtocolErrorCode::InvalidOperation,
            "transaction makes no semantic change",
        ));
    }
    let mut seen = HashSet::new();
    for expected in supplied {
        let file = tree
            .files()
            .iter()
            .find(|file| file.origin.logical_path == expected.path)
            .ok_or_else(|| {
                error(
                    ProtocolErrorCode::PreconditionFailed,
                    format!("unknown file precondition for {}", expected.path),
                )
            })?;
        if !seen.insert(&expected.path)
            || expected.bytes != file.exact_source_len
            || expected.sha256 != crate::semantic::tree::hex(&file.exact_source_sha256)
        {
            return Err(error(
                ProtocolErrorCode::PreconditionFailed,
                format!("file precondition failed for {}", expected.path),
            ));
        }
    }
    for change in changes {
        if !seen.contains(&change.path) {
            return Err(error(
                ProtocolErrorCode::PreconditionFailed,
                format!("missing file precondition for {}", change.path),
            ));
        }
    }
    Ok(())
}
