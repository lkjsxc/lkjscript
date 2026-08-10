use std::path::{Path, PathBuf};
use std::time::Instant;

use lkjscript_core::{Error, Result};

use crate::{CompileMetrics, ExecutableProgram, ImportMetrics};

use super::common::{compile_snapshot, compile_snapshot_with_metrics, SnapshotCompileMetrics};
use super::{PackageCompileError, PackageCompileResult};

pub fn compile_path(path: &Path) -> Result<ExecutableProgram> {
    let snapshot = crate::workspace::import_path(path)?;
    compile_snapshot(&snapshot).map_err(crate::CompileSnapshotError::into_core)
}

pub fn compile_package_path(path: &Path) -> PackageCompileResult<ExecutableProgram> {
    let snapshot = crate::workspace::import_package_path(path)?;
    compile_snapshot(&snapshot).map_err(PackageCompileError::from)
}

pub fn compile_path_with_metrics(path: &Path) -> Result<(ExecutableProgram, CompileMetrics)> {
    let total_started = Instant::now();
    let (snapshot, import) = crate::workspace::import_path_with_metrics(path)?;
    let (program, compile) =
        compile_snapshot_with_metrics(&snapshot).map_err(crate::CompileSnapshotError::into_core)?;
    Ok((program, observed_metrics(total_started, import, compile)))
}

pub fn compile_package_path_with_metrics(
    path: &Path,
) -> PackageCompileResult<(ExecutableProgram, CompileMetrics)> {
    let total_started = Instant::now();
    let (snapshot, import) = crate::workspace::import_package_path_with_metrics(path)?;
    let (program, compile) =
        compile_snapshot_with_metrics(&snapshot).map_err(PackageCompileError::from)?;
    Ok((program, observed_metrics(total_started, import, compile)))
}

fn observed_metrics(
    total_started: Instant,
    import: ImportMetrics,
    compile: SnapshotCompileMetrics,
) -> CompileMetrics {
    CompileMetrics {
        total: total_started.elapsed(),
        source_loading: import.source_loading,
        parsing: import.parsing,
        hir_analysis: import.hir_analysis,
        effect_analysis: import.effect_analysis,
        memory_planning: compile.memory_planning,
        ssa_construction: compile.ssa_construction,
        ssa_verification: compile.ssa_verification,
        normalization: compile.normalization,
        bytecode_lowering: compile.bytecode_lowering,
        bytecode_validation: compile.bytecode_validation,
        package_validation: import
            .package_validation
            .saturating_add(compile.package_validation),
        source_files: import.source_files,
    }
}

pub fn compile_path_with_sources(path: &Path) -> Result<(ExecutableProgram, Vec<PathBuf>)> {
    let snapshot = crate::workspace::import_path(path)?;
    let attachments = snapshot
        .attachments()
        .ok_or_else(|| Error::msg("path import omitted source attachments"))?;
    let mut sources = Vec::new();
    sources
        .try_reserve(attachments.files().len())
        .map_err(|_| Error::host("compiled source path allocation failed"))?;
    sources.extend(
        attachments
            .files()
            .iter()
            .map(|source| source.path().to_path_buf()),
    );
    let program = compile_snapshot(&snapshot).map_err(crate::CompileSnapshotError::into_core)?;
    Ok((program, sources))
}
