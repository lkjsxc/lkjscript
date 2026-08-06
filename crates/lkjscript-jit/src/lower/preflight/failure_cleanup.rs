use super::*;

// The native machine-plan boundary still stores cleanup calls per instruction.
// Decline shapes that would expand shared SSA chains beyond its compact tuning
// envelope; the validated VM continues to execute the same SSA/bytecode program.
const MAX_MATERIALIZED_NATIVE_CLEANUP_CALLS: u64 = 65_535;

pub(in crate::lower) fn preflight_failure_cleanups(
    function: &Function,
) -> Result<(), LoweringError> {
    let mut lengths = Vec::with_capacity(function.failure_cleanups.len());
    for node in &function.failure_cleanups {
        let tail = node
            .next
            .and_then(|next| lengths.get(next.index().unwrap_or(usize::MAX)).copied())
            .unwrap_or(0_u64);
        lengths.push(tail.checked_add(1).ok_or_else(|| {
            LoweringError::new(
                LoweringFailureCode::InvalidFunction,
                Some(function.id),
                "SSA native cleanup chain length overflow",
            )
        })?);
        match node.action {
            lkjscript_ir::FailureCleanupAction::EndBorrow { .. }
            | lkjscript_ir::FailureCleanupAction::DropOwner {
                glue:
                    lkjscript_ir::DropGlueIdentity::ByteVector
                    | lkjscript_ir::DropGlueIdentity::Bytes
                    | lkjscript_ir::DropGlueIdentity::Resource(_)
                    | lkjscript_ir::DropGlueIdentity::Structural(_),
                ..
            } => {}
        }
    }
    let mut expanded = 0_u64;
    for root in function.blocks.iter().flat_map(|block| {
        std::iter::once(block.metadata.failure_cleanup)
            .chain(block.instructions.iter().map(|item| item.metadata.failure_cleanup))
            .flatten()
            .flat_map(lkjscript_ir::FailureCleanupRoots::ids)
    }) {
        let length = lengths
            .get(root.index().unwrap_or(usize::MAX))
            .copied()
            .ok_or_else(|| {
                LoweringError::new(
                    LoweringFailureCode::InvalidFunction,
                    Some(function.id),
                    "SSA native cleanup root is invalid",
                )
            })?;
        expanded = expanded.checked_add(length).ok_or_else(|| {
            LoweringError::new(
                LoweringFailureCode::InvalidFunction,
                Some(function.id),
                "SSA native expanded cleanup size overflow",
            )
        })?;
    }
    if expanded > MAX_MATERIALIZED_NATIVE_CLEANUP_CALLS {
        return unsupported_operation(function.id, "wide shared failure cleanup chains");
    }
    Ok(())
}
