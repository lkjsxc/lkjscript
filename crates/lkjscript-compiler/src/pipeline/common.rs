use std::time::{Duration, Instant};

#[cfg(test)]
thread_local! {
    static LOWERING_INVOCATIONS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_lowering_invocations() {
    LOWERING_INVOCATIONS.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn lowering_invocations() -> u64 {
    LOWERING_INVOCATIONS.with(std::cell::Cell::get)
}

use lkjscript_core::{validate_chunk, ValidationPolicy};

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
/// This module deliberately has no source, parser, formatter, or source
/// analyzer dependency. It derives complete compiler HIR from the selected
/// semantic revision and rejects incompleteness before compiler-only lowering.
pub fn compile_snapshot(
    snapshot: &WorkspaceSnapshot,
) -> std::result::Result<ExecutableProgram, CompileSnapshotError> {
    compile_snapshot_with_metrics(snapshot).map(|(program, _)| program)
}

pub(super) fn compile_snapshot_with_metrics(
    snapshot: &WorkspaceSnapshot,
) -> std::result::Result<(ExecutableProgram, SnapshotCompileMetrics), CompileSnapshotError> {
    require_complete(snapshot)?;
    let hir = snapshot
        .validated_complete_hir()
        .map_err(CompileSnapshotError::Compiler)?;
    #[cfg(test)]
    LOWERING_INVOCATIONS.with(|count| count.set(count.get().saturating_add(1)));

    let memory_started = Instant::now();
    let memory_verified =
        crate::memory_plan::verify_hir_memory(&hir).map_err(CompileSnapshotError::Compiler)?;
    let memory_planning = memory_started.elapsed();

    let memory_plan = memory_verified.plan().clone();
    let package_validation_started = Instant::now();
    snapshot
        .validate_memory_plan(&memory_plan)
        .map_err(CompileSnapshotError::Package)?;
    let mut package_validation = package_validation_started.elapsed();

    let (ssa, ssa_metrics) =
        lower_program_with_metrics(&memory_verified).map_err(CompileSnapshotError::Compiler)?;

    let bytecode_started = Instant::now();
    let (chunk, bytecode_links) = compile_program(&ssa).map_err(CompileSnapshotError::Compiler)?;
    let bytecode_lowering = bytecode_started.elapsed();

    let validation_started = Instant::now();
    let bytecode = validate_chunk(chunk, ValidationPolicy::Unrestricted)
        .map_err(CompileSnapshotError::Compiler)?;
    let bytecode_validation = validation_started.elapsed();

    let capability_validation_started = Instant::now();
    snapshot
        .validate_required_capabilities(bytecode.required_capabilities())
        .map_err(CompileSnapshotError::Package)?;
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

fn require_complete(snapshot: &WorkspaceSnapshot) -> Result<(), CompileSnapshotError> {
    if snapshot.state() != crate::workspace::ProgramState::Incomplete {
        return Ok(());
    }
    let mut blockers = Vec::new();
    blockers
        .try_reserve(snapshot.completeness_blockers().len())
        .map_err(|_| {
            CompileSnapshotError::Compiler(lkjscript_core::Error::host(
                "incomplete snapshot blocker allocation failed",
            ))
        })?;
    blockers.extend(snapshot.completeness_blockers().iter().cloned());
    Err(CompileSnapshotError::Incomplete(IncompleteSnapshotError {
        revision: snapshot.revision(),
        blockers,
    }))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{compile_snapshot, compile_snapshot_with_metrics};
    use crate::{CompileSnapshotError, Workspace};

    #[test]
    fn measured_and_unmeasured_compilation_share_the_completeness_gate() {
        let workspace = Workspace::empty().expect("empty workspace");
        let plain = compile_snapshot(&workspace.current());
        let measured = compile_snapshot_with_metrics(&workspace.current());
        assert!(matches!(&plain, Err(CompileSnapshotError::Incomplete(_))));
        assert!(matches!(
            &measured,
            Err(CompileSnapshotError::Incomplete(_))
        ));
        if let (
            Err(CompileSnapshotError::Incomplete(plain)),
            Err(CompileSnapshotError::Incomplete(measured)),
        ) = (plain, measured)
        {
            assert_eq!(plain.revision, measured.revision);
            assert_eq!(plain.blockers, measured.blockers);
        }
    }
}
