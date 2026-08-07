use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use lkjscript_core::{Error, Result};

use super::{
    CapturedCompilationProvenance, ImportMetrics, PresentationAttachments, SourceAttachment,
    WorkspaceNamespace, WorkspaceSnapshot,
};

pub fn import_path(path: &Path) -> Result<WorkspaceSnapshot> {
    import_path_with_metrics(path).map(|(snapshot, _)| snapshot)
}

pub fn import_path_with_metrics(path: &Path) -> Result<(WorkspaceSnapshot, ImportMetrics)> {
    import_path_in_namespace(path, WorkspaceNamespace::fresh()?)
}

pub fn import_source(source: &str, path: &str) -> Result<WorkspaceSnapshot> {
    import_source_in_namespace(source, path, WorkspaceNamespace::fresh()?)
        .map(|(snapshot, _)| snapshot)
}

fn import_path_in_namespace(
    path: &Path,
    namespace: WorkspaceNamespace,
) -> Result<(WorkspaceSnapshot, ImportMetrics)> {
    crate::ensure_source_path(path)?;
    let package = crate::package::verify_for_compilation(path)?;
    let (source_tree, loading) = crate::source::load_with_metrics(path)
        .map_err(crate::source::SourceDiagnostic::into_core)?;
    if let Some(root) = &package {
        crate::package::verify_loaded_sources(root, path, &source_tree)?;
    }
    let development_source = source_tree
        .files()
        .first()
        .ok_or_else(|| Error::msg("imported source closure is empty"))?
        .exact_source_sha256;
    let attachments = attachments(&source_tree)?;
    let source_files = attachments.files().len();
    let projection = source_tree
        .module_scoped_projection()
        .map_err(crate::source::SourceDiagnostic::into_core)?;

    let hir_started = Instant::now();
    let mut hir = crate::analyze::analyze_program_without_effects(&projection)?;
    let hir_analysis = hir_started.elapsed();
    let effects_started = Instant::now();
    crate::effects::infer(&mut hir);
    crate::analyze::verify_match_plans(&hir)?;
    let effect_analysis = effects_started.elapsed();

    let provenance = match package {
        Some(root) => CapturedCompilationProvenance::Locked(Arc::new(
            crate::package::capture_preparation(&root, path)?,
        )),
        None => CapturedCompilationProvenance::Development {
            source_identity: development_source,
            path: Arc::from(path.to_string_lossy().as_ref()),
        },
    };
    let snapshot = WorkspaceSnapshot::new(namespace, hir, provenance, Some(attachments))?;
    Ok((
        snapshot,
        ImportMetrics {
            source_loading: loading.source_loading,
            parsing: loading.parsing,
            hir_analysis,
            effect_analysis,
            source_files,
        },
    ))
}

fn import_source_in_namespace(
    source: &str,
    path: &str,
    namespace: WorkspaceNamespace,
) -> Result<(WorkspaceSnapshot, ImportMetrics)> {
    crate::ensure_source_path(Path::new(path))?;
    let parsing_started = Instant::now();
    let source_tree = crate::source::validate_for_compiler(source, path)?;
    let parsing = parsing_started.elapsed();
    let source_file = source_tree
        .files()
        .first()
        .ok_or_else(|| Error::msg("imported development source closure is empty"))?;
    let source_identity = source_file.exact_source_sha256;
    let attachments = attachments(&source_tree)?;
    let source_files = attachments.files().len();
    let projection = source_tree
        .module_scoped_projection()
        .map_err(crate::source::SourceDiagnostic::into_core)?;
    let hir_started = Instant::now();
    let mut hir = crate::analyze::analyze_program_without_effects(&projection)?;
    let hir_analysis = hir_started.elapsed();
    let effects_started = Instant::now();
    crate::effects::infer(&mut hir);
    crate::analyze::verify_match_plans(&hir)?;
    let effect_analysis = effects_started.elapsed();
    let snapshot = WorkspaceSnapshot::new(
        namespace,
        hir,
        CapturedCompilationProvenance::Development {
            source_identity,
            path: Arc::from(path),
        },
        Some(attachments),
    )?;
    Ok((
        snapshot,
        ImportMetrics {
            source_loading: Default::default(),
            parsing,
            hir_analysis,
            effect_analysis,
            source_files,
        },
    ))
}

fn attachments(
    source_tree: &crate::source::ValidatedSourceTree,
) -> Result<PresentationAttachments> {
    let mut files = Vec::new();
    files
        .try_reserve(source_tree.files().len())
        .map_err(|_| Error::host("workspace source attachment allocation failed"))?;
    for file in source_tree.files() {
        files.push(SourceAttachment::new(
            file.path.clone(),
            file.exact_source_len,
            file.exact_source_sha256,
        ));
    }
    Ok(PresentationAttachments::new(files))
}

#[cfg(test)]
pub(super) fn import_source_with_namespace(
    source: &str,
    path: &str,
    namespace: WorkspaceNamespace,
) -> Result<WorkspaceSnapshot> {
    import_source_in_namespace(source, path, namespace).map(|(snapshot, _)| snapshot)
}
