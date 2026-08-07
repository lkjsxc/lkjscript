mod containment;
mod frame;
mod imports;
mod read;
mod traversal;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::source::{
    api::LoadMetrics, validate::finish_tree, SourceFile, SourceOrigin, SourceResult, SourceSpan,
    ValidatedSourceTree,
};

pub(crate) use containment::ensure_source_path;
#[cfg(test)]
pub(crate) use containment::source_origin;
#[cfg(all(test, target_os = "linux"))]
pub(crate) use containment::{open_source_file, opened_source_path};
#[cfg(test)]
pub(crate) use imports::resolve_for_test;
#[cfg(test)]
pub(crate) use read::read_source_bytes;

struct LoadState<'a> {
    package_root: &'a Path,
    installed_root: Option<&'a Path>,
    loading: HashSet<PathBuf>,
    done: HashSet<PathBuf>,
    files: Vec<SourceFile>,
    completed_source_bytes: u64,
    metrics: LoadMetrics,
}

#[derive(Clone)]
struct PendingImport {
    spec: String,
    span: SourceSpan,
}

struct LoadFrame {
    canonical: PathBuf,
    parent: PathBuf,
    parsed: SourceFile,
    imports: Vec<PendingImport>,
    next_import: usize,
    reached_by: Option<(SourceOrigin, SourceSpan)>,
}

pub(crate) fn load_with_metrics(path: &Path) -> SourceResult<(ValidatedSourceTree, LoadMetrics)> {
    ensure_source_path(path)?;
    let loading_started = Instant::now();
    let entry = path.canonicalize().map_err(|error| {
        crate::source::SourceDiagnostic::loading(
            containment::host_diagnostic_origin(path),
            format!("cannot resolve requested source {path:?}: {error}"),
        )
    })?;
    let package_root = containment::find_package_root(&entry);
    let mut state = LoadState {
        package_root: &package_root,
        installed_root: None,
        loading: HashSet::new(),
        done: HashSet::new(),
        files: Vec::new(),
        completed_source_bytes: 0,
        metrics: LoadMetrics {
            source_loading: loading_started.elapsed(),
            ..LoadMetrics::default()
        },
    };
    let (root_path, root_origin) = traversal::load_files_depth_first(&entry, &mut state)?;
    let tree = finish_tree(root_path, root_origin, state.files)?;
    Ok((tree, state.metrics))
}
