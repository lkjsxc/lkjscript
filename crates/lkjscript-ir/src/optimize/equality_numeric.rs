use crate::InstructionKind;

pub(crate) fn exact_numeric_instruction_kind_equal(
    left: &InstructionKind,
    right: &InstructionKind,
) -> bool {
    match (left, right) {
        (
            InstructionKind::F64FromI64Exact { value: left },
            InstructionKind::F64FromI64Exact { value: right },
        )
        | (
            InstructionKind::F64FromI64Rounded { value: left },
            InstructionKind::F64FromI64Rounded { value: right },
        )
        | (
            InstructionKind::I64FromF64Exact { value: left },
            InstructionKind::I64FromF64Exact { value: right },
        )
        | (
            InstructionKind::I64FromF64Trunc { value: left },
            InstructionKind::I64FromF64Trunc { value: right },
        ) => left == right,
        _ => false,
    }
}
