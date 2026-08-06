fn verify_failure_cleanup_shape(
    program: &Program,
    function: &Function,
    types: &[SsaType],
) -> crate::Result<()> {
    let mut seen = HashSet::new();
    for (index, node) in function.failure_cleanups.iter().enumerate() {
        if node
            .next
            .is_some_and(|next| next.index().is_none_or(|next| next >= index))
        {
            return fail("SSA failure-cleanup links must reference prior nodes");
        }
        if !seen.insert(*node) {
            return fail("SSA failure-cleanup nodes must be interned uniquely");
        }
        match &node.action {
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
    for roots in function.blocks.iter().flat_map(|block| {
        std::iter::once(block.metadata.failure_cleanup)
            .chain(block.instructions.iter().map(|item| item.metadata.failure_cleanup))
            .flatten()
    }) {
        if roots.ids().next().is_none() {
            return fail("SSA empty failure cleanup must be None");
        }
        if roots.ids().any(|root| {
            root.index()
                .is_none_or(|index| index >= function.failure_cleanups.len())
        }) {
            return fail("SSA failure-cleanup root is out of range");
        }
    }
    Ok(())
}
