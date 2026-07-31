pub(in crate::lower) fn lower_trap_message(
    function: &Function,
    value: ValueId,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    builder: &mut FunctionBuilder,
) -> Result<(), LoweringError> {
    let ValueType::StructuralOwner(value_type) = value_type(value_types, value)? else {
        return Err(invalid_structural(
            "dynamic trap string has no compiler-produced native structural owner",
        ));
    };
    if value_type.kind() != lkjscript_native::StructuralKind::String {
        return Err(invalid_structural("dynamic trap payload is not a string"));
    }
    let input = read_value(builder, block, locals, value, function.id)?;
    let _ = structural_call(
        builder,
        block,
        lkjscript_native::StructuralOperation::CaptureTrap(value_type),
        vec![input],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_structural_runtime(
    function: &Function,
    instruction: &Instruction,
    operation: RuntimeOp,
    arguments: &[ValueId],
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    layouts: &LayoutInterner,
    static_bytes: &HashMap<Vec<u8>, lkjscript_native::StaticBytesIdentity>,
    builder: &mut FunctionBuilder,
) -> NativeResult {
    match operation {
        RuntimeOp::EmptyStr => {
            let ValueType::StaticString(value_type) = value_type(value_types, instruction.id)?
            else {
                return Err(invalid_structural("empty string native type is invalid"));
            };
            builder
                .static_string_const(
                    block,
                    *static_bytes.get(&Vec::new()).ok_or_else(|| {
                        invalid_structural("empty static string identity is absent")
                    })?,
                    value_type,
                )
                .map_err(LoweringError::backend)
        }
        RuntimeOp::StrFromI64 => {
            let [value] = arguments else {
                return Err(invalid_structural(
                    "integer string conversion arity is invalid",
                ));
            };
            let value_type = structural_type(layouts.structural(), &instruction.ty)?;
            let input = read_value(builder, block, locals, *value, function.id)?;
            structural_call(
                builder,
                block,
                lkjscript_native::StructuralOperation::PublishFormattedI64(value_type),
                vec![input],
            )
        }
        RuntimeOp::StrLen => {
            let [value] = arguments else {
                return Err(invalid_structural("string length arity is invalid"));
            };
            let value_type = structural_type(layouts.structural(), source_type(function, *value)?)?;
            let input = observe_value(function, *value, block, locals, builder)?;
            structural_call(
                builder,
                block,
                lkjscript_native::StructuralOperation::PayloadLength(value_type),
                vec![input],
            )
        }
        _ => Err(invalid_structural("runtime operation is not structural")),
    }
}
