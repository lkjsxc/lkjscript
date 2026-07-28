use super::*;

pub(super) fn cleanup_values(
    function: &Function,
    cleanup: Option<lkjscript_ir::FailureCleanupId>,
) -> Result<Vec<ValueId>> {
    let Some(cleanup) = cleanup else {
        return Ok(Vec::new());
    };
    let plan = function
        .failure_cleanups
        .get(cleanup.index().unwrap_or(usize::MAX))
        .filter(|plan| plan.id == cleanup)
        .ok_or_else(|| Error::msg("local allocation lost failure-cleanup plan"))?;
    Ok(plan
        .actions
        .iter()
        .map(|action| match action {
            SsaFailureCleanupAction::EndBorrow { value, .. }
            | SsaFailureCleanupAction::DropOwner { value, .. } => *value,
        })
        .collect())
}
