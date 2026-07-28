use super::*;

mod runtime;
use runtime::*;

pub(super) fn preflight_function(
    program: &lkjscript_ir::Program,
    function: &Function,
    layouts: &LayoutInterner,
    domain: LoweringDomain,
) -> Result<(), LoweringError> {
    lower_signature(function.id, &function.signature, layouts)?;
    for ty in function
        .signature
        .parameters
        .iter()
        .chain(std::iter::once(function.signature.result.as_ref()))
    {
        match domain {
            LoweringDomain::ResourceIsland => require_resource_island_type(function.id, ty)?,
            LoweringDomain::UniqueIsland => require_unique_island_type(function.id, ty)?,
            LoweringDomain::Legacy => {}
        }
    }
    if function.id.raw() >= 64 {
        return Err(LoweringError::new(
            LoweringFailureCode::UnsupportedSignature,
            Some(function.id),
            "native entry accounting supports at most 64 dense source functions",
        ));
    }
    for block in &function.blocks {
        for parameter in &block.parameters {
            lower_type(function.id, &parameter.ty, layouts)?;
            match domain {
                LoweringDomain::ResourceIsland => {
                    require_resource_island_type(function.id, &parameter.ty)?;
                }
                LoweringDomain::UniqueIsland => {
                    require_unique_island_type(function.id, &parameter.ty)?;
                }
                LoweringDomain::Legacy => {}
            }
        }
        for instruction in &block.instructions {
            lower_type(function.id, &instruction.ty, layouts)?;
            match domain {
                LoweringDomain::ResourceIsland => {
                    require_resource_island_type(function.id, &instruction.ty)?;
                }
                LoweringDomain::UniqueIsland => {
                    require_unique_island_type(function.id, &instruction.ty)?;
                }
                LoweringDomain::Legacy => {}
            }
            match &instruction.kind {
                InstructionKind::Constant(constant) => match constant {
                    Constant::Unit
                    | Constant::Bool(_)
                    | Constant::I64(_)
                    | Constant::F64(_)
                    | Constant::Str(_)
                    | Constant::EmptyList => {}
                    Constant::StaticBytes(_) => {
                        return unsupported_operation(function.id, "immutable bytes constant")
                    }
                    Constant::Symbol(_) => {
                        return unsupported_operation(function.id, "Symbol constant")
                    }
                },
                InstructionKind::Copy(_)
                    if matches!(instruction.ty, SsaType::Resource(_) | SsaType::ByteVector) =>
                {
                    return unsupported_operation(function.id, "copy of affine value");
                }
                InstructionKind::Copy(_) => {}
                InstructionKind::PlaceInit { .. }
                | InstructionKind::PlaceEnd { .. }
                | InstructionKind::EndBorrow { .. }
                | InstructionKind::Drop { .. }
                | InstructionKind::Move { .. }
                    if domain == LoweringDomain::ResourceIsland => {}
                InstructionKind::PlaceInit { .. }
                | InstructionKind::PlaceEnd { .. }
                | InstructionKind::EndBorrow { .. }
                | InstructionKind::Drop {
                    glue: lkjscript_ir::DropGlueIdentity::ByteVector,
                    ..
                }
                | InstructionKind::Move { .. }
                | InstructionKind::Borrow { .. }
                    if domain == LoweringDomain::UniqueIsland => {}
                InstructionKind::Drop { .. } if domain == LoweringDomain::UniqueIsland => {
                    return unsupported_operation(function.id, "non-byte-vector drop glue");
                }
                InstructionKind::PlaceInit { .. }
                | InstructionKind::PlaceEnd { .. }
                | InstructionKind::EndBorrow { .. }
                | InstructionKind::Drop { .. }
                | InstructionKind::Move { .. }
                | InstructionKind::Borrow { .. } => {
                    return unsupported_operation(function.id, "ownership/reference operation");
                }
                InstructionKind::Runtime { operation, .. }
                    if supported_runtime(*operation, domain) => {}
                InstructionKind::F64FromI64Exact { .. }
                | InstructionKind::F64FromI64Rounded { .. }
                | InstructionKind::I64FromF64Exact { .. }
                | InstructionKind::I64FromF64Trunc { .. } => {}
                InstructionKind::Call {
                    target: CallTarget::Direct(_),
                    signature,
                    ..
                } => {
                    lower_signature(function.id, signature, layouts)?;
                }
                InstructionKind::Call {
                    target: CallTarget::Indirect(_),
                    ..
                } => {
                    return Err(LoweringError::new(
                        LoweringFailureCode::IndirectCall,
                        Some(function.id),
                        "indirect native calls are unsupported",
                    ));
                }
                InstructionKind::FunctionRef(_) => {
                    return unsupported_operation(function.id, "first-class function reference");
                }
                InstructionKind::Runtime { operation, .. } => {
                    return unsupported_operation(
                        function.id,
                        &format!("runtime operation {operation:?}"),
                    );
                }
                InstructionKind::ProductValue { .. }
                | InstructionKind::ProductField { .. }
                | InstructionKind::WithProductField { .. } => {}
                InstructionKind::EnumValue { .. }
                | InstructionKind::EnumIsVariant { .. }
                | InstructionKind::EnumField { .. } => {
                    preflight_enum_instruction(program, function, instruction, layouts)?;
                }
            }
        }
        if let Terminator::Outcome {
            detail: Some(_), ..
        } = block.terminator
        {
            return unsupported_operation(function.id, "structured outcome reference detail");
        }
    }
    Ok(())
}

pub(super) fn unsupported_operation<T>(
    function: FunctionId,
    operation: &str,
) -> Result<T, LoweringError> {
    Err(LoweringError::new(
        LoweringFailureCode::UnsupportedOperation,
        Some(function),
        format!("{operation} is unsupported by allocation-free scalar native code"),
    ))
}
