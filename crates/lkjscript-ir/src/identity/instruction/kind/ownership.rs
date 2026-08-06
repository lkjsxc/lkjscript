use super::super::super::{function::borrow_kind, memory};
use super::super::{constant, place_value, representation_value, Encoder};
use crate::*;

pub(super) fn encode(out: &mut Encoder, value: &InstructionKind) {
    match value {
        InstructionKind::Constant(value) => {
            out.tag(0);
            constant(out, value);
        }
        InstructionKind::Copy(value) => {
            out.tag(1);
            out.u32(value.raw());
        }
        InstructionKind::PlaceInit { place, value } => {
            out.tag(2);
            place_value(out, *place, *value);
        }
        InstructionKind::PlaceEnd { place } => {
            out.tag(3);
            out.u32(place.raw());
        }
        InstructionKind::EndBorrow { place, loan, value } => {
            out.tag(4);
            out.u32(place.raw());
            out.u32(loan.raw());
            out.u32(value.raw());
        }
        InstructionKind::Drop {
            place,
            value,
            glue,
            kind,
        } => {
            out.tag(5);
            place_value(out, *place, *value);
            memory::drop_glue(out, *glue);
            out.tag(match kind {
                DropEventKind::ImplicitCleanup => 0,
                DropEventKind::ExplicitClose => 1,
            });
        }
        InstructionKind::Move { place, value } => {
            out.tag(6);
            place_value(out, *place, *value);
        }
        InstructionKind::Borrow {
            place,
            loan,
            kind,
            value,
        } => {
            out.tag(7);
            out.u32(place.raw());
            out.u32(loan.raw());
            borrow_kind(out, *kind);
            out.u32(value.raw());
        }
        InstructionKind::StructuralPublish {
            representation,
            value,
        } => {
            out.tag(8);
            representation_value(out, *representation, *value);
        }
        InstructionKind::DestinationCreate {
            representation,
            active_variant,
        } => {
            out.tag(9);
            out.u64(representation.raw());
            out.option(active_variant.as_ref(), |out, value| {
                out.fixed(&value.bytes())
            });
        }
        InstructionKind::DestinationFieldInit {
            destination,
            field,
            value,
        } => {
            out.tag(10);
            out.u32(destination.raw());
            out.u64(*field);
            out.u32(value.raw());
        }
        InstructionKind::DestinationFinish { destination } => {
            out.tag(11);
            out.u32(destination.raw());
        }
        InstructionKind::DestinationAbort { destination } => {
            out.tag(12);
            out.u32(destination.raw());
        }
        InstructionKind::AggregateFieldBorrow {
            representation,
            place,
            loan,
            field,
            value,
        } => {
            out.tag(13);
            out.u64(representation.raw());
            out.u32(place.raw());
            out.u32(loan.raw());
            out.u64(*field);
            out.u32(value.raw());
        }
        InstructionKind::AggregateTag {
            representation,
            value,
        } => {
            out.tag(14);
            representation_value(out, *representation, *value);
        }
        InstructionKind::AggregateConsumePayload {
            representation,
            place,
            variant,
            value,
        } => {
            out.tag(15);
            out.u64(representation.raw());
            out.option(place.as_ref(), |out, value| out.u32(value.raw()));
            out.fixed(&variant.bytes());
            out.u32(value.raw());
        }
        InstructionKind::StringUtf8View {
            representation,
            place,
            loan,
            value,
        } => {
            out.tag(16);
            out.u64(representation.raw());
            out.u32(place.raw());
            out.u32(loan.raw());
            out.u32(value.raw());
        }
        InstructionKind::StructuralCopy {
            representation,
            value,
        } => {
            out.tag(17);
            representation_value(out, *representation, *value);
        }
        InstructionKind::MemoryWitnessIndependentOwner { parameter, value } => {
            out.tag(18);
            out.string(parameter);
            out.u32(value.raw());
        }
        InstructionKind::MemoryWitnessDispose { parameter, value } => {
            out.tag(19);
            out.string(parameter);
            out.u32(value.raw());
        }
        InstructionKind::MemoryWitnessCompare {
            parameter,
            left,
            right,
        } => {
            out.tag(20);
            out.string(parameter);
            out.u32(left.raw());
            out.u32(right.raw());
        }
        _ => out.fail("verified SSA identity ownership instruction partition failed"),
    }
}
