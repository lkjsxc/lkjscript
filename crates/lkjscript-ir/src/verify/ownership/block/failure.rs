use super::*;

pub(super) fn verify_failure_cleanup(
    function: &Function,
    instruction: &Instruction,
    state: &OwnershipState,
    live_loans: &BTreeMap<crate::PlaceId, Vec<LiveLoan>>,
    types: &[SsaType],
    nonowned_affine: &HashSet<ValueId>,
) -> crate::Result<()> {
    let exclusions = match &instruction.kind {
        InstructionKind::Call { arguments, .. } => arguments.as_slice(),
        _ => &[],
    };
    verify_failure_cleanup_plan(
        function,
        instruction.metadata.failure_cleanup,
        &expected_failure_cleanup(
            function,
            state,
            live_loans,
            types,
            nonowned_affine,
            exclusions,
        )?,
        &format!(
            "instruction {} {:?}",
            instruction.id.raw(),
            instruction.kind
        ),
    )
}

pub(super) fn expected_failure_cleanup(
    function: &Function,
    state: &OwnershipState,
    live_loans: &BTreeMap<crate::PlaceId, Vec<LiveLoan>>,
    types: &[SsaType],
    nonowned_affine: &HashSet<ValueId>,
    exclusions: &[ValueId],
) -> crate::Result<Vec<FailureCleanupAction>> {
    let mut loans: Vec<_> = live_loans
        .iter()
        .flat_map(|(place, loans)| loans.iter().map(|loan| (*place, loan)))
        .collect();
    loans.sort_by_key(|(_, loan)| loan.loan);
    let mut expected = Vec::new();
    for (place, loan) in loans.into_iter().rev() {
        expected.push(FailureCleanupAction::EndBorrow {
            place,
            loan: loan.loan,
            kind: loan.kind,
            value: loan.value,
        });
    }
    let placed: std::collections::BTreeSet<_> = state.owners.values().copied().collect();
    let mut unplaced = Vec::new();
    for value in state.affine.keys() {
        if !exclusions.contains(value)
            && !placed.contains(value)
            && !nonowned_affine.contains(value)
            && is_owned_value(value_type(types, *value)?)
        {
            unplaced.push(*value);
        }
    }
    unplaced.sort();
    for value in unplaced.into_iter().rev() {
        let glue = expected_drop_glue(value_type(types, value)?)
            .ok_or_else(|| IrError::new("SSA unplaced failure owner has no drop glue"))?;
        expected.push(FailureCleanupAction::DropOwner {
            place: None,
            value,
            glue,
        });
    }
    for place in function.places.iter().rev() {
        if let Some(value) = state.owners.get(&place.id) {
            let glue = place
                .drop_glue
                .ok_or_else(|| IrError::new("SSA failure cleanup owner has no drop glue"))?;
            expected.push(FailureCleanupAction::DropOwner {
                place: Some(place.id),
                value: *value,
                glue,
            });
        }
    }
    Ok(expected)
}

pub(super) fn verify_failure_cleanup_plan(
    function: &Function,
    id: Option<FailureCleanupId>,
    expected: &[FailureCleanupAction],
    site: &str,
) -> crate::Result<()> {
    let Some(id) = id else {
        return if expected.is_empty() {
            Ok(())
        } else {
            fail(format!("SSA {site} lacks nonempty failure cleanup"))
        };
    };
    let plan = function
        .failure_cleanups
        .get(id.index().unwrap_or(usize::MAX))
        .filter(|plan| plan.id == id)
        .ok_or_else(|| IrError::new(format!("SSA {site} has invalid failure cleanup")))?;
    if plan.actions != expected {
        return fail(format!(
            "SSA {site} failure cleanup {:?} does not match expected {:?}",
            plan.actions, expected
        ));
    }
    Ok(())
}
