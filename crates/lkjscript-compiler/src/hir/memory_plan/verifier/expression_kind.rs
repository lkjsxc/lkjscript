use super::*;

pub(super) fn verified_expression_kind(kind: &hir::ExprKind) -> MemoryExpressionKind {
    use hir::ExprKind as K;
    match kind {
        K::Hole => unreachable!("complete HIR cannot contain a hole"),
        K::UnresolvedValueReference { .. } => {
            unreachable!("complete HIR cannot contain an unresolved value reference")
        }
        K::Match { .. } => {
            unreachable!("semantic matches must be lowered before memory verification")
        }
        K::LitI64(value) => MemoryExpressionKind::I64Literal(*value),
        K::LitF64(value) => MemoryExpressionKind::F64Literal(value.to_bits()),
        K::LitBool(value) => MemoryExpressionKind::BoolLiteral(*value),
        K::LitUnit => MemoryExpressionKind::UnitLiteral,
        K::EmptyList => MemoryExpressionKind::EmptyList,
        K::LitStr(_) => MemoryExpressionKind::StringLiteral,
        K::LitBytes(_) => MemoryExpressionKind::BytesLiteral,
        K::Load(binding) => MemoryExpressionKind::Load {
            binding: binding.binding.raw(),
            storage: verified_binding_storage(binding.storage),
        },
        K::Move { place, binding } => MemoryExpressionKind::Move {
            place: place.raw(),
            binding: binding.binding.raw(),
        },
        K::BorrowBytes {
            place,
            loan,
            binding,
        } => MemoryExpressionKind::Borrow {
            place: place.raw(),
            loan: loan.raw(),
            kind: MemoryBorrowKind::Shared,
            binding: binding.binding.raw(),
        },
        K::Borrow {
            place,
            loan,
            kind,
            binding,
        } => MemoryExpressionKind::Borrow {
            place: place.raw(),
            loan: loan.raw(),
            kind: match kind {
                hir::BorrowKind::Shared => MemoryBorrowKind::Shared,
                hir::BorrowKind::Mutable => MemoryBorrowKind::Exclusive,
            },
            binding: binding.binding.raw(),
        },
        K::Call { callee, .. } => match callee.storage {
            hir::BindingStorage::Function => MemoryExpressionKind::DirectCall,
            hir::BindingStorage::Local(_) => MemoryExpressionKind::IndirectCall,
        },
        K::Operation { operation, .. } => {
            MemoryExpressionKind::Operation(operation.identity().as_u16())
        }
        K::F64FromI64Exact(_) => MemoryExpressionKind::F64FromI64Exact,
        K::F64FromI64Rounded(_) => MemoryExpressionKind::F64FromI64Rounded,
        K::I64FromF64Exact(_) => MemoryExpressionKind::I64FromF64Exact,
        K::I64FromF64Trunc(_) => MemoryExpressionKind::I64FromF64Trunc,
        K::Do(_) => MemoryExpressionKind::Sequence,
        K::If { .. } => MemoryExpressionKind::If,
        K::While { .. } => MemoryExpressionKind::While,
        K::Loop { .. } => MemoryExpressionKind::Loop,
        K::Return { .. } => MemoryExpressionKind::Return,
        K::Break { .. } => MemoryExpressionKind::Break,
        K::Continue { .. } => MemoryExpressionKind::Continue,
        K::Trap { .. } => MemoryExpressionKind::Trap,
        K::Exit { .. } => MemoryExpressionKind::Exit,
        K::Let { .. } => MemoryExpressionKind::Let,
        K::MutableLocal { .. } => MemoryExpressionKind::MutableLocal,
        K::SetLocal { .. } => MemoryExpressionKind::SetLocal,
        K::ProductValue { .. } => MemoryExpressionKind::ProductValue,
        K::ProductField { .. } => MemoryExpressionKind::ProductField,
        K::WithProductField { .. } => MemoryExpressionKind::WithProductField,
        K::EnumValue { .. } => MemoryExpressionKind::EnumValue,
        K::EnumIsVariant { .. } => MemoryExpressionKind::EnumIsVariant,
        K::EnumField { .. } => MemoryExpressionKind::EnumField,
        K::EnumUnwrap { .. } => MemoryExpressionKind::EnumUnwrap,
        K::MatchUnreachable { .. } => MemoryExpressionKind::MatchUnreachable,
        K::QuoteSymbol(_) => MemoryExpressionKind::SymbolLiteral,
    }
}

fn verified_binding_storage(storage: hir::BindingStorage) -> MemoryBindingStorage {
    match storage {
        hir::BindingStorage::Local(_) => MemoryBindingStorage::Local,
        hir::BindingStorage::Function => MemoryBindingStorage::Function,
    }
}
