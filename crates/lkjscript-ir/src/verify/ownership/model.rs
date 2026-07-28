use std::collections::{BTreeMap, BTreeSet};

use crate::verify::*;
use crate::{BlockId, CallTarget, Function, InstructionKind, Terminator, ValueId};

pub(crate) fn precheck_ownership_work_shape(function: &Function) -> crate::Result<()> {
    let mut work = 0usize;
    charge_ownership_work(&mut work, function.signature.type_parameters.len())?;
    charge_ownership_work(&mut work, function.signature.bounds.len())?;
    charge_ownership_work(&mut work, function.signature.parameters.len())?;
    charge_ownership_work(&mut work, function.blocks.len())?;
    charge_ownership_work(&mut work, function.places.len())?;
    charge_ownership_work(&mut work, function.failure_cleanups.len())?;
    for plan in &function.failure_cleanups {
        charge_ownership_work(&mut work, plan.actions.len())?;
    }
    for block in &function.blocks {
        charge_ownership_work(&mut work, block.parameters.len())?;
        charge_ownership_work(&mut work, block.instructions.len())?;
        charge_ownership_work(&mut work, terminator_operand_count(&block.terminator))?;
        if let Some(frame) = &block.metadata.frame_state {
            charge_ownership_work(&mut work, frame.locals.len())?;
            charge_ownership_work(&mut work, frame.operand_stack.len())?;
        }
        for instruction in &block.instructions {
            charge_ownership_work(&mut work, instruction_operand_count(&instruction.kind))?;
            if let Some(frame) = &instruction.metadata.frame_state {
                charge_ownership_work(&mut work, frame.locals.len())?;
                charge_ownership_work(&mut work, frame.operand_stack.len())?;
            }
        }
    }
    Ok(())
}

pub(crate) fn instruction_operand_count(kind: &InstructionKind) -> usize {
    match kind {
        InstructionKind::Constant(_)
        | InstructionKind::PlaceEnd { .. }
        | InstructionKind::FunctionRef(_) => 0,
        InstructionKind::Copy(_)
        | InstructionKind::PlaceInit { .. }
        | InstructionKind::EndBorrow { .. }
        | InstructionKind::Drop { .. }
        | InstructionKind::Move { .. }
        | InstructionKind::Borrow { .. }
        | InstructionKind::F64FromI64Exact { .. }
        | InstructionKind::F64FromI64Rounded { .. }
        | InstructionKind::I64FromF64Exact { .. }
        | InstructionKind::I64FromF64Trunc { .. }
        | InstructionKind::ProductField { .. }
        | InstructionKind::EnumIsVariant { .. }
        | InstructionKind::EnumField { .. } => 1,
        InstructionKind::Runtime { arguments, .. }
        | InstructionKind::Call {
            target: CallTarget::Direct(_),
            arguments,
            ..
        }
        | InstructionKind::ProductValue {
            fields: arguments, ..
        }
        | InstructionKind::EnumValue {
            fields: arguments, ..
        } => arguments.len(),
        InstructionKind::Call {
            target: CallTarget::Indirect(_),
            arguments,
            ..
        } => arguments.len().saturating_add(1),
        InstructionKind::WithProductField { .. } => 2,
    }
}

pub(crate) fn terminator_operand_count(terminator: &Terminator) -> usize {
    match terminator {
        Terminator::Branch { arguments, .. } => arguments.len(),
        Terminator::ConditionalBranch {
            true_arguments,
            false_arguments,
            ..
        } => 1usize
            .saturating_add(true_arguments.len())
            .saturating_add(false_arguments.len()),
        Terminator::Return(_) | Terminator::Exit { .. } => 1,
        Terminator::Trap { .. } => 1,
        Terminator::Outcome { detail, .. } => usize::from(detail.is_some()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum AffineProvenance {
    Place(crate::PlaceId),
    Fresh(ValueId),
    Transferred(ValueId),
    External(ValueId),
    Loan(crate::LoanId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AffineFact {
    pub(crate) provenance: AffineProvenance,
    pub(crate) transferred: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OwnershipState {
    pub(crate) active_places: BTreeSet<crate::PlaceId>,
    pub(crate) owners: BTreeMap<crate::PlaceId, ValueId>,
    pub(crate) pending_drops: BTreeMap<crate::PlaceId, ValueId>,
    pub(crate) affine: BTreeMap<ValueId, AffineFact>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BorrowDefinition {
    pub(crate) block: BlockId,
    pub(crate) place: crate::PlaceId,
    pub(crate) loan: crate::LoanId,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LiveLoan {
    pub(crate) loan: crate::LoanId,
    pub(crate) kind: crate::BorrowKind,
    pub(crate) value: ValueId,
}
