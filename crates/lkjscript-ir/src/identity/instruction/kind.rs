use super::Encoder;
use crate::InstructionKind;

mod execution;
mod ownership;

pub(super) fn kind_value(out: &mut Encoder, value: &InstructionKind) {
    match value {
        InstructionKind::Constant(_)
        | InstructionKind::Copy(_)
        | InstructionKind::PlaceInit { .. }
        | InstructionKind::PlaceEnd { .. }
        | InstructionKind::EndBorrow { .. }
        | InstructionKind::Drop { .. }
        | InstructionKind::Move { .. }
        | InstructionKind::Borrow { .. }
        | InstructionKind::StructuralPublish { .. }
        | InstructionKind::DestinationCreate { .. }
        | InstructionKind::DestinationFieldInit { .. }
        | InstructionKind::DestinationFinish { .. }
        | InstructionKind::DestinationAbort { .. }
        | InstructionKind::AggregateFieldBorrow { .. }
        | InstructionKind::AggregateTag { .. }
        | InstructionKind::AggregateConsumePayload { .. }
        | InstructionKind::StringUtf8View { .. }
        | InstructionKind::StructuralCopy { .. }
        | InstructionKind::MemoryWitnessIndependentOwner { .. }
        | InstructionKind::MemoryWitnessDispose { .. } => ownership::encode(out, value),
        InstructionKind::FunctionRef(_)
        | InstructionKind::Runtime { .. }
        | InstructionKind::F64FromI64Exact { .. }
        | InstructionKind::F64FromI64Rounded { .. }
        | InstructionKind::I64FromF64Exact { .. }
        | InstructionKind::I64FromF64Trunc { .. }
        | InstructionKind::Call { .. }
        | InstructionKind::ProductValue { .. }
        | InstructionKind::ProductField { .. }
        | InstructionKind::WithProductField { .. }
        | InstructionKind::EnumValue { .. }
        | InstructionKind::EnumIsVariant { .. }
        | InstructionKind::EnumField { .. } => execution::encode(out, value),
    }
}
