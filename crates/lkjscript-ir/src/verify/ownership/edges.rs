use std::collections::BTreeSet;

use crate::verify::*;
use crate::{BlockId, Function, IrError, SsaType, ValueId};

pub(crate) fn consume_affine_arguments(
    arguments: &[ValueId],
    state: &mut OwnershipState,
    types: &[SsaType],
    user_call: bool,
    consume_handles: bool,
) -> crate::Result<()> {
    let mut seen = BTreeSet::new();
    for argument in arguments {
        let ty = value_type(types, *argument)?;
        if !is_affine(ty) {
            continue;
        }
        if !seen.insert(*argument) {
            return fail("SSA call duplicates one affine argument");
        }
        let resource = matches!(ty, SsaType::Resource(_));
        let Some(fact) = state.affine.get(argument) else {
            if resource && !consume_handles {
                continue;
            }
            return Err(IrError::new("SSA call uses an unavailable affine argument"));
        };
        if resource && !fact.transferred && !consume_handles {
            continue;
        }
        if user_call && is_owned_buf(ty) && !fact.transferred {
            return fail("SSA Owned call argument requires explicit Move transfer provenance");
        }
        if let Some(place) = current_owner_place(state, *argument) {
            if resource && consume_handles {
                state.owners.remove(&place);
            } else {
                return fail(format!(
                    "SSA call consumes current owner of PlaceId {} without explicit Move",
                    place.raw()
                ));
            }
        }
        state.affine.remove(argument);
    }
    Ok(())
}

pub(crate) fn transfer_edge(
    function: &Function,
    state: &OwnershipState,
    target: BlockId,
    arguments: &[ValueId],
    types: &[SsaType],
    work: &mut usize,
) -> crate::Result<OwnershipState> {
    let target_block = block_by_id(function, target)?;
    charge_ownership_work(work, ownership_state_cells(state)?)?;
    let argument_values: BTreeSet<ValueId> = arguments.iter().copied().collect();
    if state
        .owners
        .values()
        .any(|owner| !argument_values.contains(owner))
    {
        return fail(
            "SSA edge must transport every current affine owner through an explicit block argument",
        );
    }
    let mut next = state.clone();
    next.affine.clear();
    let mut seen = BTreeSet::new();
    for (argument, parameter) in arguments.iter().zip(&target_block.parameters) {
        let ty = value_type(types, *argument)?;
        if !is_affine(ty) {
            continue;
        }
        if !seen.insert(*argument) {
            return fail("SSA edge duplicates one affine argument");
        }
        let fact = state
            .affine
            .get(argument)
            .cloned()
            .ok_or_else(|| IrError::new("SSA edge transports an unavailable affine value"))?;
        let source_place = current_owner_place(state, *argument);
        match parameter.owner_place {
            Some(place) => {
                if source_place != Some(place) || next.owners.get(&place) != Some(argument) {
                    return fail(
                        "SSA owner block argument does not transport the current value for its PlaceId",
                    );
                }
                next.owners.insert(place, parameter.id);
                next.affine.remove(argument);
                next.affine.insert(
                    parameter.id,
                    AffineFact {
                        provenance: AffineProvenance::Place(place),
                        transferred: false,
                    },
                );
            }
            None => {
                if source_place.is_some() {
                    return fail(
                        "SSA edge cannot implicitly transfer a current Owned place; use Move",
                    );
                }
                next.affine.remove(argument);
                let provenance = if fact.transferred {
                    AffineProvenance::Transferred(parameter.id)
                } else {
                    match fact.provenance {
                        AffineProvenance::Fresh(_) => AffineProvenance::Fresh(parameter.id),
                        AffineProvenance::External(_) => AffineProvenance::External(parameter.id),
                        other => other,
                    }
                };
                next.affine.insert(
                    parameter.id,
                    AffineFact {
                        provenance,
                        transferred: fact.transferred,
                    },
                );
            }
        }
    }
    Ok(next)
}
