fn expression_kind(kind: &ExprKind) -> MemoryExpressionKind {
    match kind {
        ExprKind::LitI64(value) => MemoryExpressionKind::I64Literal(*value),
        ExprKind::LitF64(value) => MemoryExpressionKind::F64Literal(value.to_bits()),
        ExprKind::LitBool(value) => MemoryExpressionKind::BoolLiteral(*value),
        ExprKind::LitUnit => MemoryExpressionKind::UnitLiteral,
        ExprKind::EmptyList => MemoryExpressionKind::EmptyList,
        ExprKind::LitStr(_) => MemoryExpressionKind::StringLiteral,
        ExprKind::LitBytes(_) => MemoryExpressionKind::BytesLiteral,
        ExprKind::Load(binding) => MemoryExpressionKind::Load {
            binding: binding.binding.raw(),
            storage: binding_storage(binding.storage),
        },
        ExprKind::Move { place, binding } => MemoryExpressionKind::Move {
            place: place.raw(),
            binding: binding.binding.raw(),
        },
        ExprKind::BorrowBytes {
            place,
            loan,
            binding,
        } => MemoryExpressionKind::Borrow {
            place: place.raw(),
            loan: loan.raw(),
            kind: MemoryBorrowKind::Shared,
            binding: binding.binding.raw(),
        },
        ExprKind::Borrow {
            place,
            loan,
            kind,
            binding,
        } => MemoryExpressionKind::Borrow {
            place: place.raw(),
            loan: loan.raw(),
            kind: borrow_kind(*kind),
            binding: binding.binding.raw(),
        },
        ExprKind::Call { callee, .. } => match callee.storage {
            BindingStorage::Function => MemoryExpressionKind::DirectCall,
            BindingStorage::Local(_) => MemoryExpressionKind::IndirectCall,
        },
        ExprKind::Operation { operation, .. } => {
            MemoryExpressionKind::Operation(operation.identity().as_u16())
        }
        ExprKind::F64FromI64Exact(_) => MemoryExpressionKind::F64FromI64Exact,
        ExprKind::F64FromI64Rounded(_) => MemoryExpressionKind::F64FromI64Rounded,
        ExprKind::I64FromF64Exact(_) => MemoryExpressionKind::I64FromF64Exact,
        ExprKind::I64FromF64Trunc(_) => MemoryExpressionKind::I64FromF64Trunc,
        ExprKind::Do(_) => MemoryExpressionKind::Sequence,
        ExprKind::If { .. } => MemoryExpressionKind::If,
        ExprKind::While { .. } => MemoryExpressionKind::While,
        ExprKind::Loop { .. } => MemoryExpressionKind::Loop,
        ExprKind::Return { .. } => MemoryExpressionKind::Return,
        ExprKind::Break { .. } => MemoryExpressionKind::Break,
        ExprKind::Continue { .. } => MemoryExpressionKind::Continue,
        ExprKind::Trap { .. } => MemoryExpressionKind::Trap,
        ExprKind::Exit { .. } => MemoryExpressionKind::Exit,
        ExprKind::Let { .. } => MemoryExpressionKind::Let,
        ExprKind::MutableLocal { .. } => MemoryExpressionKind::MutableLocal,
        ExprKind::SetLocal { .. } => MemoryExpressionKind::SetLocal,
        ExprKind::ProductValue { .. } => MemoryExpressionKind::ProductValue,
        ExprKind::ProductField { .. } => MemoryExpressionKind::ProductField,
        ExprKind::WithProductField { .. } => MemoryExpressionKind::WithProductField,
        ExprKind::EnumValue { .. } => MemoryExpressionKind::EnumValue,
        ExprKind::EnumIsVariant { .. } => MemoryExpressionKind::EnumIsVariant,
        ExprKind::EnumField { .. } => MemoryExpressionKind::EnumField,
        ExprKind::EnumUnwrap { .. } => MemoryExpressionKind::EnumUnwrap,
        ExprKind::MatchUnreachable { .. } => MemoryExpressionKind::MatchUnreachable,
        ExprKind::QuoteSymbol(_) => MemoryExpressionKind::SymbolLiteral,
    }
}
fn binding_storage(storage: BindingStorage) -> MemoryBindingStorage {
    match storage {
        BindingStorage::Local(_) => MemoryBindingStorage::Local,
        BindingStorage::Function => MemoryBindingStorage::Function,
    }
}
fn memory_type(ty: &Type) -> MemoryType {
    match ty {
        Type::Never => MemoryType::Never,
        Type::Unit => MemoryType::Unit,
        Type::Bool => MemoryType::Bool,
        Type::I64 => MemoryType::I64,
        Type::F64 => MemoryType::F64,
        Type::Str => MemoryType::String,
        Type::Buf => MemoryType::Buffer,
        Type::Bytes => MemoryType::Bytes,
        Type::Path => MemoryType::Path,
        Type::Capability(kind) => MemoryType::Capability(*kind),
        Type::ByteVector => MemoryType::ByteVector,
        Type::ByteSlice => MemoryType::ByteSlice,
        Type::ByteSliceMut => MemoryType::ByteSliceMut,
        Type::Symbol => MemoryType::Symbol,
        Type::Resource(kind) => MemoryType::Resource(*kind),
        Type::Product(name) => MemoryType::Product(name.clone()),
        Type::Enum {
            id,
            name,
            arguments,
        } => MemoryType::Enum {
            id: id.bytes(),
            name: name.clone(),
            arguments: arguments.iter().map(memory_type).collect(),
        },
        Type::Param(name) => MemoryType::TypeParameter(name.clone()),
        Type::List(inner) => MemoryType::List(Box::new(memory_type(inner))),
        Type::Fn { params, ret } => MemoryType::Function {
            parameters: params.iter().map(memory_type).collect(),
            result: Box::new(memory_type(ret)),
        },
        Type::Forall { vars, body } => MemoryType::ForAll {
            variables: vars.clone(),
            body: Box::new(memory_type(body)),
        },
    }
}
