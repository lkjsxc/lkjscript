use super::*;
mod calls;
mod constants;
mod failure_cleanup;
mod output;
mod products;
mod runtime_bytes;
mod structural;
mod structural_dispatch;
mod unique;
include!("imports.rs");

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_instruction(
    _program: &lkjscript_ir::Program,
    function: &Function,
    instruction: &Instruction,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    native_functions: &[(FunctionId, lkjscript_native::FunctionId)],
    layouts: &LayoutInterner,
    static_bytes: &HashMap<Vec<u8>, lkjscript_native::StaticBytesIdentity>,
    builder: &mut FunctionBuilder,
) -> Result<(), LoweringError> {
    if structural_dispatch::lower_instruction(
        function,
        instruction,
        block,
        locals,
        value_types,
        layouts,
        static_bytes,
        builder,
    )? {
        return Ok(());
    }
    let output = match &instruction.kind {
        InstructionKind::Constant(constant) => lower_constant(
            function,
            instruction,
            constant,
            block,
            value_types,
            static_bytes,
            builder,
        )?,
        InstructionKind::Copy(value) => {
            let value = read_value(builder, block, locals, *value, function.id)?;
            Ok(value)
        }
        InstructionKind::Move { value, .. } => {
            lower_move(function, *value, block, locals, value_types, builder)
        }
        InstructionKind::PlaceInit { .. } | InstructionKind::PlaceEnd { .. } => builder.unit(block),
        InstructionKind::EndBorrow { value, .. } => {
            lower_end_borrow(function, *value, block, locals, value_types, builder)
        }
        InstructionKind::Drop { value, glue, .. } => {
            lower_drop(function, *value, *glue, block, locals, builder)
        }
        InstructionKind::Runtime {
            operation,
            arguments,
            ..
        } => lower_runtime(
            function,
            *operation,
            arguments,
            RuntimeLoweringContext {
                block,
                locals,
                value_types,
                result_type: value_type(value_types, instruction.id)?,
            },
            builder,
        ),
        InstructionKind::F64FromI64Exact { .. }
        | InstructionKind::F64FromI64Rounded { .. }
        | InstructionKind::I64FromF64Exact { .. }
        | InstructionKind::I64FromF64Trunc { .. } => Ok(lower_numeric_conversion(
            function,
            instruction,
            block,
            locals,
            value_types,
            layouts,
            builder,
        )?),
        InstructionKind::Call {
            target: CallTarget::Direct(callee),
            arguments,
            ..
        } => lower_direct_call(
            function,
            instruction,
            *callee,
            arguments,
            block,
            locals,
            value_types,
            layouts,
            native_functions,
            builder,
        )?,
        InstructionKind::Call {
            target: CallTarget::Indirect(_),
            ..
        } => return indirect_call(function.id),
        InstructionKind::ProductValue { .. }
        | InstructionKind::ProductField { .. }
        | InstructionKind::WithProductField { .. } => {
            return products::lower_product_instruction(
                function,
                instruction,
                block,
                locals,
                value_types,
                layouts,
                builder,
            );
        }
        InstructionKind::EnumValue { .. }
        | InstructionKind::EnumIsVariant { .. }
        | InstructionKind::EnumField { .. } => {
            return unsupported_operation(
                function.id,
                "enum instruction without structural metadata and operations",
            );
        }
        InstructionKind::Borrow { kind, value, .. } => {
            lower_borrow(function, *kind, *value, block, locals, value_types, builder)
        }
        InstructionKind::StructuralPublish { .. }
        | InstructionKind::DestinationCreate { .. }
        | InstructionKind::DestinationFieldInit { .. }
        | InstructionKind::DestinationFinish { .. }
        | InstructionKind::DestinationAbort { .. }
        | InstructionKind::AggregateFieldBorrow { .. }
        | InstructionKind::AggregateTag { .. }
        | InstructionKind::AggregateConsumePayload { .. }
        | InstructionKind::StringUtf8View { .. }
        | InstructionKind::StructuralCopy { .. } => {
            let lowered = structural_dispatch::lower_instruction(
                function,
                instruction,
                block,
                locals,
                value_types,
                layouts,
                static_bytes,
                builder,
            )?;
            if lowered {
                return Ok(());
            }
            return unsupported_operation(function.id, "structural instruction");
        }
        InstructionKind::FunctionRef(_) => {
            return unsupported_operation(function.id, "first-class function reference")
        }
    }
    .map_err(LoweringError::backend)?;
    write_instruction_output(
        function,
        instruction,
        block,
        locals,
        value_types,
        layouts,
        output,
        builder,
    )
}
