use std::collections::{BTreeSet, VecDeque};

use crate::verify::*;
use crate::{Function, InstructionKind, IrError, SsaType};

pub(crate) fn verify_ownership_facts(function: &Function, types: &[SsaType]) -> crate::Result<()> {
    let (mut work, entry) = collect_ownership_provenance(function)?;
    let reachable = reachable(function)?;
    for block in &function.blocks {
        let has_loop_action = block.instructions.iter().any(|instruction| {
            matches!(
                instruction.kind,
                InstructionKind::Move { .. } | InstructionKind::Borrow { .. }
            )
        });
        if has_loop_action && block_is_cyclic(function, block.id, &mut work)? {
            return fail(
                "SSA loop ownership state must be invariant; Move and Borrow are unavailable in loop cycles",
            );
        }
        if reachable.contains(&block.id) {
            continue;
        }
        let has_ownership = block.parameters.iter().any(|parameter| {
            parameter.owner_place.is_some() || contains_ownership_type(&parameter.ty)
        }) || block.instructions.iter().any(|instruction| {
            contains_ownership_type(&instruction.ty)
                || matches!(
                    instruction.kind,
                    InstructionKind::PlaceInit { .. }
                        | InstructionKind::PlaceEnd { .. }
                        | InstructionKind::EndBorrow { .. }
                        | InstructionKind::Drop { .. }
                        | InstructionKind::Move { .. }
                        | InstructionKind::Borrow { .. }
                )
        });
        if has_ownership {
            return fail("SSA unreachable blocks cannot contain ownership facts in this slice");
        }
    }

    let mut initial = OwnershipState::default();
    for parameter in &entry.parameters {
        if let Some(place) = parameter.owner_place {
            initial.active_places.insert(place);
            initial.owners.insert(place, parameter.id);
            initial.affine.insert(
                parameter.id,
                AffineFact {
                    provenance: AffineProvenance::Place(place),
                    transferred: false,
                },
            );
        } else if is_affine(&parameter.ty) {
            initial.affine.insert(
                parameter.id,
                AffineFact {
                    provenance: AffineProvenance::External(parameter.id),
                    transferred: false,
                },
            );
        }
    }

    let mut incoming = vec![None; function.blocks.len()];
    let entry_index = function
        .entry
        .index()
        .ok_or_else(|| IrError::new("SSA entry BlockId cannot index ownership state"))?;
    let Some(entry_state) = incoming.get_mut(entry_index) else {
        return fail("SSA entry ownership state is missing");
    };
    *entry_state = Some(initial);
    let mut queue = VecDeque::from([function.entry]);
    let mut queued = BTreeSet::from([function.entry]);
    let mut retained_state_cells = ownership_state_cells(
        incoming
            .get(entry_index)
            .and_then(Option::as_ref)
            .ok_or_else(|| IrError::new("SSA entry ownership state is missing"))?,
    )?;

    let nonowned_affine = nonowned_affine_values(function);
    while let Some(block_id) = queue.pop_front() {
        queued.remove(&block_id);
        charge_ownership_work(&mut work, 1)?;
        let index = block_id
            .index()
            .ok_or_else(|| IrError::new("SSA BlockId cannot index ownership state"))?;
        let state_ref = incoming
            .get(index)
            .and_then(Option::as_ref)
            .ok_or_else(|| IrError::new("SSA ownership worklist lost incoming state"))?;
        charge_ownership_work(&mut work, ownership_state_cells(state_ref)?)?;
        let state = state_ref.clone();
        let current = block_by_id(function, block_id)?;
        let successors =
            process_ownership_block(function, current, state, types, &nonowned_affine, &mut work)?;
        for (successor, successor_state) in successors {
            charge_ownership_work(&mut work, 1)?;
            let successor_index = successor
                .index()
                .ok_or_else(|| IrError::new("SSA successor cannot index ownership state"))?;
            let Some(slot) = incoming.get_mut(successor_index) else {
                return fail("SSA successor ownership state is missing");
            };
            match slot {
                Some(previous) if previous != &successor_state => {
                    charge_ownership_work(
                        &mut work,
                        ownership_state_cells(previous)?
                            .saturating_add(ownership_state_cells(&successor_state)?),
                    )?;
                    return fail(format!(
                        "SSA ownership predecessor states do not join exactly at block {}",
                        successor.raw()
                    ));
                }
                Some(previous) => {
                    charge_ownership_work(
                        &mut work,
                        ownership_state_cells(previous)?
                            .saturating_add(ownership_state_cells(&successor_state)?),
                    )?;
                }
                None => {
                    let cells = ownership_state_cells(&successor_state)?;
                    retained_state_cells = retained_state_cells
                        .checked_add(cells)
                        .ok_or_else(|| IrError::new("SSA retained ownership state overflow"))?;
                    if retained_state_cells > OWNERSHIP_VERIFY_MAX_RETAINED_STATE_CELLS {
                        return fail(format!(
                            "SSA retained ownership state exceeds {OWNERSHIP_VERIFY_MAX_RETAINED_STATE_CELLS} cells"
                        ));
                    }
                    charge_ownership_work(&mut work, cells)?;
                    *slot = Some(successor_state);
                    if queued.insert(successor) {
                        queue.push_back(successor);
                    }
                }
            }
        }
    }
    Ok(())
}
