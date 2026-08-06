use super::*;

pub(super) fn validate_failure_cleanup_shape(proto: &FunctionProto) -> Result<()> {
    let mut seen = HashSet::new();
    for (index, node) in proto.failure_cleanups.iter().enumerate() {
        if node
            .next
            .is_some_and(|next| next.index().is_none_or(|next| next >= index))
        {
            return Err(Error::msg(
                "bytecode failure-cleanup links must reference prior nodes",
            ));
        }
        if !seen.insert(*node) {
            return Err(Error::msg(
                "bytecode failure-cleanup nodes must be interned uniquely",
            ));
        }
        let (local, place) = match &node.action {
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
            crate::FailureCleanupAction::DropStructural { local, place, .. } => (*local, *place),
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

    let mut previous_end = 0_u64;
    for (index, range) in proto.failure_cleanup_ranges.iter().enumerate() {
        let end = usize::try_from(range.end)
            .map_err(|_| Error::msg("bytecode failure-cleanup range exceeds host usize"))?;
        if range.start >= range.end
            || end > proto.code.len()
            || range
                .plan
                .is_some_and(|roots| roots.ids().next().is_none())
            || range
                .plan
                .into_iter()
                .flat_map(crate::FailureCleanupRoots::ids)
                .chain(range.unentered_plan)
                .any(|root| {
                    root.index()
                        .is_none_or(|root| root >= proto.failure_cleanups.len())
                })
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
    let node_bytes = proto
        .failure_cleanups
        .len()
        .checked_mul(std::mem::size_of::<crate::FailureCleanupNode>())
        .ok_or_else(|| Error::host("bytecode failure-cleanup metadata size overflow"))?;
    let range_bytes = proto
        .failure_cleanup_ranges
        .len()
        .checked_mul(std::mem::size_of::<crate::FailureCleanupRange>())
        .ok_or_else(|| Error::host("bytecode failure-cleanup metadata size overflow"))?;
    node_bytes
        .checked_add(range_bytes)
        .ok_or_else(|| Error::host("bytecode failure-cleanup metadata size overflow"))
}
