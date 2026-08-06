fn process_aggregate_payload(
    program: &Program,
    function: &Function,
    place: &Option<PlaceId>,
    value: &crate::ValueId,
    state: &mut OwnershipState,
    live_loans: &std::collections::BTreeMap<PlaceId, Vec<LiveLoan>>,
    types: &[SsaType],
) -> crate::Result<()> {
    let value_ty = types
        .get(value.index().unwrap_or(usize::MAX))
        .ok_or_else(|| IrError::new("SSA payload transfer value type is missing"))?;
    let copy_type = program
        .memory
        .type_for(value_ty)
        .is_some_and(|ty| ty.mode == crate::StructuralTypeMode::Copy);
    if copy_type {
        let fresh_copy = fresh_copy_owner(function, *value, &mut std::collections::HashSet::new());
        if place.is_some() || !fresh_copy {
            return fail("SSA copy aggregate payload transfer requires one unplaced fresh value");
        }
    } else {
        let fact = state.affine.get(value).ok_or_else(|| {
            IrError::new("SSA aggregate payload transfer consumes unavailable whole owner")
        })?;
        match place {
            Some(place) => {
                let current = state.owners.get(place) == Some(value);
                let moved = fact.transferred
                    && matches!(fact.provenance, AffineProvenance::Place(source) if source == *place)
                    && state.active_places.contains(place);
                if !current && !moved {
                    return fail("SSA aggregate payload transfer has stale owner-place provenance");
                }
                if live_loans.get(place).is_some_and(|loans| !loans.is_empty()) {
                    return fail("SSA aggregate payload transfer conflicts with a live loan");
                }
                state.owners_mut().remove(place);
            }
            None if current_owner_place(state, *value).is_some() => {
                return fail("SSA unplaced aggregate payload transfer names a placed owner");
            }
            None => {}
        }
        state.affine_mut().remove(value);
    }
    Ok(())
}

fn fresh_copy_owner(
    function: &Function,
    value: crate::ValueId,
    visiting: &mut std::collections::HashSet<crate::ValueId>,
) -> bool {
    if !visiting.insert(value) {
        return false;
    }
    if function.blocks.iter().any(|block| {
        block.instructions.iter().any(|candidate| {
            candidate.id == value
                && matches!(
                    candidate.kind,
                    InstructionKind::StructuralCopy { .. }
                        | InstructionKind::StructuralPublish { .. }
                        | InstructionKind::DestinationFinish { .. }
                        | InstructionKind::AggregateConsumePayload { .. }
                        | InstructionKind::Call { .. }
                        | InstructionKind::Runtime { .. }
                        | InstructionKind::F64FromI64Exact { .. }
                        | InstructionKind::I64FromF64Exact { .. }
                        | InstructionKind::I64FromF64Trunc { .. }
                )
        })
    }) {
        visiting.remove(&value);
        return true;
    }
    let result = function.blocks.iter().any(|block| {
        block
            .parameters
            .iter()
            .position(|parameter| parameter.id == value)
            .is_some_and(|index| {
                let incoming = function
                    .blocks
                    .iter()
                    .flat_map(|predecessor| {
                        edge_arguments_to(&predecessor.terminator, block.id, index)
                    })
                    .collect::<Vec<_>>();
                !incoming.is_empty()
                    && incoming
                        .into_iter()
                        .all(|argument| fresh_copy_owner(function, argument, visiting))
            })
    });
    visiting.remove(&value);
    result
}
