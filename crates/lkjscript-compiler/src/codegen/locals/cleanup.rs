use super::*;

pub(super) fn cleanup_values(
    function: &Function,
    cleanup: Option<lkjscript_ir::FailureCleanupRoots>,
) -> Result<impl Iterator<Item = ValueId> + '_> {
    if cleanup.is_some_and(|cleanup| {
        cleanup.ids().any(|root| {
            root.index()
                .is_none_or(|index| index >= function.failure_cleanups.len())
        })
    }) {
        return Err(Error::msg("local allocation lost failure-cleanup chain"));
    }
    Ok(function
        .failure_cleanup_actions(cleanup)
        .map(|action| match action {
            SsaFailureCleanupAction::EndBorrow { value, .. }
            | SsaFailureCleanupAction::DropOwner { value, .. } => *value,
        }))
}
