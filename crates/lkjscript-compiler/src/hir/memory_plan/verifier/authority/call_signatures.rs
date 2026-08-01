use super::*;

type VerifiedCall<'a> = (
    MemoryCallTarget,
    Vec<MemoryWitnessArgument>,
    Vec<MemoryParameterMode>,
    MemoryResultMode,
    bool,
    &'a [hir::Expr],
);

pub(super) fn verified_call_signature<'a>(
    program: &'a hir::Program,
    plan: &HirMemoryPlan,
    fact: &ExprFact<'a>,
    types: &mut VerifiedTypes<'_>,
) -> Result<VerifiedCall<'a>> {
    match &fact.expression.kind {
        hir::ExprKind::Call {
            callee,
            args,
            instantiation,
        } => match callee.storage {
            hir::BindingStorage::Function => {
                let target = program
                    .functions
                    .iter()
                    .position(|item| item.binding == callee.binding)
                    .ok_or_else(|| Error::msg("memory verifier lost direct call target"))?;
                let signature = &plan
                    .function(MemoryFunctionId::new(index_u32(target)?))
                    .ok_or_else(|| Error::msg("memory verifier lost direct signature"))?
                    .signature;
                let binding = program
                    .binding(callee.binding)
                    .ok_or_else(|| Error::msg("memory verifier lost direct callable"))?;
                let witness_parameters = verified_witness_parameters(&binding.ty)?;
                let witness_arguments = verified_witness_arguments(
                    types,
                    &binding.ty,
                    &witness_parameters,
                    instantiation.as_ref(),
                )?;
                if instantiation.is_some() {
                    let parameters = args
                        .iter()
                        .map(|arg| verified_parameter_mode(types, &arg.ty, false))
                        .collect::<Result<Vec<_>>>()?;
                    Ok((
                        MemoryCallTarget::Direct(signature.function),
                        witness_arguments,
                        parameters,
                        verified_result_mode(types, &fact.expression.ty)?,
                        true,
                        args,
                    ))
                } else {
                    Ok((
                        MemoryCallTarget::Direct(signature.function),
                        witness_arguments,
                        signature.parameters.clone(),
                        signature.result,
                        true,
                        args,
                    ))
                }
            }
            hir::BindingStorage::Local(_) => {
                let binding = program
                    .binding(callee.binding)
                    .ok_or_else(|| Error::msg("memory verifier lost indirect callable"))?;
                if instantiation.is_some() || matches!(binding.ty, Type::Forall { .. }) {
                    return Err(Error::msg("memory verifier rejected indirect generic call"));
                }
                let (parameters, result) = verified_callable_type(&binding.ty)?;
                Ok((
                    MemoryCallTarget::Indirect(callee.binding.raw()),
                    Vec::new(),
                    parameters.iter().map(verified_indirect_parameter).collect(),
                    verified_call_result(result),
                    false,
                    args,
                ))
            }
        },
        hir::ExprKind::Operation {
            operation,
            resolved_signature,
            args,
            ..
        } => {
            let (parameters, result) = verified_callable_type(resolved_signature)?;
            Ok((
                MemoryCallTarget::Operation(operation.identity().as_u16()),
                Vec::new(),
                parameters
                    .iter()
                    .map(|ty| verified_operation_parameter(*operation, ty))
                    .collect(),
                verified_call_result(result),
                false,
                args,
            ))
        }
        _ => Err(Error::msg(
            "memory call record references non-call expression",
        )),
    }
}

pub(super) fn inferred_scope_source(
    types: &mut VerifiedTypes<'_>,
    argument: &hir::Expr,
    mode: MemoryParameterMode,
) -> Result<Option<(u32, MemoryBorrowKind)>> {
    let hir::ExprKind::Load(reference) = argument.kind else {
        return Ok(None);
    };
    let kind = match mode {
        MemoryParameterMode::BorrowShared => MemoryBorrowKind::Shared,
        MemoryParameterMode::BorrowExclusive => MemoryBorrowKind::Exclusive,
        _ => return Ok(None),
    };
    let id = types.intern(&argument.ty)?;
    let fact = types.expected(id)?;
    if kind == MemoryBorrowKind::Shared
        && (fact.derived.closure.class != MemoryClosureClass::Deterministic
            || fact.derived.mode != MemoryAggregateMode::ImmutableValue)
    {
        return Ok(None);
    }
    Ok(Some((reference.binding.raw(), kind)))
}

pub(super) fn child_fact<'a>(
    facts: &'a Facts<'a>,
    parent: MemoryExpressionId,
    index: usize,
) -> Result<&'a ExprFact<'a>> {
    let index = index_u32(index)?;
    facts
        .expressions
        .iter()
        .find(|item| item.parent == Some(parent) && item.child_index == index)
        .ok_or_else(|| Error::msg("memory call lost argument expression"))
}

fn verified_callable_type(ty: &Type) -> Result<(&[Type], &Type)> {
    match ty {
        Type::Fn { params, ret } => Ok((params, ret)),
        Type::Forall { body, .. } => verified_callable_type(body),
        _ => Err(Error::msg("memory verifier expected callable type")),
    }
}

fn verified_indirect_parameter(ty: &Type) -> MemoryParameterMode {
    match ty {
        Type::Bytes | Type::ByteVector => MemoryParameterMode::Consume,
        Type::ByteSlice => MemoryParameterMode::BorrowShared,
        Type::ByteSliceMut => MemoryParameterMode::BorrowExclusive,
        Type::Str | Type::Path => MemoryParameterMode::BorrowShared,
        Type::Resource(_) => MemoryParameterMode::BorrowExclusive,
        _ => MemoryParameterMode::Copy,
    }
}

fn verified_operation_parameter(operation: hir::Operation, ty: &Type) -> MemoryParameterMode {
    if matches!(ty, Type::Resource(_))
        && matches!(
            operation,
            hir::Operation::DropResource
                | hir::Operation::SysSqliteClose
                | hir::Operation::SysSqliteFinalize
        )
    {
        MemoryParameterMode::Consume
    } else {
        verified_indirect_parameter(ty)
    }
}

fn verified_call_result(ty: &Type) -> MemoryResultMode {
    match ty {
        Type::Bytes | Type::ByteVector => MemoryResultMode::Owned,
        Type::ByteSlice | Type::ByteSliceMut => MemoryResultMode::Trivial,
        Type::Str | Type::Path | Type::Product(_) | Type::Enum { .. } => MemoryResultMode::Owned,
        Type::Resource(_) => MemoryResultMode::External,
        _ => MemoryResultMode::Trivial,
    }
}
