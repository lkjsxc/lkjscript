use std::path::{Path, PathBuf};
use std::time::Duration;

use lkjscript_core::Result;

use crate::source::{
    load as loader, parse, validate as authority, SourceDiagnostic, SourceOrigin, SourceResult,
};

use super::ValidatedSourceTree;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct LoadMetrics {
    pub(crate) source_loading: Duration,
    pub(crate) parsing: Duration,
}

/// Validate one canonical relative in-memory source unit.
pub fn validate(source: &str, logical_path: &str) -> SourceResult<ValidatedSourceTree> {
    let origin = authority::validate_logical_source_path(logical_path)?;
    validate_one_source(source, PathBuf::from(logical_path), origin)
}

fn validate_one_source(
    source: &str,
    path: PathBuf,
    origin: SourceOrigin,
) -> SourceResult<ValidatedSourceTree> {
    let parsed = parse::parse_file(source, origin.clone(), path.clone())?;
    authority::finish_tree(path, origin, vec![parsed])
}

/// Load, contain, parse, and validate a complete import closure.
///
/// Files are returned in deterministic dependency-first DFS order; imports in
/// each file are visited in source order. Loading uses an explicit stack.
pub fn load(path: &Path) -> SourceResult<ValidatedSourceTree> {
    loader::load_with_metrics(path).map(|(tree, _)| tree)
}

pub(crate) fn load_with_metrics(path: &Path) -> SourceResult<(ValidatedSourceTree, LoadMetrics)> {
    loader::load_with_metrics(path)
}

pub(crate) fn validate_for_compiler(
    source: &str,
    logical_path: &str,
) -> Result<ValidatedSourceTree> {
    validate(source, logical_path).map_err(SourceDiagnostic::into_core)
}

pub(crate) fn ensure_source_path_for_compiler(path: &Path) -> Result<()> {
    loader::ensure_source_path(path).map_err(SourceDiagnostic::into_core)
}

#[cfg(test)]
pub(crate) fn validate_source_set_for_analysis(
    files: &[(&str, &str)],
    root: &str,
) -> Result<ValidatedSourceTree> {
    let mut parsed = Vec::with_capacity(files.len());
    for (path, source) in files {
        let origin = SourceOrigin::in_memory(path);
        parsed.push(
            parse::parse_file(source, origin, PathBuf::from(path))
                .map_err(SourceDiagnostic::into_core)?,
        );
    }
    let root_origin = SourceOrigin::in_memory(root);
    authority::finish_tree(PathBuf::from(root), root_origin, parsed)
        .map_err(SourceDiagnostic::into_core)
}
