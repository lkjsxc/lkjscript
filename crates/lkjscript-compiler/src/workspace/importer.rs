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
    import_path_in_namespace(path, WorkspaceNamespace::fresh()?, false)
}

pub(crate) fn import_package_path(path: &Path) -> Result<WorkspaceSnapshot> {
    import_package_path_with_metrics(path).map(|(snapshot, _)| snapshot)
}

pub(crate) fn import_package_path_with_metrics(
    path: &Path,
) -> Result<(WorkspaceSnapshot, ImportMetrics)> {
    import_path_in_namespace(path, WorkspaceNamespace::fresh()?, true)
}

pub fn import_source(source: &str, path: &str) -> Result<WorkspaceSnapshot> {
    import_source_in_namespace(source, path, WorkspaceNamespace::fresh()?)
        .map(|(snapshot, _)| snapshot)
}

fn import_path_in_namespace(
    path: &Path,
    namespace: WorkspaceNamespace,
    package_required: bool,
) -> Result<(WorkspaceSnapshot, ImportMetrics)> {
    crate::ensure_source_path(path)?;
    let package_started = Instant::now();
    let package = crate::package::verify_for_compilation(path)?;
    let mut package_validation = package_started.elapsed();
    if package_required && package.is_none() {
        return Err(Error::msg(format!(
            "no {} contains {}",
            crate::package::MANIFEST_FILE,
            path.display()
        )));
    }
    let (source_tree, loading) = crate::source::load_with_metrics(path)
        .map_err(crate::source::SourceDiagnostic::into_core)?;
    if let Some(verified) = &package {
        let locked_sources_started = Instant::now();
        crate::package::verify_loaded_sources(verified, &source_tree)?;
        package_validation = package_validation.saturating_add(locked_sources_started.elapsed());
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
        Some(verified) => {
            let capture_started = Instant::now();
            let captured = crate::package::capture_compilation(&verified)?;
            package_validation = package_validation.saturating_add(capture_started.elapsed());
            CapturedCompilationProvenance::Locked(Arc::new(captured))
        }
        None => CapturedCompilationProvenance::Development(development_source),
    };
    let snapshot = WorkspaceSnapshot::new(namespace, hir, provenance, Some(attachments))?;
    Ok((
        snapshot,
        ImportMetrics {
            source_loading: loading.source_loading,
            parsing: loading.parsing,
            hir_analysis,
            effect_analysis,
            package_validation,
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
        CapturedCompilationProvenance::Development(source_identity),
        Some(attachments),
    )?;
    Ok((
        snapshot,
        ImportMetrics {
            source_loading: Default::default(),
            parsing,
            hir_analysis,
            effect_analysis,
            package_validation: Default::default(),
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
