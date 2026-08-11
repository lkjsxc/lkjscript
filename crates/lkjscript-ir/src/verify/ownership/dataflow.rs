use std::collections::{BTreeSet, VecDeque};

use crate::verify::*;
use crate::{Function, InstructionKind, IrError, Program, SsaType};

pub(crate) fn verify_ownership_facts(
    program: &Program,
    function: &Function,
    types: &[SsaType],
    cfg: &ControlFlowGraph,
) -> crate::Result<()> {
    let entry = collect_ownership_provenance(function)?;
    for block in &function.blocks {
        if cfg.is_reachable(block.id)? {
            continue;
        }
        let has_ownership = block.parameters.iter().any(|parameter| {
            parameter.owner_place.is_some() || contains_ownership_type(program, &parameter.ty)
        }) || block.instructions.iter().any(|instruction| {
            contains_ownership_type(program, &instruction.ty)
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
            initial.active_places_mut().insert(place);
            initial.owners_mut().insert(place, parameter.id);
            initial.affine_mut().insert(
                parameter.id,
                AffineFact {
                    provenance: AffineProvenance::Place(place),
                    transferred: false,
                },
            );
        } else if is_affine(program, &parameter.ty) {
            initial.affine_mut().insert(
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

    // A state with only one possible incoming source can be moved into block processing. States
    // that participate in joins remain shared so later predecessors can be compared exactly.
    let mut incoming_sources = vec![0usize; function.blocks.len()];
    incoming_sources[entry_index] = 1;
    for block in &function.blocks {
        for successor in successors(&block.terminator) {
            let successor_index = successor
                .index()
                .ok_or_else(|| IrError::new("SSA successor cannot index ownership state"))?;
            let Some(count) = incoming_sources.get_mut(successor_index) else {
                return fail("SSA successor ownership state is missing");
            };
            *count = count
                .checked_add(1)
                .ok_or_else(|| IrError::new("SSA ownership predecessor count overflow"))?;
        }
    }

    let mut queue = VecDeque::from([function.entry]);
    let mut queued = BTreeSet::from([function.entry]);
    let nonowned_affine = nonowned_affine_values(program, function);
    while let Some(block_id) = queue.pop_front() {
        queued.remove(&block_id);
        let index = block_id
            .index()
            .ok_or_else(|| IrError::new("SSA BlockId cannot index ownership state"))?;
        let state = if incoming_sources.get(index).copied() == Some(1) {
            incoming
                .get_mut(index)
                .and_then(Option::take)
                .ok_or_else(|| IrError::new("SSA ownership worklist lost incoming state"))?
        } else {
            incoming
                .get(index)
                .and_then(Option::as_ref)
                .cloned()
                .ok_or_else(|| IrError::new("SSA ownership worklist lost incoming state"))?
        };
        let current = block_by_id(function, block_id)?;
        let successors =
            process_ownership_block(program, function, current, state, types, &nonowned_affine)?;
        for (successor, successor_state) in successors {
            let successor_index = successor
                .index()
                .ok_or_else(|| IrError::new("SSA successor cannot index ownership state"))?;
            let Some(slot) = incoming.get_mut(successor_index) else {
                return fail("SSA successor ownership state is missing");
            };
            match slot {
                Some(previous) if previous != &successor_state => {
                    return fail(format!(
                        concat!(
                            "SSA ownership predecessor states do not join exactly at block {}: ",
                            "previous {previous:?}, incoming {successor_state:?}",
                        ),
                        successor.raw(),
                        previous = previous,
                        successor_state = successor_state,
                    ));
                }
                Some(_) => {}
                None => {
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
