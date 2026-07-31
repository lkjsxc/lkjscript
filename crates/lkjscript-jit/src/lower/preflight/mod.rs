use super::*;
mod bytes;
mod bytes_edges;
mod bytes_graph;
mod failure_cleanup;
mod runtime;
mod structural;
use bytes::{bytes_mode_error, preflight_bytes_runtime};
pub(in crate::lower) use bytes::{BytesMode, BytesModes};
use bytes_edges::*;
use bytes_graph::*;
use failure_cleanup::preflight_failure_cleanups;
use runtime::*;
pub(in crate::lower) use structural::{explicit_structural, unsupported_operation};
use structural::{preflight_instruction_type, require_domain_type};
pub(super) fn preflight_function(
    program: &lkjscript_ir::Program,
    function: &Function,
    layouts: &LayoutInterner,
    modes: &BytesModes,
    domain: LoweringDomain,
) -> Result<(), LoweringError> {
    lower_signature(function, modes, layouts)?;
    preflight_failure_cleanups(function)?;
    for ty in function
        .signature
        .parameters
        .iter()
        .chain(std::iter::once(function.signature.result.as_ref()))
    {
        require_domain_type(function.id, ty, layouts, domain)?;
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
            lower_value_type(function.id, parameter.id, &parameter.ty, modes, layouts)?;
            require_domain_type(function.id, &parameter.ty, layouts, domain)?;
        }
        for instruction in &block.instructions {
            let static_trap = static_trap_message(function, instruction.id).is_some();
            preflight_instruction_type(function, instruction, layouts, modes, domain)?;
            match &instruction.kind {
                InstructionKind::Constant(Constant::Str(_)) if static_trap => {}
                InstructionKind::Constant(constant) => match constant {
                    Constant::Unit
                    | Constant::Bool(_)
                    | Constant::I64(_)
                    | Constant::F64(_)
                    | Constant::EmptyList => {}
                    Constant::Str(_)
                        if domain == LoweringDomain::StructuralIsland
                            && layouts.structural().selected(&instruction.ty) => {}
                    Constant::Str(_) => {
                        return unsupported_operation(function.id, "source string constant")
                    }
                    Constant::StaticBytes(_) => {}
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
                InstructionKind::ProductField { value, .. }
                    if domain == LoweringDomain::StructuralIsland
                        && structural::selected_structural_source(function, *value, layouts)? => {}
                kind if explicit_structural(kind) && domain == LoweringDomain::StructuralIsland => {
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
                    return unsupported_operation(function.id, "structural operation")
                }
                InstructionKind::Drop {
                    glue: lkjscript_ir::DropGlueIdentity::Structural(_),
                    ..
                } if domain == LoweringDomain::StructuralIsland => {}
                InstructionKind::Drop {
                    glue: lkjscript_ir::DropGlueIdentity::Structural(_),
                    ..
                } => return unsupported_operation(function.id, "structural drop"),
                InstructionKind::PlaceInit { .. }
                | InstructionKind::PlaceEnd { .. }
                | InstructionKind::EndBorrow { .. }
                | InstructionKind::Drop { .. }
                | InstructionKind::Move { .. }
                    if matches!(
                        domain,
                        LoweringDomain::ResourceIsland | LoweringDomain::StructuralIsland
                    ) => {}
                InstructionKind::Borrow { .. } if domain == LoweringDomain::StructuralIsland => {}
                InstructionKind::PlaceInit { .. }
                | InstructionKind::PlaceEnd { .. }
                | InstructionKind::EndBorrow { .. }
                | InstructionKind::Drop {
                    glue:
                        lkjscript_ir::DropGlueIdentity::ByteVector
                        | lkjscript_ir::DropGlueIdentity::Bytes,
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
                InstructionKind::Runtime {
                    operation,
                    arguments,
                    ..
                } if supported_runtime(*operation, domain) => {
                    preflight_bytes_runtime(function, instruction, *operation, arguments, modes)?;
                }
                InstructionKind::F64FromI64Exact { .. }
                | InstructionKind::F64FromI64Rounded { .. }
                | InstructionKind::I64FromF64Exact { .. }
                | InstructionKind::I64FromF64Trunc { .. } => {}
                InstructionKind::Call {
                    target: CallTarget::Direct(callee),
                    ..
                } => {
                    let callee = source_function(program, *callee)?;
                    lower_signature(callee, modes, layouts)?;
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
                | InstructionKind::WithProductField { .. }
                | InstructionKind::EnumValue { .. }
                | InstructionKind::EnumIsVariant { .. }
                | InstructionKind::EnumField { .. }
                    if domain == LoweringDomain::StructuralIsland =>
                {
                    return unsupported_operation(
                        function.id,
                        "legacy aggregate operation in structural group",
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
