use std::path::{Path, PathBuf};
use std::time::Instant;

use lkjscript_core::{validate_chunk, Result, ValidationPolicy};

use crate::analyze::{analyze_program, analyze_program_without_effects};
use crate::codegen::compile_program;
use crate::source::{load_for_compiler, load_with_metrics};
use crate::ssa::lower_program_with_metrics;
use crate::{CompileMetrics, ExecutableProgram};

use super::common::{checked_memory_inventory, compile_analyzed};

pub fn compile_path(path: &Path) -> Result<ExecutableProgram> {
    compile_path_with_sources(path).map(|(program, _)| program)
}

pub fn compile_path_with_metrics(path: &Path) -> Result<(ExecutableProgram, CompileMetrics)> {
    let total_started = Instant::now();
    crate::ensure_source_path(path)?;
    let package = crate::package::verify_for_compilation(path)?;
    let (program, loading) =
        load_with_metrics(path).map_err(crate::source::SourceDiagnostic::into_core)?;
    if let Some(root) = &package {
        crate::package::verify_loaded_sources(root, path, &program)?;
    }
    let development_source = program
        .files()
        .first()
        .ok_or_else(|| lkjscript_core::Error::msg("compiled source closure is empty"))?
        .exact_source_sha256;
    let source_files = program.files().len();
    let projection = program
        .module_scoped_projection()
        .map_err(crate::source::SourceDiagnostic::into_core)?;
    let hir_started = Instant::now();
    let mut analyzed = analyze_program_without_effects(&projection)?;
    let hir_analysis = hir_started.elapsed();
    let effects_started = Instant::now();
    crate::effects::infer(&mut analyzed);
    let effect_analysis = effects_started.elapsed();
    let memory_started = Instant::now();
    let memory_verified = crate::memory_plan::verify_hir_memory(&analyzed)?;
    let memory_planning = memory_started.elapsed();
    let (ssa, ssa_metrics) = lower_program_with_metrics(&memory_verified)?;
    let memory_plan = memory_verified.plan().clone();
    let inventory_started = Instant::now();
    let memory_inventory = checked_memory_inventory(&ssa)?;
    let memory_inventory_time = inventory_started.elapsed();
    let bytecode_started = Instant::now();
    let (chunk, bytecode_links) = compile_program(&ssa)?;
    let bytecode_lowering = bytecode_started.elapsed();
    let validation_started = Instant::now();
    let bytecode = validate_chunk(chunk, ValidationPolicy::Unrestricted)?;
    let bytecode_validation = validation_started.elapsed();
    let preparation_started = Instant::now();
    let provenance = match package {
        Some(root) => crate::package::program::locked(crate::package::prepared_facts(
            &root,
            path,
            &memory_plan,
        )?),
        None => crate::package::program::development(
            development_source,
            &path.to_string_lossy(),
            &memory_plan,
        ),
    };
    let (prepared, ssa, bytecode) =
        crate::package::program::bind(ssa, bytecode, &memory_plan, provenance)?;
    let preparation = preparation_started.elapsed();
    let executable = ExecutableProgram {
        prepared,
        bytecode,
        ssa,
        memory_plan,
        memory_inventory,
        bytecode_links,
    };
    Ok((
        executable,
        CompileMetrics {
            total: total_started.elapsed(),
            source_loading: loading.source_loading,
            parsing: loading.parsing,
            hir_analysis,
            effect_analysis,
            memory_planning,
            ssa_construction: ssa_metrics.construction,
            ssa_verification: ssa_metrics.verification,
            normalization: ssa_metrics.normalization,
            memory_inventory: memory_inventory_time,
            bytecode_lowering,
            bytecode_validation,
            preparation,
            source_files,
        },
    ))
}

pub fn compile_path_with_sources(path: &Path) -> Result<(ExecutableProgram, Vec<PathBuf>)> {
    crate::ensure_source_path(path)?;
    let package = crate::package::verify_for_compilation(path)?;
    let program = load_for_compiler(path)?;
    if let Some(root) = &package {
        crate::package::verify_loaded_sources(root, path, &program)?;
    }
    let development_source = program
        .files()
        .first()
        .ok_or_else(|| lkjscript_core::Error::msg("compiled source closure is empty"))?
        .exact_source_sha256;
    let sources = program
        .files()
        .iter()
        .map(|source| source.path.clone())
        .collect();
    let projection = program
        .module_scoped_projection()
        .map_err(crate::source::SourceDiagnostic::into_core)?;
    let analyzed = analyze_program(&projection)?;
    let executable = compile_analyzed(&analyzed, |plan| {
        Ok(match package {
            Some(root) => {
                crate::package::program::locked(crate::package::prepared_facts(&root, path, plan)?)
            }
            None => crate::package::program::development(
                development_source,
                &path.to_string_lossy(),
                plan,
            ),
        })
    })?;
    Ok((executable, sources))
}
