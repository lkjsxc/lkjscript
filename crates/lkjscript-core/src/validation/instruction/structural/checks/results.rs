fn unique_field_result(
    field: StructuralFieldMetadata,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    match field.runtime_type.map(|item| item.kind) {
        Some(crate::StructuralKind::Bytes) => {
            Ok(Kind::Bytes(fresh_identity(proto, instruction, 2)?))
        }
        Some(crate::StructuralKind::ByteVector) => {
            Ok(Kind::ByteVector(fresh_identity(proto, instruction, 2)?))
        }
        _ => fail(
            proto,
            instruction,
            "unique payload consume lacks exact bytes ownership metadata",
        ),
    }
}

fn exact_copy_kind(expected: crate::StructuralKind, actual: Kind) -> bool {
    matches!(
        (expected, actual),
        (crate::StructuralKind::Unit, Kind::Unit)
            | (crate::StructuralKind::Bool, Kind::Bool)
            | (crate::StructuralKind::I64, Kind::I64)
            | (crate::StructuralKind::F64, Kind::F64)
            | (crate::StructuralKind::Static, Kind::Symbol)
    )
}

fn exact_copy_result(expected: crate::StructuralType) -> Option<Kind> {
    match expected.kind {
        crate::StructuralKind::Unit => Some(Kind::Unit),
        crate::StructuralKind::Bool => Some(Kind::Bool),
        crate::StructuralKind::I64 => Some(Kind::I64),
        crate::StructuralKind::F64 => Some(Kind::F64),
        crate::StructuralKind::Static => Some(Kind::Symbol),
        crate::StructuralKind::String
        | crate::StructuralKind::Path
        | crate::StructuralKind::Bytes
        | crate::StructuralKind::ByteVector
        | crate::StructuralKind::Product
        | crate::StructuralKind::Enum => None,
    }
}

fn fresh_identity(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    increment: u32,
) -> Result<u32> {
    u32::try_from(instruction.offset())
        .ok()
        .and_then(|offset| offset.checked_add(increment))
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "structural owner identity overflow",
            )
        })
}

fn fail<T>(proto: &FunctionProto, instruction: DecodedInstruction, message: &str) -> Result<T> {
    Err(instruction_error(
        proto,
        instruction.op(),
        instruction.offset(),
        message,
    ))
}
