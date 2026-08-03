use crate::InstructionKind;

pub(super) fn exact_ownership_instruction_kind_equal(
    left: &InstructionKind,
    right: &InstructionKind,
) -> bool {
    if matches!(
        left,
        InstructionKind::StructuralPublish { .. }
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
            | InstructionKind::MemoryWitnessCompare { .. }
            | InstructionKind::MemoryWitnessDispose { .. }
    ) {
        return left == right;
    }
    match (left, right) {
        (
            InstructionKind::PlaceInit {
                place: left_place,
                value: left_value,
            },
            InstructionKind::PlaceInit {
                place: right_place,
                value: right_value,
            },
        ) => left_place == right_place && left_value == right_value,
        (InstructionKind::PlaceEnd { place: left }, InstructionKind::PlaceEnd { place: right }) => {
            left == right
        }
        (
            InstructionKind::EndBorrow {
                place: left_place,
                loan: left_loan,
                value: left_value,
            },
            InstructionKind::EndBorrow {
                place: right_place,
                loan: right_loan,
                value: right_value,
            },
        ) => left_place == right_place && left_loan == right_loan && left_value == right_value,
        (
            InstructionKind::Drop {
                place: left_place,
                value: left_value,
                glue: left_glue,
                kind: left_kind,
            },
            InstructionKind::Drop {
                place: right_place,
                value: right_value,
                glue: right_glue,
                kind: right_kind,
            },
        ) => {
            left_place == right_place
                && left_value == right_value
                && left_glue == right_glue
                && left_kind == right_kind
        }
        (
            InstructionKind::Move {
                place: left_place,
                value: left_value,
            },
            InstructionKind::Move {
                place: right_place,
                value: right_value,
            },
        ) => left_place == right_place && left_value == right_value,
        (
            InstructionKind::Borrow {
                place: left_place,
                loan: left_loan,
                kind: left_kind,
                value: left_value,
            },
            InstructionKind::Borrow {
                place: right_place,
                loan: right_loan,
                kind: right_kind,
                value: right_value,
            },
        ) => {
            left_place == right_place
                && left_loan == right_loan
                && left_kind == right_kind
                && left_value == right_value
        }
        _ => false,
    }
}
