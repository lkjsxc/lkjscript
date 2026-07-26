use crate::optimize::*;
use crate::{InstructionKind, Program};

pub(crate) fn preflight_program(
    program: &Program,
    budget: &mut Budget,
) -> Result<ProgramShape, OptimizationError> {
    let mut counter = ShapeCounter::new(budget);
    counter.add_bounded(
        ShapeField::Functions,
        u64::try_from(program.functions.len()).map_err(|_| budget_error())?,
    )?;
    let top_metadata = program
        .sources
        .len()
        .checked_add(program.products.len())
        .and_then(|value| value.checked_add(program.enums.len()))
        .and_then(|value| value.checked_add(program.traits.len()))
        .and_then(|value| value.checked_add(program.implementations.len()))
        .ok_or_else(budget_error)?;
    counter.add_bounded(
        ShapeField::MetadataItems,
        u64::try_from(top_metadata).map_err(|_| budget_error())?,
    )?;
    counter.add_bounded(
        ShapeField::StringAndMetadataBytes,
        u64::try_from(top_metadata)
            .map_err(|_| budget_error())?
            .saturating_mul(8),
    )?;
    for source in &program.sources {
        counter.add_string(&source.path)?;
    }
    for product in &program.products {
        counter.add_string(&product.name)?;
        for field in &product.fields {
            counter.add_metadata()?;
            counter.add_string(&field.name)?;
            counter.add_type(&field.ty)?;
        }
    }
    for definition in &program.enums {
        counter.add_string(&definition.name)?;
        counter.add_metadata()?;
        for parameter in &definition.type_parameters {
            counter.add_string(parameter)?;
            counter.add_metadata()?;
        }
        for variant in &definition.variants {
            counter.add_string(&variant.name)?;
            counter.add_metadata()?;
            for field in &variant.fields {
                counter.add_string(&field.name)?;
                counter.add_metadata()?;
                counter.add_type(&field.ty)?;
            }
        }
    }
    for trait_metadata in &program.traits {
        counter.add_string(&trait_metadata.name)?;
    }
    for function in &program.functions {
        counter.add_string(&function.name)?;
        counter.add_signature(&function.signature)?;
        for place in &function.places {
            counter.add_metadata()?;
            counter.add_type(&place.ty)?;
        }
        counter.add_bounded(
            ShapeField::Blocks,
            u64::try_from(function.blocks.len()).map_err(|_| budget_error())?,
        )?;
        for block in &function.blocks {
            counter.add_metadata()?;
            if let Some(frame) = &block.metadata.frame_state {
                counter.add_frame(frame)?;
            }
            counter.add_bounded(
                ShapeField::Parameters,
                u64::try_from(block.parameters.len()).map_err(|_| budget_error())?,
            )?;
            for parameter in &block.parameters {
                counter.add_metadata()?;
                counter.add_type(&parameter.ty)?;
            }
            for instruction in &block.instructions {
                counter.add_instruction(instruction)?;
            }
            counter.add_bounded(
                ShapeField::Operands,
                terminator_operand_count(&block.terminator)?,
            )?;
        }
    }
    Ok(counter.shape)
}

pub(crate) fn instruction_operand_count(kind: &InstructionKind) -> Result<u64, OptimizationError> {
    let count = match kind {
        InstructionKind::Constant(_)
        | InstructionKind::PlaceEnd { .. }
        | InstructionKind::FunctionRef(_) => 0,
        InstructionKind::Copy(_)
        | InstructionKind::PlaceInit { .. }
        | InstructionKind::Move { .. }
        | InstructionKind::Borrow { .. }
        | InstructionKind::ProductField { .. }
        | InstructionKind::EnumIsVariant { .. }
        | InstructionKind::EnumField { .. } => 1,
        InstructionKind::Runtime { arguments, .. }
        | InstructionKind::Call {
            target: crate::CallTarget::Direct(_),
            arguments,
            ..
        } => arguments.len(),
        InstructionKind::Call {
            target: crate::CallTarget::Indirect(_),
            arguments,
            ..
        } => arguments.len().checked_add(1).ok_or_else(budget_error)?,
        InstructionKind::ProductValue { fields, .. }
        | InstructionKind::EnumValue { fields, .. } => fields.len(),
        InstructionKind::WithProductField { .. } => 2,
    };
    u64::try_from(count).map_err(|_| budget_error())
}

pub(crate) fn terminator_operand_count(
    terminator: &crate::Terminator,
) -> Result<u64, OptimizationError> {
    let count = match terminator {
        crate::Terminator::Branch { arguments, .. } => arguments.len(),
        crate::Terminator::ConditionalBranch {
            true_arguments,
            false_arguments,
            ..
        } => 1_usize
            .checked_add(true_arguments.len())
            .and_then(|value| value.checked_add(false_arguments.len()))
            .ok_or_else(budget_error)?,
        crate::Terminator::Return(_)
        | crate::Terminator::Trap { .. }
        | crate::Terminator::Exit { .. } => 1,
        crate::Terminator::Outcome { detail, .. } => usize::from(detail.is_some()),
    };
    u64::try_from(count).map_err(|_| budget_error())
}
