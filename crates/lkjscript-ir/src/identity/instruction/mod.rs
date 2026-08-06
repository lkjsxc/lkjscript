mod kind;

use super::{encoder::Encoder, metadata, types};
use crate::*;

pub(super) fn instruction(out: &mut Encoder, value: &Instruction) {
    let Instruction {
        id,
        ty,
        kind,
        metadata: instruction_metadata,
    } = value;
    out.wide(id.raw());
    types::ty(out, ty);
    kind::kind_value(out, kind);
    instruction_meta(out, instruction_metadata);
}

fn constant(out: &mut Encoder, value: &Constant) {
    match value {
        Constant::Unit => out.tag(0),
        Constant::Bool(value) => {
            out.tag(1);
            out.bool(*value);
        }
        Constant::I64(value) => {
            out.tag(2);
            out.i64(*value);
        }
        Constant::F64(value) => {
            out.tag(3);
            out.u64(value.to_bits());
        }
        Constant::Str(value) => {
            out.tag(4);
            out.string(value);
        }
        Constant::StaticBytes(value) => {
            out.tag(5);
            out.bytes(value);
        }
        Constant::Symbol(value) => {
            out.tag(6);
            out.string(value);
        }
        Constant::EmptyList => out.tag(7),
    }
}
fn instruction_meta(out: &mut Encoder, value: &InstructionMetadata) {
    let InstructionMetadata {
        origin,
        effects,
        failure,
        failure_cleanup,
        frame_state: frame,
    } = value;
    metadata::origin(out, origin);
    metadata::effects(out, *effects);
    out.tag(match failure {
        FailureBehavior::None => 0,
        FailureBehavior::Trap => 1,
        FailureBehavior::StructuredOutcome => 2,
        FailureBehavior::TrapOrOutcome => 3,
    });
    out.option(failure_cleanup.as_ref(), |out, roots| {
        out.option(roots.loans.as_ref(), |out, value| out.u64(value.raw()));
        out.option(roots.unplaced.as_ref(), |out, value| out.u64(value.raw()));
        out.option(roots.places.as_ref(), |out, value| out.u64(value.raw()));
    });
    out.option(frame.as_ref(), frame_state);
}
pub(super) fn frame_state(out: &mut Encoder, value: &FrameState) {
    let FrameState {
        bytecode_position,
        locals,
        operand_stack,
    } = value;
    out.wide(*bytecode_position);
    out.sequence(locals, |out, value| {
        let FrameLocal {
            binding,
            slot,
            value,
        } = value;
        out.wide(binding.raw());
        out.u64(*slot);
        out.wide(value.raw());
    });
    ids(out, operand_stack);
}
fn place_value(out: &mut Encoder, place: PlaceId, value: ValueId) {
    out.wide(place.raw());
    out.wide(value.raw());
}
fn representation_value(
    out: &mut Encoder,
    representation: StructuralRepresentationId,
    value: ValueId,
) {
    out.u64(representation.raw());
    out.wide(value.raw());
}
fn scalar(out: &mut Encoder, tag: u16, value: ValueId) {
    out.tag(tag);
    out.wide(value.raw());
}
fn ids(out: &mut Encoder, values: &[ValueId]) {
    out.sequence(values, |out, value| out.wide(value.raw()));
}
fn product_field(
    out: &mut Encoder,
    tag: u16,
    product: ProductId,
    field: u64,
    value: ValueId,
    replacement: Option<ValueId>,
) {
    out.tag(tag);
    out.u64(product.raw());
    out.u64(field);
    out.wide(value.raw());
    out.option(replacement.as_ref(), |out, value| out.wide(value.raw()));
}
fn enum_header(out: &mut Encoder, id: EnumId, variant: VariantId, layout: RuntimeLayoutId) {
    out.fixed(&id.bytes());
    out.fixed(&variant.bytes());
    out.fixed(&layout.bytes());
}
