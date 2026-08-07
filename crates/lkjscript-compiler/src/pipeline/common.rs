use lkjscript_core::{validate_chunk, Result, ValidationPolicy};

use crate::codegen::compile_program;
use crate::ssa::lower_program_with_metrics;
use crate::ExecutableProgram;

pub(super) fn compile_analyzed(
    analyzed: &crate::hir::Program,
    provenance: impl FnOnce(
        &crate::HirMemoryPlan,
    ) -> Result<crate::package::program::PreparationProvenance>,
) -> Result<ExecutableProgram> {
    let memory_verified = crate::memory_plan::verify_hir_memory(analyzed)?;
    let (ssa, _) = lower_program_with_metrics(&memory_verified)?;
    let memory_plan = memory_verified.plan().clone();
    let (chunk, bytecode_links) = compile_program(&ssa)?;
    let bytecode = validate_chunk(chunk, ValidationPolicy::Unrestricted)?;
    let provenance = provenance(&memory_plan)?;
    let (prepared, ssa, bytecode) =
        crate::package::program::bind(ssa, bytecode, &memory_plan, provenance)?;
    Ok(ExecutableProgram {
        prepared,
        bytecode,
        ssa,
        memory_plan,
        bytecode_links,
    })
}
