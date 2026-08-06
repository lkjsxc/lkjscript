use std::path::{Path, PathBuf};
use std::time::Duration;

use lkjscript_core::{Limits, Result};

use crate::source::{
    load as loader, parse, validate as authority, SourceDiagnostic, SourceFoundationBudget,
    SourceOrigin, SourceResult,
};

use super::ValidatedSourceTree;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct LoadMetrics {
    pub(crate) source_loading: Duration,
    pub(crate) parsing: Duration,
}

/// Validate one canonical relative in-memory source unit.
pub fn validate(
    source: &str,
    logical_path: &str,
    limits: &Limits,
) -> SourceResult<ValidatedSourceTree> {
    let origin = authority::validate_logical_source_path(logical_path)?;
    validate_one_source(source, PathBuf::from(logical_path), origin, limits)
}

fn validate_one_source(
    source: &str,
    path: PathBuf,
    origin: SourceOrigin,
    limits: &Limits,
) -> SourceResult<ValidatedSourceTree> {
    let source_len = u64::try_from(source.len()).map_err(|_| {
        authority::foundation_resource_error(
            origin.clone(),
            "source-file-bytes",
            u64::MAX,
            crate::source::FOUNDATION_MAX_SOURCE_FILE_BYTES,
        )
    })?;
    let mut budget = SourceFoundationBudget::default();
    budget.check_metadata(&origin, source_len)?;
    budget.record_read(&origin, source_len)?;
    let parsed = parse::parse_file(source, origin.clone(), path.clone(), limits)?;
    authority::finish_tree(path, origin, vec![parsed])
}

/// Load, contain, parse, and validate a complete import closure.
///
/// Files are returned in deterministic dependency-first DFS order; imports in
/// each file are visited in source order. Loading uses an explicit stack.
pub fn load(path: &Path, limits: &Limits) -> SourceResult<ValidatedSourceTree> {
    loader::load_with_metrics(path, limits).map(|(tree, _)| tree)
}

pub(crate) fn load_with_metrics(
    path: &Path,
    limits: &Limits,
) -> SourceResult<(ValidatedSourceTree, LoadMetrics)> {
    loader::load_with_metrics(path, limits)
}

pub(crate) fn load_for_protocol(
    path: &Path,
    limits: &Limits,
    max_source_bytes: u64,
    max_source_units: u64,
) -> SourceResult<ValidatedSourceTree> {
    let budget = SourceFoundationBudget::with_limits(max_source_units, max_source_bytes);
    loader::load_with_budget(path, limits, budget).map(|(tree, _)| tree)
}

pub(crate) fn validate_for_compiler(
    source: &str,
    logical_path: &str,
    limits: &Limits,
) -> Result<ValidatedSourceTree> {
    validate(source, logical_path, limits).map_err(SourceDiagnostic::into_core)
}

pub(crate) fn load_for_compiler(path: &Path, limits: &Limits) -> Result<ValidatedSourceTree> {
    load(path, limits).map_err(SourceDiagnostic::into_core)
}

pub(crate) fn ensure_source_path_for_compiler(path: &Path) -> Result<()> {
    loader::ensure_source_path(path).map_err(SourceDiagnostic::into_core)
}

pub(crate) fn rebuild_staged_sources(
    sources: &[(PathBuf, SourceOrigin, String)],
    root: PathBuf,
    root_origin: SourceOrigin,
    limits: &Limits,
) -> SourceResult<ValidatedSourceTree> {
    let mut parsed = Vec::with_capacity(sources.len());
    for (path, origin, source) in sources {
        parsed.push(parse::parse_file(
            source,
            origin.clone(),
            path.clone(),
            limits,
        )?);
    }
    authority::finish_tree(root, root_origin, parsed)
}

#[cfg(test)]
pub(crate) fn validate_source_set_for_analysis(
    files: &[(&str, &str)],
    root: &str,
    limits: &Limits,
) -> Result<ValidatedSourceTree> {
    let mut parsed = Vec::with_capacity(files.len());
    for (path, source) in files {
        let origin = SourceOrigin::in_memory(path);
        parsed.push(
            parse::parse_file(source, origin, PathBuf::from(path), limits)
                .map_err(SourceDiagnostic::into_core)?,
        );
    }
    let root_origin = SourceOrigin::in_memory(root);
    authority::finish_tree(PathBuf::from(root), root_origin, parsed)
        .map_err(SourceDiagnostic::into_core)
}
