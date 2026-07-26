use std::path::{Path, PathBuf};
use std::time::Instant;

use lkjscript_core::{
    validate_chunk, BudgetLedger, Error, Limits, ResourceCategory, ResourceProfile, Result,
};

use crate::analyze::{analyze_program_with_budget, analyze_program_without_effects_with_budget};
use crate::codegen::compile_program;
use crate::source::{
    load_for_compiler_with_budget, load_with_metrics_and_budget, validate_for_compiler_with_budget,
};
use crate::ssa::{lower_program_with_budget, lower_program_with_metrics_and_budget};
use crate::{CompileMetrics, ExecutableProgram};

pub fn compile_path(path: &Path, limits: &Limits) -> Result<ExecutableProgram> {
    compile_path_with_profile(path, limits, ResourceProfile::default())
}

pub fn compile_path_with_profile(
    path: &Path,
    limits: &Limits,
    profile: ResourceProfile,
) -> Result<ExecutableProgram> {
    compile_path_with_sources_and_profile(path, limits, profile).map(|(program, _)| program)
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
    let total_started = Instant::now();
    let mut ledger = BudgetLedger::new(profile);
    let result = (|| {
        crate::ensure_source_path(path)?;
        let (program, loading) = load_with_metrics_and_budget(path, limits, &mut ledger)?;
        crate::source::require_edition2_for_compiler(&program)?;
        let source_files = program.files().len();

        let hir_started = Instant::now();
        let mut analyzed = analyze_program_without_effects_with_budget(&program, &mut ledger)?;
        let hir_analysis = hir_started.elapsed();
        let effects_started = Instant::now();
        crate::effects::infer(&mut analyzed);
        let effect_analysis = effects_started.elapsed();

        let (ssa, ssa_metrics) = lower_program_with_metrics_and_budget(&analyzed, &mut ledger)?;
        let bytecode_started = Instant::now();
        let (chunk, bytecode_links) = compile_program(&ssa)?;
        let bytecode_lowering = bytecode_started.elapsed();
        let validation_started = Instant::now();
        let bytecode = validate_chunk(chunk, &limits.validation)?;
        let bytecode_validation = validation_started.elapsed();
        let identity = profile.identity();
        let executable = ExecutableProgram {
            bytecode,
            ssa,
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
    finish(result, &mut ledger)
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
    let result = (|| {
        crate::ensure_source_path(path)?;
        let program = load_for_compiler_with_budget(path, limits, &mut ledger)?;
        crate::source::require_edition2_for_compiler(&program)?;
        let sources = program
            .files()
            .iter()
            .map(|source| source.path.clone())
            .collect();
        let analyzed = analyze_program_with_budget(&program, &mut ledger)?;
        let executable = compile_analyzed(&analyzed, limits, profile, &mut ledger)?;
        Ok((executable, sources))
    })();
    finish(result, &mut ledger)
}

pub fn compile_source(source: &str, path: &str, limits: &Limits) -> Result<ExecutableProgram> {
    compile_source_with_profile(source, path, limits, ResourceProfile::default())
}

pub fn compile_source_with_profile(
    source: &str,
    path: &str,
    limits: &Limits,
    profile: ResourceProfile,
) -> Result<ExecutableProgram> {
    let mut ledger = BudgetLedger::new(profile);
    let result = (|| {
        crate::ensure_source_path(Path::new(path))?;
        let program = validate_for_compiler_with_budget(source, path, limits, &mut ledger)?;
        crate::source::require_edition2_for_compiler(&program)?;
        let analyzed = analyze_program_with_budget(&program, &mut ledger)?;
        compile_analyzed(&analyzed, limits, profile, &mut ledger)
    })();
    finish(result, &mut ledger)
}

fn compile_analyzed(
    analyzed: &crate::hir::Program,
    limits: &Limits,
    profile: ResourceProfile,
    ledger: &mut BudgetLedger,
) -> Result<ExecutableProgram> {
    let ssa = lower_program_with_budget(analyzed, ledger)?;
    let (chunk, bytecode_links) = compile_program(&ssa)?;
    let bytecode = validate_chunk(chunk, &limits.validation)?;
    Ok(ExecutableProgram {
        bytecode,
        ssa,
        bytecode_links,
        profile: profile.identity(),
    })
}

pub fn validate_source(source: &str, path: &str, limits: &Limits) -> Result<()> {
    validate_source_with_profile(source, path, limits, ResourceProfile::default())
}

pub fn validate_source_with_profile(
    source: &str,
    path: &str,
    limits: &Limits,
    profile: ResourceProfile,
) -> Result<()> {
    let mut ledger = BudgetLedger::new(profile);
    let result = (|| {
        crate::ensure_source_path(Path::new(path))?;
        validate_for_compiler_with_budget(source, path, limits, &mut ledger).map(|_| ())
    })();
    finish(result, &mut ledger)
}

fn finish<T>(result: Result<T>, ledger: &mut BudgetLedger) -> Result<T> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            ledger
                .charge(ResourceCategory::Diagnostics, 1)
                .map_err(Error::compiler_resource)?;
            Err(error)
        }
    }
}

pub fn validate_source_tree(root: &Path, limits: &Limits) -> Result<()> {
    crate::source::validate_source_directory_tree(root, limits.max_dir_children)
        .map_err(crate::source::SourceDiagnostic::into_core)
}
