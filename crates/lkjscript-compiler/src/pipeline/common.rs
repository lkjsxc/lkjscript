use std::time::{Duration, Instant};

use lkjscript_core::{validate_chunk, Result, ValidationPolicy};

use crate::codegen::compile_program;
use crate::ssa::lower_program_with_metrics;
use crate::{CompileSnapshotError, ExecutableProgram, IncompleteSnapshotError, WorkspaceSnapshot};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct SnapshotCompileMetrics {
    pub memory_planning: Duration,
    pub ssa_construction: Duration,
    pub ssa_verification: Duration,
    pub normalization: Duration,
    pub bytecode_lowering: Duration,
    pub bytecode_validation: Duration,
    pub package_validation: Duration,
}

/// The sole post-import compiler boundary.
///
/// This module deliberately has no source, parser, formatter, or analyzer
/// dependency. It consumes the snapshot's complete typed HIR directly.
pub fn compile_snapshot(
    snapshot: &WorkspaceSnapshot,
) -> std::result::Result<ExecutableProgram, CompileSnapshotError> {
    if snapshot.state() == crate::workspace::ProgramState::Incomplete {
        let mut holes = Vec::new();
        holes.try_reserve(snapshot.holes().len()).map_err(|_| {
            CompileSnapshotError::Compiler(lkjscript_core::Error::host(
                "incomplete snapshot hole-list allocation failed",
            ))
        })?;
        holes.extend(snapshot.holes().map(|hole| hole.id));
        return Err(CompileSnapshotError::Incomplete(IncompleteSnapshotError {
            revision: snapshot.revision(),
            holes,
        }));
    }
    compile_snapshot_with_metrics(snapshot)
        .map(|(program, _)| program)
        .map_err(CompileSnapshotError::from)
}

pub(super) fn compile_snapshot_with_metrics(
    snapshot: &WorkspaceSnapshot,
) -> Result<(ExecutableProgram, SnapshotCompileMetrics)> {
    snapshot.validate_consistency()?;

    let memory_started = Instant::now();
    let memory_verified = crate::memory_plan::verify_hir_memory(snapshot.hir())?;
    let memory_planning = memory_started.elapsed();

    let memory_plan = memory_verified.plan().clone();
    let package_validation_started = Instant::now();
    snapshot.validate_memory_plan(&memory_plan)?;
    let mut package_validation = package_validation_started.elapsed();

    let (ssa, ssa_metrics) = lower_program_with_metrics(&memory_verified)?;

    let bytecode_started = Instant::now();
    let (chunk, bytecode_links) = compile_program(&ssa)?;
    let bytecode_lowering = bytecode_started.elapsed();

    let validation_started = Instant::now();
    let bytecode = validate_chunk(chunk, ValidationPolicy::Unrestricted)?;
    let bytecode_validation = validation_started.elapsed();

    let capability_validation_started = Instant::now();
    snapshot.validate_required_capabilities(bytecode.required_capabilities())?;
    package_validation = package_validation.saturating_add(capability_validation_started.elapsed());

    Ok((
        ExecutableProgram {
            bytecode,
            ssa,
            memory_plan,
            bytecode_links,
        },
        SnapshotCompileMetrics {
            memory_planning,
            ssa_construction: ssa_metrics.construction,
            ssa_verification: ssa_metrics.verification,
            normalization: ssa_metrics.normalization,
            bytecode_lowering,
            bytecode_validation,
            package_validation,
        },
    ))
}
