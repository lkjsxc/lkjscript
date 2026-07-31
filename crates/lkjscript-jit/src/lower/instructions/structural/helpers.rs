fn observe_value(
    function: &Function,
    value: ValueId,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let local = value_local(locals, value, function.id)?;
    builder
        .observe_local(block, local)
        .map_err(LoweringError::backend)
}

pub(in crate::lower) fn structural_call(
    builder: &mut FunctionBuilder,
    block: lkjscript_native::BlockId,
    operation: lkjscript_native::StructuralOperation,
    arguments: Vec<lkjscript_native::ValueId>,
) -> NativeResult {
    let descriptor = lkjscript_native::StructuralCallDescriptor::new(operation)
        .map_err(LoweringError::backend)?;
    builder
        .structural_call(block, descriptor, arguments)
        .map_err(LoweringError::backend)
}

fn structural_type(
    catalog: &StructuralCatalog,
    ty: &SsaType,
) -> Result<lkjscript_native::StructuralTypeIdentity, LoweringError> {
    catalog
        .value_type(ty)
        .ok_or_else(|| invalid_structural("structural native type identity is missing"))
}

fn source_type(function: &Function, value: ValueId) -> Result<&SsaType, LoweringError> {
    function
        .blocks
        .iter()
        .find_map(|block| {
            block
                .parameters
                .iter()
                .find(|parameter| parameter.id == value)
                .map(|parameter| &parameter.ty)
                .or_else(|| {
                    block
                        .instructions
                        .iter()
                        .find(|instruction| instruction.id == value)
                        .map(|instruction| &instruction.ty)
                })
        })
        .ok_or_else(|| invalid_structural("structural source type is missing"))
}

fn publish_operation(
    function: &Function,
    value: ValueId,
    root_type: lkjscript_native::StructuralTypeIdentity,
    value_types: &[ValueType],
) -> Result<lkjscript_native::StructuralOperation, LoweringError> {
    let source = value_type(value_types, value)?;
    Ok(match source {
        ValueType::StaticString(_) => lkjscript_native::StructuralOperation::PublishStatic {
            value_type: root_type,
            payload: lkjscript_native::StructuralPayloadKind::String,
        },
        ValueType::StaticBytes => lkjscript_native::StructuralOperation::PublishStatic {
            value_type: root_type,
            payload: lkjscript_native::StructuralPayloadKind::Bytes,
        },
        ValueType::I64 => lkjscript_native::StructuralOperation::PublishI64(root_type),
        ValueType::Unique(unique) => lkjscript_native::StructuralOperation::PublishUnique {
            value_type: root_type,
            payload: match root_type.kind() {
                lkjscript_native::StructuralKind::Bytes => {
                    lkjscript_native::StructuralPayloadKind::Bytes
                }
                lkjscript_native::StructuralKind::ByteVector => {
                    lkjscript_native::StructuralPayloadKind::ByteVector
                }
                _ => {
                    return Err(invalid_structural(
                        "unique structural payload kind is invalid",
                    ))
                }
            },
            unique,
        },
        ValueType::StructuralOwner(actual) if actual == root_type => {
            lkjscript_native::StructuralOperation::Move(root_type)
        }
        _ => {
            return Err(LoweringError::new(
                LoweringFailureCode::InvalidFunction,
                Some(function.id),
                "structural publish source has no exact native payload route",
            ))
        }
    })
}
