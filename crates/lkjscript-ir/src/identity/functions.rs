use super::{instructions, types, writer::Writer, IdentityError};
use crate::*;

type IdentityResult<T = ()> = std::result::Result<T, IdentityError>;

pub(super) fn function(out: &mut Writer, value: &Function) -> IdentityResult {
    out.u32(value.id.raw())?;
    out.string(&value.name)?;
    types::signature(out, &value.signature)?;
    out.sequence(&value.places, place)?;
    out.u16(value.effects.bits())?;
    out.u32(value.entry.raw())?;
    out.sequence(&value.blocks, block)?;
    types::origin(out, value.origin)
}

fn place(out: &mut Writer, value: &PlaceMetadata) -> IdentityResult {
    out.u32(value.id.raw())?;
    out.u32(value.binding.raw())?;
    types::ssa_type(out, &value.ty)
}

fn block(out: &mut Writer, value: &Block) -> IdentityResult {
    out.u32(value.id.raw())?;
    out.sequence(&value.parameters, parameter)?;
    out.sequence(&value.instructions, instructions::instruction)?;
    terminator(out, &value.terminator)?;
    out.bool(value.metadata.loop_header)?;
    types::origin(out, value.metadata.origin)?;
    out.option(value.metadata.frame_state.as_ref(), frame_state)
}

fn parameter(out: &mut Writer, value: &BlockParameter) -> IdentityResult {
    out.u32(value.id.raw())?;
    types::ssa_type(out, &value.ty)?;
    out.option(value.owner_place.as_ref(), |out, place| {
        out.u32(place.raw())
    })?;
    types::origin(out, value.origin)
}

pub(super) fn frame_state(out: &mut Writer, value: &FrameState) -> IdentityResult {
    out.u32(value.bytecode_position)?;
    out.sequence(&value.locals, |out, local| {
        out.u32(local.binding.raw())?;
        out.u16(local.slot)?;
        out.u32(local.value.raw())
    })?;
    values(out, &value.operand_stack)
}

fn terminator(out: &mut Writer, value: &Terminator) -> IdentityResult {
    match value {
        Terminator::Branch { target, arguments } => {
            out.u8(0)?;
            out.u32(target.raw())?;
            values(out, arguments)
        }
        Terminator::ConditionalBranch {
            condition,
            true_target,
            true_arguments,
            false_target,
            false_arguments,
        } => {
            out.u8(1)?;
            out.u32(condition.raw())?;
            out.u32(true_target.raw())?;
            values(out, true_arguments)?;
            out.u32(false_target.raw())?;
            values(out, false_arguments)
        }
        Terminator::Return(value) => tagged_value(out, 2, *value),
        Terminator::Trap { value } => tagged_value(out, 3, *value),
        Terminator::Exit { code } => tagged_value(out, 4, *code),
        Terminator::Outcome { outcome, detail } => {
            out.u8(5)?;
            out.u8(match outcome {
                StructuredOutcome::DeadlineExceeded => 0,
                StructuredOutcome::ResourceLimitExceeded => 1,
                StructuredOutcome::HostFailure => 2,
            })?;
            out.option(detail.as_ref(), |out, value| out.u32(value.raw()))
        }
    }
}

pub(super) fn values(out: &mut Writer, values: &[ValueId]) -> IdentityResult {
    out.sequence(values, |out, value| out.u32(value.raw()))
}

fn tagged_value(out: &mut Writer, tag: u8, value: ValueId) -> IdentityResult {
    out.u8(tag)?;
    out.u32(value.raw())
}
