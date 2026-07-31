use super::*;

pub(super) fn validate_failure_cleanup_shape(
    proto: &FunctionProto,
    category: &str,
    limits: &ValidationLimits,
) -> Result<()> {
    if proto.failure_cleanups.len() > limits.max_table_entries
        || proto.failure_cleanup_ranges.len() > limits.max_table_entries
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} failure-cleanup table exceeds limit",
            proto.name
        )));
    }
    let mut actions = 0usize;
    let mut seen = HashSet::new();
    for plan in &proto.failure_cleanups {
        actions = actions
            .checked_add(plan.actions.len())
            .ok_or_else(|| Error::msg("bytecode failure-cleanup action count overflow"))?;
        if plan.actions.is_empty() || !seen.insert(plan) {
            return Err(Error::msg(
                "bytecode failure-cleanup plans must be nonempty and unique",
            ));
        }
        for action in &plan.actions {
            let (local, place) = match action {
                crate::FailureCleanupAction::EndBorrow { local, place, kind } => {
                    if !matches!(
                        kind,
                        crate::UniqueValueKind::Bytes
                            | crate::UniqueValueKind::ByteSlice
                            | crate::UniqueValueKind::ByteSliceMut
                    ) {
                        return Err(Error::msg(
                            "bytecode failure loan-end has an owner-only unique kind",
                        ));
                    }
                    (*local, Some(*place))
                }
                crate::FailureCleanupAction::DropUnique { local, place, kind } => {
                    if !matches!(
                        kind,
                        crate::UniqueValueKind::Bytes | crate::UniqueValueKind::ByteVector
                    ) {
                        return Err(Error::msg(
                            "bytecode failure drop has a borrowed unique kind",
                        ));
                    }
                    (*local, *place)
                }
                crate::FailureCleanupAction::DropResource { local, place, kind } => {
                    if matches!(
                        kind,
                        crate::ResourceKind::InputStream | crate::ResourceKind::OutputStream
                    ) {
                        return Err(Error::msg(
                            "bytecode failure cleanup cannot own borrowed standard streams",
                        ));
                    }
                    (*local, *place)
                }
                crate::FailureCleanupAction::EndStructuralBorrow { local, place, .. } => {
                    (*local, Some(*place))
                }
                crate::FailureCleanupAction::DropStructural { local, place, .. } => {
                    (*local, *place)
                }
                crate::FailureCleanupAction::AbortStructuralDestination { local, .. } => {
                    (*local, None)
                }
            };
            if local >= proto.locals || place.is_some_and(|place| place >= proto.unique_places) {
                return Err(Error::msg(
                    "bytecode failure-cleanup local or place is out of range",
                ));
            }
        }
    }
    if actions > limits.max_table_entries {
        return Err(Error::msg(
            "bytecode failure-cleanup aggregate actions exceed limit",
        ));
    }
    let mut previous_end = 0u16;
    for (index, range) in proto.failure_cleanup_ranges.iter().enumerate() {
        if range.start >= range.end
            || usize::from(range.end) > proto.code.len()
            || range
                .plan
                .into_iter()
                .chain(range.unentered_plan)
                .any(|plan| usize::from(plan) >= proto.failure_cleanups.len())
            || (index > 0 && range.start < previous_end)
        {
            return Err(Error::msg(
                "bytecode failure-cleanup ranges are malformed or overlapping",
            ));
        }
        previous_end = range.end;
    }
    Ok(())
}

pub(super) fn failure_metadata_bytes(proto: &FunctionProto) -> Result<usize> {
    let actions = proto
        .failure_cleanups
        .iter()
        .try_fold(0usize, |total, plan| total.checked_add(plan.actions.len()))
        .ok_or_else(|| Error::msg("bytecode failure-cleanup metadata size overflow"))?;
    let plans = proto
        .failure_cleanups
        .len()
        .checked_mul(4)
        .ok_or_else(|| Error::msg("bytecode failure-cleanup metadata size overflow"))?;
    let action_bytes = actions
        .checked_mul(8)
        .ok_or_else(|| Error::msg("bytecode failure-cleanup metadata size overflow"))?;
    let range_bytes = proto
        .failure_cleanup_ranges
        .len()
        .checked_mul(8)
        .ok_or_else(|| Error::msg("bytecode failure-cleanup metadata size overflow"))?;
    plans
        .checked_add(action_bytes)
        .and_then(|bytes| bytes.checked_add(range_bytes))
        .ok_or_else(|| Error::msg("bytecode failure-cleanup metadata size overflow"))
}
