use std::path::{Path, PathBuf};
use std::time::Instant;

use lkjscript_core::{validate_chunk, BudgetLedger, Limits, ResourceProfile, Result};

use crate::analyze::{analyze_program_with_budget, analyze_program_without_effects_with_budget};
use crate::codegen::compile_program;
use crate::source::{load_for_compiler_with_budget, load_with_metrics_and_budget};
use crate::ssa::lower_program_with_metrics_and_budget;
use crate::{CompileMetrics, ExecutableProgram};

use super::common::{checked_memory_inventory, compile_analyzed, finish};

pub fn compile_path(path: &Path, limits: &Limits) -> Result<ExecutableProgram> {
    compile_path_with_profile(path, limits, ResourceProfile::default())
}

pub fn compile_path_with_profile(
    path: &Path,
    limits: &Limits,
    profile: ResourceProfile,
) -> Result<ExecutableProgram> {
    let mut ledger = BudgetLedger::new(profile);
    compile_path_with_ledger(path, limits, &mut ledger)
}

pub fn compile_path_with_ledger(
    path: &Path,
    limits: &Limits,
    ledger: &mut BudgetLedger,
) -> Result<ExecutableProgram> {
    compile_path_with_sources_and_ledger(path, limits, ledger).map(|(program, _)| program)
}

pub fn compile_path_with_metrics(
    path: &Path,
    limits: &Limits,
) -> Result<(ExecutableProgram, CompileMetrics)> {
    compile_path_with_profile_and_metrics(path, limits, ResourceProfile::default())
}

pub fn compile_path_with_profile_and_metrics(
    path: &Path,
    limits: &Limits,
    profile: ResourceProfile,
) -> Result<(ExecutableProgram, CompileMetrics)> {
    let mut ledger = BudgetLedger::new(profile);
    compile_path_with_metrics_and_ledger(path, limits, &mut ledger)
}

pub fn compile_path_with_metrics_and_ledger(
    path: &Path,
    limits: &Limits,
    ledger: &mut BudgetLedger,
) -> Result<(ExecutableProgram, CompileMetrics)> {
    let total_started = Instant::now();
    let result = (|| {
        crate::ensure_source_path(path)?;
        let (program, loading) = load_with_metrics_and_budget(path, limits, ledger)?;
        let source_files = program.files().len();
        let projection = program
            .module_scoped_projection()
            .map_err(crate::source::SourceDiagnostic::into_core)?;
        let hir_started = Instant::now();
        let mut analyzed = analyze_program_without_effects_with_budget(&projection, ledger)?;
        let hir_analysis = hir_started.elapsed();
        let effects_started = Instant::now();
        crate::effects::infer(&mut analyzed);
        let effect_analysis = effects_started.elapsed();
        let memory_verified = crate::memory_plan::verify_hir_memory(&analyzed)?;
        let (ssa, ssa_metrics) = lower_program_with_metrics_and_budget(&memory_verified, ledger)?;
        let memory_plan = memory_verified.plan().clone();
        let memory_inventory = checked_memory_inventory(&ssa)?;
        let bytecode_started = Instant::now();
        let (chunk, bytecode_links) = compile_program(&ssa)?;
        let bytecode_lowering = bytecode_started.elapsed();
        let validation_started = Instant::now();
        let bytecode = validate_chunk(chunk, &limits.validation)?;
        let bytecode_validation = validation_started.elapsed();
        let identity = ledger.profile().identity();
        let executable = ExecutableProgram {
            bytecode,
            ssa,
            memory_plan,
            memory_inventory,
            bytecode_links,
            profile: identity,
        };
        Ok((
            executable,
            CompileMetrics {
                total: total_started.elapsed(),
                source_loading: loading.source_loading,
                parsing: loading.parsing,
                hir_analysis,
                effect_analysis,
                ssa_construction: ssa_metrics.construction,
                ssa_verification: ssa_metrics.verification,
                normalization: ssa_metrics.normalization,
                bytecode_lowering,
                bytecode_validation,
                source_files,
                profile: identity,
                resources: ledger.usage(),
            },
        ))
    })();
    finish(result, ledger)
}

pub fn compile_path_with_sources(
    path: &Path,
    limits: &Limits,
) -> Result<(ExecutableProgram, Vec<PathBuf>)> {
    compile_path_with_sources_and_profile(path, limits, ResourceProfile::default())
}

pub fn compile_path_with_sources_and_profile(
    path: &Path,
    limits: &Limits,
    profile: ResourceProfile,
) -> Result<(ExecutableProgram, Vec<PathBuf>)> {
    let mut ledger = BudgetLedger::new(profile);
    compile_path_with_sources_and_ledger(path, limits, &mut ledger)
}

pub fn compile_path_with_sources_and_ledger(
    path: &Path,
    limits: &Limits,
    ledger: &mut BudgetLedger,
) -> Result<(ExecutableProgram, Vec<PathBuf>)> {
    let result = (|| {
        crate::ensure_source_path(path)?;
        let program = load_for_compiler_with_budget(path, limits, ledger)?;
        let sources = program
            .files()
            .iter()
            .map(|source| source.path.clone())
            .collect();
        let projection = program
            .module_scoped_projection()
            .map_err(crate::source::SourceDiagnostic::into_core)?;
        let analyzed = analyze_program_with_budget(&projection, ledger)?;
        let executable = compile_analyzed(&analyzed, limits, ledger)?;
        Ok((executable, sources))
    })();
    finish(result, ledger)
}
