use std::path::Path;

use lkjscript_core::{Error, Result};

pub(super) struct ModuleAnalysis {
    pub(super) logical_id: String,
    pub(super) source_sha256: [u8; 32],
    pub(super) source: crate::source::ValidatedSourceTree,
    pub(super) hir: crate::hir::Program,
    pub(super) memory_plan: crate::memory_plan::HirMemoryPlan,
}

pub(super) fn module(root: &Path, id: &str) -> Result<ModuleAnalysis> {
    let source = crate::source::load(&root.join(id), &lkjscript_core::Limits::default())
        .map_err(crate::source::SourceDiagnostic::into_core)?;
    let root_file = source
        .files()
        .iter()
        .find(|file| file.path == source.root_path())
        .ok_or_else(|| Error::msg(format!("loaded package module is absent: {id}")))?;
    let logical_id = root_file.origin.logical_path().into();
    let source_sha256 = root_file.exact_source_sha256;
    let projection = source
        .module_scoped_projection()
        .map_err(crate::source::SourceDiagnostic::into_core)?;
    let hir = crate::analyze::analyze_interface_program(&projection)?;
    let memory_plan = crate::memory_plan::verify_hir_memory(&hir)?.plan().clone();
    Ok(ModuleAnalysis {
        logical_id,
        source_sha256,
        source,
        hir,
        memory_plan,
    })
}
