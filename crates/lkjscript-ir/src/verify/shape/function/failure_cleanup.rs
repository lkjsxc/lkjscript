fn verify_failure_cleanup_shape(
    program: &Program,
    function: &Function,
    types: &[SsaType],
) -> crate::Result<()> {
    let mut seen = HashSet::new();
    for (index, plan) in function.failure_cleanups.iter().enumerate() {
        if plan.id.index() != Some(index) {
            return fail("SSA failure-cleanup plans must have dense IDs in order");
        }
        if plan.actions.is_empty() || !seen.insert(&plan.actions) {
            return fail("SSA failure-cleanup plans must be nonempty and interned uniquely");
        }
        for action in &plan.actions {
            match action {
                FailureCleanupAction::EndBorrow {
                    place, kind, value, ..
                } => {
                    let value_type = value_type(types, *value)?;
                    let valid_type = match kind {
                        BorrowKind::Shared => {
                            matches!(value_type, SsaType::Bytes | SsaType::ByteSlice)
                        }
                        BorrowKind::Mutable => *value_type == SsaType::ByteSliceMut,
                    };
                    if !valid_type
                        || function
                            .places
                            .get(place.index().unwrap_or(usize::MAX))
                            .is_none()
                    {
                        return fail(format!(
                            "SSA failure cleanup has malformed loan-end place {} kind {:?} value {} type {:?}",
                            place.raw(), kind, value.raw(), value_type
                        ));
                    }
                }
                FailureCleanupAction::DropOwner { place, value, glue } => {
                    if expected_drop_glue(program, value_type(types, *value)?) != Some(*glue) {
                        return fail("SSA failure cleanup has mismatched owner drop glue");
                    }
                    if let Some(place) = place {
                        let declared = function
                            .places
                            .get(place.index().unwrap_or(usize::MAX))
                            .ok_or_else(|| IrError::new("SSA failure cleanup has invalid place"))?;
                        if declared.id != *place || declared.drop_glue != Some(*glue) {
                            return fail("SSA failure cleanup has mismatched owner place");
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
