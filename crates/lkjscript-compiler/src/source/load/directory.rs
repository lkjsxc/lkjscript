use std::fs;
use std::path::Path;

use crate::source::{
    DiagnosticCategory, SourceDiagnostic, SourceResult, SourceSpan,
    FOUNDATION_MAX_SOURCE_TREE_ENTRIES,
};

pub fn validate_source_directory_tree(root: &Path, max: u32) -> SourceResult<()> {
    let mut pending = vec![root.to_path_buf()];
    let mut traversed_entries = 0_u64;
    while let Some(directory) = pending.pop() {
        let entries = source_directory_entries(&directory, max)?;
        traversed_entries = traversed_entries
            .checked_add(u64::try_from(entries.len()).map_err(|_| {
                SourceDiagnostic::loading(
                    super::containment::host_diagnostic_origin(&directory),
                    "source-tree entry count is not representable",
                )
            })?)
            .ok_or_else(|| source_tree_entry_limit_error(&directory, u64::MAX))?;
        if traversed_entries > FOUNDATION_MAX_SOURCE_TREE_ENTRIES {
            return Err(source_tree_entry_limit_error(&directory, traversed_entries));
        }
        for entry in entries.into_iter().rev() {
            let kind = entry.file_type().map_err(|error| {
                SourceDiagnostic::loading(
                    super::containment::host_diagnostic_origin(&entry.path()),
                    format!("inspect source entry {:?}: {error}", entry.path()),
                )
            })?;
            if kind.is_dir() {
                pending.push(entry.path());
            }
        }
    }
    Ok(())
}

fn source_tree_entry_limit_error(path: &Path, attempted: u64) -> SourceDiagnostic {
    SourceDiagnostic::new(
        "LKJ-SRC-LIMIT",
        DiagnosticCategory::ResourceLimit,
        format!(
            concat!(
                "Semantic Source Foundation V1 resource limit: ",
                "category=source-tree-entries; attempted={attempted}; ",
                "limit={limit}"
            ),
            attempted = attempted,
            limit = FOUNDATION_MAX_SOURCE_TREE_ENTRIES
        ),
        super::containment::host_diagnostic_origin(path),
        SourceSpan::zero(),
    )
}

pub(super) fn validate_source_directory(dir: &Path, max: u32) -> SourceResult<()> {
    source_directory_entries(dir, max).map(|_| ())
}

fn source_directory_entries(dir: &Path, max: u32) -> SourceResult<Vec<fs::DirEntry>> {
    let origin = super::containment::host_diagnostic_origin(dir);
    let entries = fs::read_dir(dir).map_err(|error| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("read source directory {}: {error}", dir.display()),
        )
    })?;
    let max_entries = usize::try_from(max).map_err(|_| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("source directory entry limit is not representable: {max}"),
        )
    })?;
    let implementation_max =
        usize::try_from(FOUNDATION_MAX_SOURCE_TREE_ENTRIES).unwrap_or(usize::MAX);
    let effective_max = max_entries.min(implementation_max);
    let mut children = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            SourceDiagnostic::loading(
                origin.clone(),
                format!("read entry in {}: {error}", dir.display()),
            )
        })?;
        if children.len() == effective_max {
            let attempted = u64::try_from(effective_max)
                .unwrap_or(u64::MAX)
                .saturating_add(1);
            if effective_max < max_entries {
                return Err(source_tree_entry_limit_error(dir, attempted));
            }
            return Err(SourceDiagnostic::new(
                "LKJ-SRC-LIMIT",
                DiagnosticCategory::ResourceLimit,
                format!(
                    "lkjscript source directory {} has at least {attempted} entries (max {max}); split it",
                    dir.display()
                ),
                origin,
                SourceSpan::zero(),
            ));
        }
        children.push(entry);
    }
    children.sort_by_key(fs::DirEntry::file_name);
    Ok(children)
}
