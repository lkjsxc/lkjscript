use std::collections::{HashMap, HashSet};

use crate::{BlockId, FailureCleanupAction, InstructionKind, Program, Terminator};

pub(crate) fn compact_blocks(program: &mut Program) -> crate::Result<()> {
    for function in &mut program.functions {
        let mut mapping = HashMap::with_capacity(function.blocks.len());
        for (index, block) in function.blocks.iter().enumerate() {
            let raw = u32::try_from(index)
                .map_err(|_| crate::IrError::new("SSA block count exceeds u32"))?;
            mapping.insert(block.id, BlockId::new(raw));
        }
        function.entry = mapped_block(&mapping, function.entry)?;
        for block in &mut function.blocks {
            block.id = mapped_block(&mapping, block.id)?;
            match &mut block.terminator {
                Terminator::Branch { target, .. } => *target = mapped_block(&mapping, *target)?,
                Terminator::ConditionalBranch {
                    true_target,
                    false_target,
                    ..
                } => {
                    *true_target = mapped_block(&mapping, *true_target)?;
                    *false_target = mapped_block(&mapping, *false_target)?;
                }
                _ => {}
            }
        }
    }
    compact_places(program)
}

pub(crate) fn compact_places(program: &mut Program) -> crate::Result<()> {
    for function in &mut program.functions {
        let mut referenced = HashSet::new();
        for block in &function.blocks {
            referenced.extend(
                block
                    .parameters
                    .iter()
                    .filter_map(|parameter| parameter.owner_place),
            );
            for instruction in &block.instructions {
                match instruction.kind {
                    InstructionKind::PlaceInit { place, .. }
                    | InstructionKind::PlaceEnd { place }
                    | InstructionKind::EndBorrow { place, .. }
                    | InstructionKind::Drop { place, .. }
                    | InstructionKind::Move { place, .. }
                    | InstructionKind::Borrow { place, .. } => {
                        referenced.insert(place);
                    }
                    _ => {}
                }
            }
        }
        for plan in &function.failure_cleanups {
            for action in &plan.actions {
                match action {
                    FailureCleanupAction::EndBorrow { place, .. } => {
                        referenced.insert(*place);
                    }
                    FailureCleanupAction::DropOwner {
                        place: Some(place), ..
                    } => {
                        referenced.insert(*place);
                    }
                    FailureCleanupAction::DropOwner { place: None, .. } => {}
                }
            }
        }
        let retained: Vec<_> = function
            .places
            .iter()
            .filter(|place| referenced.contains(&place.id))
            .cloned()
            .collect();
        let mut mapping = HashMap::with_capacity(retained.len());
        for (index, place) in retained.iter().enumerate() {
            let raw = u32::try_from(index)
                .map_err(|_| crate::IrError::new("SSA place count exceeds u32"))?;
            mapping.insert(place.id, crate::PlaceId::new(raw));
        }
        function.places = retained
            .into_iter()
            .map(|mut place| {
                place.id = mapped_place(&mapping, place.id)?;
                Ok(place)
            })
            .collect::<crate::Result<Vec<_>>>()?;
        for plan in &mut function.failure_cleanups {
            for action in &mut plan.actions {
                match action {
                    FailureCleanupAction::EndBorrow { place, .. } => {
                        *place = mapped_place(&mapping, *place)?;
                    }
                    FailureCleanupAction::DropOwner {
                        place: Some(place), ..
                    } => {
                        *place = mapped_place(&mapping, *place)?;
                    }
                    FailureCleanupAction::DropOwner { place: None, .. } => {}
                }
            }
        }
        for block in &mut function.blocks {
            for parameter in &mut block.parameters {
                if let Some(place) = parameter.owner_place {
                    parameter.owner_place = Some(mapped_place(&mapping, place)?);
                }
            }
            for instruction in &mut block.instructions {
                match &mut instruction.kind {
                    InstructionKind::PlaceInit { place, .. }
                    | InstructionKind::PlaceEnd { place }
                    | InstructionKind::EndBorrow { place, .. }
                    | InstructionKind::Drop { place, .. }
                    | InstructionKind::Move { place, .. }
                    | InstructionKind::Borrow { place, .. } => {
                        *place = mapped_place(&mapping, *place)?;
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn mapped_place(
    mapping: &HashMap<crate::PlaceId, crate::PlaceId>,
    id: crate::PlaceId,
) -> crate::Result<crate::PlaceId> {
    mapping
        .get(&id)
        .copied()
        .ok_or_else(|| crate::IrError::new(format!("pass lost SSA place {}", id.raw())))
}

pub(crate) fn mapped_block(
    mapping: &HashMap<BlockId, BlockId>,
    id: BlockId,
) -> crate::Result<BlockId> {
    mapping
        .get(&id)
        .copied()
        .ok_or_else(|| crate::IrError::new(format!("pass lost SSA block {}", id.raw())))
}
