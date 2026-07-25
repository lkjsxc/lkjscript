//! Compile canonical line-oriented `.lkjscript` source into verified typed SSA
//! and validated reference bytecode.

mod analyze;
mod codegen;
mod effects;
mod hir;
mod operation;
mod ownership;
pub mod source;
mod ssa;
mod types;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use lkjscript_core::{validate_chunk, Limits, Result, ValidatedChunk};
use lkjscript_ir::{BytecodeLinkMetadata, VerifiedProgram};

use crate::analyze::{analyze_program, analyze_program_without_effects};
use crate::codegen::compile_program;
use crate::source::{load_for_compiler, validate_for_compiler};
use crate::ssa::{lower_program, lower_program_with_metrics};

pub const SOURCE_EXTENSION: &str = "lkjscript";

/// Monotonic direct phase timings for one source compilation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompileMetrics {
    pub total: Duration,
    pub source_loading: Duration,
    pub parsing: Duration,
    pub hir_analysis: Duration,
    pub effect_analysis: Duration,
    pub ssa_construction: Duration,
    pub ssa_verification: Duration,
    pub normalization: Duration,
    pub bytecode_lowering: Duration,
    pub bytecode_validation: Duration,
    pub source_files: usize,
}

/// One compiled semantic program shared by the reference VM and later backends.
#[derive(Debug, Clone)]
pub struct ExecutableProgram {
    bytecode: ValidatedChunk,
    ssa: VerifiedProgram,
    bytecode_links: BytecodeLinkMetadata,
}

impl ExecutableProgram {
    pub fn bytecode(&self) -> &ValidatedChunk {
        &self.bytecode
    }

    pub fn ssa(&self) -> &VerifiedProgram {
        &self.ssa
    }

    pub fn bytecode_links(&self) -> &BytecodeLinkMetadata {
        &self.bytecode_links
    }

    pub fn into_bytecode(self) -> ValidatedChunk {
        self.bytecode
    }
}

pub fn compile_path(path: &Path, limits: &Limits) -> Result<ExecutableProgram> {
    compile_path_with_sources(path, limits).map(|(program, _)| program)
}

/// Compile while retaining low-overhead phase metrics for benchmark tooling.
pub fn compile_path_with_metrics(
    path: &Path,
    limits: &Limits,
) -> Result<(ExecutableProgram, CompileMetrics)> {
    let total_started = Instant::now();
    ensure_source_path(path)?;
    let (program, loading) =
        source::load_with_metrics(path, limits).map_err(source::SourceDiagnostic::into_core)?;
    let source_files = program.files().len();

    let hir_started = Instant::now();
    let mut analyzed = analyze_program_without_effects(&program)?;
    let hir_analysis = hir_started.elapsed();
    let effects_started = Instant::now();
    effects::infer(&mut analyzed);
    let effect_analysis = effects_started.elapsed();

    let (ssa, ssa_metrics) = lower_program_with_metrics(&analyzed)?;
    let bytecode_started = Instant::now();
    let (chunk, bytecode_links) = compile_program(&ssa)?;
    let bytecode_lowering = bytecode_started.elapsed();
    let validation_started = Instant::now();
    let bytecode = validate_chunk(chunk, &limits.validation)?;
    let bytecode_validation = validation_started.elapsed();
    let executable = ExecutableProgram {
        bytecode,
        ssa,
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
            ssa_construction: ssa_metrics.construction,
            ssa_verification: ssa_metrics.verification,
            normalization: ssa_metrics.normalization,
            bytecode_lowering,
            bytecode_validation,
            source_files,
        },
    ))
}

pub fn compile_path_with_sources(
    path: &Path,
    limits: &Limits,
) -> Result<(ExecutableProgram, Vec<PathBuf>)> {
    ensure_source_path(path)?;
    let program = load_for_compiler(path, limits)?;
    let sources = program
        .files()
        .iter()
        .map(|source| source.path.clone())
        .collect();
    let analyzed = analyze_program(&program)?;
    let executable = compile_analyzed(&analyzed, limits)?;
    Ok((executable, sources))
}

pub fn compile_source(source: &str, path: &str, limits: &Limits) -> Result<ExecutableProgram> {
    ensure_source_path(Path::new(path))?;
    let program = validate_for_compiler(source, path, limits)?;
    let analyzed = analyze_program(&program)?;
    compile_analyzed(&analyzed, limits)
}

fn compile_analyzed(analyzed: &hir::Program, limits: &Limits) -> Result<ExecutableProgram> {
    let ssa = lower_program(analyzed)?;
    let (chunk, bytecode_links) = compile_program(&ssa)?;
    let bytecode = validate_chunk(chunk, &limits.validation)?;
    Ok(ExecutableProgram {
        bytecode,
        ssa,
        bytecode_links,
    })
}

pub fn validate_source(source: &str, path: &str, limits: &Limits) -> Result<()> {
    ensure_source_path(Path::new(path))?;
    validate_for_compiler(source, path, limits).map(|_| ())
}

pub fn validate_source_tree(root: &Path, limits: &Limits) -> Result<()> {
    source::validate_source_directory_tree(root, limits.max_dir_children)
        .map_err(source::SourceDiagnostic::into_core)
}

pub(crate) fn ensure_source_path(path: &Path) -> Result<()> {
    source::ensure_source_path_for_compiler(path)
}

pub use lkjscript_core::Limits as CompileLimits;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
