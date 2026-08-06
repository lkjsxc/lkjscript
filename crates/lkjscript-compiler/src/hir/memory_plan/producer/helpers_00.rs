fn charge(slot: &mut u64, amount: usize, limit: u64, label: &str) -> Result<()> {
    let amount = u64::try_from(amount)
        .map_err(|_| Error::msg(format!("HIR memory-plan {label} charge exceeds u64")))?;
    *slot = slot
        .checked_add(amount)
        .ok_or_else(|| Error::msg(format!("HIR memory-plan {label} work overflow")))?;
    if *slot > limit {
        return Err(Error::msg(format!(
            "HIR memory-plan {label} work exceeds {limit}"
        )));
    }
    Ok(())
}
fn observe(slot: &mut u64, amount: usize, label: &str) -> Result<()> {
    let amount = u64::try_from(amount)
        .map_err(|_| Error::msg(format!("HIR memory-plan {label} observation exceeds u64")))?;
    *slot = slot
        .checked_add(amount)
        .ok_or_else(|| Error::msg(format!("HIR memory-plan {label} observation overflow")))?;
    Ok(())
}
fn index_u32(index: usize) -> Result<u32> {
    u32::try_from(index).map_err(|_| Error::msg("HIR memory-plan child index exceeds u32"))
}
fn function_result_type(ty: &Type) -> Result<&Type> {
    let (_, result) = callable_type(ty)?;
    Ok(result)
}
fn callable_type(ty: &Type) -> Result<(&[Type], &Type)> {
    match ty {
        Type::Fn { params, ret } => Ok((params, ret)),
        Type::Forall { body, .. } => callable_type(body),
        _ => Err(Error::msg(
            "HIR memory-plan callable binding has non-function type",
        )),
    }
}
fn resource_parameter_consumed(expression: &Expr, binding: BindingId) -> bool {
    match &expression.kind {
        ExprKind::Move { binding: moved, .. } if moved.binding == binding => true,
        ExprKind::Operation {
            operation, args, ..
        } if consuming_operation(*operation)
            && args
                .iter()
                .any(|argument| expression_uses_binding(argument, binding)) =>
        {
            true
        }
        _ => children(expression)
            .into_iter()
            .any(|child| resource_parameter_consumed(child, binding)),
    }
}
fn expression_uses_binding(expression: &Expr, binding: BindingId) -> bool {
    match expression.kind {
        ExprKind::Load(reference)
        | ExprKind::Move {
            binding: reference, ..
        }
        | ExprKind::Borrow {
            binding: reference, ..
        } => reference.binding == binding,
        _ => children(expression)
            .into_iter()
            .any(|child| expression_uses_binding(child, binding)),
    }
}
fn children(expression: &Expr) -> Vec<&Expr> {
    match &expression.kind {
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::Loop { body: args, .. }
        | ExprKind::ProductValue { fields: args, .. }
        | ExprKind::EnumValue { fields: args, .. } => args.iter().collect(),
        ExprKind::While {
            condition, body, ..
        } => std::iter::once(condition.as_ref())
            .chain(body.iter())
            .collect(),
        ExprKind::F64FromI64Exact(value)
        | ExprKind::F64FromI64Rounded(value)
        | ExprKind::I64FromF64Exact(value)
        | ExprKind::I64FromF64Trunc(value)
        | ExprKind::Return { value }
        | ExprKind::Break { value, .. }
        | ExprKind::Trap { value }
        | ExprKind::Exit { code: value }
        | ExprKind::SetLocal { value, .. }
        | ExprKind::ProductField { value, .. }
        | ExprKind::EnumIsVariant { value, .. }
        | ExprKind::EnumField { value, .. }
        | ExprKind::EnumUnwrap { value, .. } => vec![value],
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => vec![condition, then_branch, else_branch],
        ExprKind::Let { bindings, body } => bindings
            .iter()
            .map(|binding| &binding.value)
            .chain(std::iter::once(body.as_ref()))
            .collect(),
        ExprKind::MutableLocal { initial, body, .. } => vec![initial, body],
        ExprKind::WithProductField {
            value, replacement, ..
        } => vec![value, replacement],
        _ => Vec::new(),
    }
}
fn consuming_operation(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::DropResource | Operation::SysSqliteClose | Operation::SysSqliteFinalize
    )
}
fn parameter_mode(ty: &Type, resource_consumed: bool) -> MemoryParameterMode {
    match ty {
        Type::Bytes | Type::ByteVector => MemoryParameterMode::Consume,
        Type::ByteSlice => MemoryParameterMode::BorrowShared,
        Type::ByteSliceMut => MemoryParameterMode::BorrowExclusive,
        Type::Str | Type::Path => MemoryParameterMode::BorrowShared,
        Type::Resource(_) if resource_consumed => MemoryParameterMode::Consume,
        Type::Resource(_) => MemoryParameterMode::BorrowExclusive,
        _ => MemoryParameterMode::Copy,
    }
}
fn operation_parameter_mode(operation: Operation, ty: &Type) -> MemoryParameterMode {
    match ty {
        Type::Resource(_) if consuming_operation(operation) => MemoryParameterMode::Consume,
        Type::Resource(_) => MemoryParameterMode::BorrowExclusive,
        _ => parameter_mode(ty, false),
    }
}
fn result_mode(ty: &Type) -> MemoryResultMode {
    match ty {
        Type::Bytes | Type::ByteVector => MemoryResultMode::Owned,
        Type::ByteSlice | Type::ByteSliceMut => MemoryResultMode::Trivial,
        Type::Str | Type::Path | Type::Product(_) | Type::Enum { .. } => MemoryResultMode::Owned,
        Type::Resource(_) => MemoryResultMode::External,
        _ => MemoryResultMode::Trivial,
    }
}
fn borrow_kind(kind: BorrowKind) -> MemoryBorrowKind {
    match kind {
        BorrowKind::Shared => MemoryBorrowKind::Shared,
        BorrowKind::Mutable => MemoryBorrowKind::Exclusive,
    }
}
fn constant_value(kind: &ExprKind) -> Option<MemoryConstantValue> {
    match kind {
        ExprKind::LitI64(value) => Some(MemoryConstantValue::I64(*value)),
        ExprKind::LitF64(value) => Some(MemoryConstantValue::F64(value.to_bits())),
        ExprKind::LitBool(value) => Some(MemoryConstantValue::Bool(*value)),
        ExprKind::LitUnit => Some(MemoryConstantValue::Unit),
        ExprKind::EmptyList => Some(MemoryConstantValue::EmptyList),
        ExprKind::LitStr(value) => Some(MemoryConstantValue::String(value.clone())),
        ExprKind::LitBytes(value) => Some(MemoryConstantValue::Bytes(value.clone())),
        ExprKind::QuoteSymbol(value) => Some(MemoryConstantValue::Symbol(value.clone())),
        _ => None,
    }
}
