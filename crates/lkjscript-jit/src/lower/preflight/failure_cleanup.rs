use super::*;

pub(super) fn preflight_failure_cleanups(function: &Function) -> Result<(), LoweringError> {
    for plan in &function.failure_cleanups {
        for action in &plan.actions {
            match action {
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
    }
    Ok(())
}
