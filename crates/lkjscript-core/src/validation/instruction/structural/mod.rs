use super::{instruction_error, types::*, Kind, State};
use crate::validation::{StructuralDestinationState, UniquePlaceState};
use crate::{
    Chunk, DecodedInstruction, FunctionProto, Op, Result, StructuralFieldMetadata,
    StructuralFieldRoute, StructuralRepresentationId, StructuralValueCategory,
};

pub(super) fn apply(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    match instruction.op() {
        Op::StoreStructuralLocal => store_local(proto, instruction, state),
        Op::TakeStructuralLocal => take_local(proto, instruction, state),
        Op::LoadStructuralViewLocal => load_view(proto, instruction, state),
        Op::EndStructuralBorrowLocal => end_view(proto, instruction, state),
        Op::LoadStructuralOwnerLocal => load_owner_ref(proto, instruction, state),
        Op::StructuralPlaceInit => place_init(proto, instruction, state),
        Op::StructuralMove => move_owner(proto, instruction, state),
        Op::StructuralDropPlace => drop_owner(proto, instruction, state),
        Op::StructuralPlaceEnd => place_end(proto, instruction, state),
        Op::StructuralBorrow | Op::StructuralBorrowMut => borrow(chunk, proto, instruction, state),
        Op::StructuralPublish => publish(chunk, proto, instruction, state),
        Op::StructuralDestinationCreate => destination_create(chunk, proto, instruction, state),
        Op::StructuralDestinationFieldInit => {
            destination_field_init(chunk, proto, instruction, state)
        }
        Op::StructuralDestinationFinish => destination_finish(chunk, proto, instruction, state),
        Op::StructuralDestinationAbort => destination_abort(chunk, proto, instruction, state),
        Op::StructuralAggregateFieldBorrow => {
            aggregate_field_borrow(chunk, proto, instruction, state)
        }
        Op::StructuralAggregateTag => aggregate_tag(chunk, proto, instruction, state),
        Op::StructuralAggregateConsumePayload => {
            aggregate_consume_payload(chunk, proto, instruction, state)
        }
        Op::StructuralStringUtf8View => string_utf8_view(chunk, proto, instruction, state),
        Op::StructuralCopy => structural_copy(chunk, proto, instruction, state),
        _ => unreachable!("structural opcode family checked"),
    }
}

include!("locals.rs");
include!("places.rs");
include!("borrows.rs");
include!("destination.rs");
include!("aggregate.rs");
include!("copy.rs");
include!("lookup.rs");
include!("checks.rs");
