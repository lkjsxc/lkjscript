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
