use super::*;

pub(super) fn lower_numeric_conversion(
    function: &Function,
    instruction: &Instruction,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    _value_types: &[ValueType],
    layouts: &LayoutInterner,
    builder: &mut FunctionBuilder,
) -> Result<lkjscript_native::ValueId, LoweringError> {
    if let InstructionKind::F64FromI64Rounded { value } = instruction.kind {
        let argument = read_value(builder, block, locals, value, function.id)?;
        return builder
            .i64_to_f64(block, argument)
            .map_err(LoweringError::backend);
    }
    let SsaType::Enum { id, arguments } = &instruction.ty else {
        return unsupported_operation(function.id, "numeric conversion result is not Result");
    };
    if id.bytes() != lkjscript_ir::prelude_contract::RESULT_ID || arguments.len() != 2 {
        return unsupported_operation(function.id, "numeric conversion Result is malformed");
    }
    let (value, kind) = match instruction.kind {
        InstructionKind::F64FromI64Exact { value } => (
            value,
            lkjscript_native::StructuralNumericConversion::F64FromI64Exact,
        ),
        InstructionKind::I64FromF64Exact { value } => (
            value,
            lkjscript_native::StructuralNumericConversion::I64FromF64Exact,
        ),
        InstructionKind::I64FromF64Trunc { value } => (
            value,
            lkjscript_native::StructuralNumericConversion::I64FromF64Truncating,
        ),
        _ => return unsupported_operation(function.id, "numeric conversion mismatch"),
    };
    let catalog = layouts.structural();
    let result_type = catalog.type_id(&instruction.ty)?;
    let error_type = catalog.type_id(&arguments[1])?;
    let success = catalog.aggregate(
        result_type,
        Some(lkjscript_ir::VariantId::new(
            lkjscript_ir::prelude_contract::RESULT_OK_ID,
        )),
    )?;
    let failure = catalog.aggregate(
        result_type,
        Some(lkjscript_ir::VariantId::new(
            lkjscript_ir::prelude_contract::RESULT_ERR_ID,
        )),
    )?;
    let errors = lkjscript_ir::prelude_contract::NUMERIC_ERROR_VARIANTS
        .into_iter()
        .map(|variant| catalog.aggregate(error_type, Some(lkjscript_ir::VariantId::new(variant))))
        .collect::<Result<Vec<_>, _>>()?;
    let argument = read_value(builder, block, locals, value, function.id)?;
    structural_call(
        builder,
        block,
        lkjscript_native::StructuralOperation::NumericConversion {
            kind,
            success,
            failure,
            errors,
        },
        vec![argument],
    )
}
