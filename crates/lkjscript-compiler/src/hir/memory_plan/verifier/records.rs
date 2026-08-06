use super::*;

pub(super) fn verify_uses_and_constants(plan: &HirMemoryPlan, facts: &Facts<'_>) -> Result<()> {
    let mut uses = Vec::new();
    let mut constants = Vec::new();
    for fact in &facts.expressions {
        if let Some(value) = verified_constant(&fact.expression.kind) {
            constants.push((fact.function, fact.id, value));
        }
        if let Some((binding, kind)) = verified_use(&fact.expression.kind) {
            uses.push((fact.function, fact.id, binding, kind));
        }
    }
    if uses.len() != plan.uses.len() || constants.len() != plan.constants.len() {
        return Err(Error::msg("HIR memory use/constant coverage mismatch"));
    }
    for (index, (actual, expected)) in plan.uses.iter().zip(uses).enumerate() {
        if actual.id.raw() != index_u64(index)?
            || actual.function != expected.0
            || actual.expression != expected.1
            || actual.binding != expected.2
            || actual.kind != expected.3
        {
            return Err(Error::msg("independent verifier rejected HIR memory use"));
        }
    }
    for (index, (actual, expected)) in plan.constants.iter().zip(constants).enumerate() {
        if actual.id.raw() != index_u64(index)?
            || actual.function != expected.0
            || actual.expression != expected.1
            || actual.value != expected.2
        {
            return Err(Error::msg(
                "independent verifier rejected HIR memory constant",
            ));
        }
    }
    Ok(())
}

fn verified_use(kind: &hir::ExprKind) -> Option<(u64, MemoryUseKind)> {
    match kind {
        hir::ExprKind::Load(reference) => Some((reference.binding.raw(), MemoryUseKind::Load)),
        hir::ExprKind::Move { binding, .. } => Some((binding.binding.raw(), MemoryUseKind::Move)),
        hir::ExprKind::Borrow { binding, .. } | hir::ExprKind::BorrowBytes { binding, .. } => {
            Some((binding.binding.raw(), MemoryUseKind::BorrowSource))
        }
        hir::ExprKind::Call { callee, .. } => Some((
            callee.binding.raw(),
            match callee.storage {
                hir::BindingStorage::Function => MemoryUseKind::DirectCallTarget,
                hir::BindingStorage::Local(_) => MemoryUseKind::IndirectCallTarget,
            },
        )),
        _ => None,
    }
}

fn verified_constant(kind: &hir::ExprKind) -> Option<MemoryConstantValue> {
    match kind {
        hir::ExprKind::LitI64(value) => Some(MemoryConstantValue::I64(*value)),
        hir::ExprKind::LitF64(value) => Some(MemoryConstantValue::F64(value.to_bits())),
        hir::ExprKind::LitBool(value) => Some(MemoryConstantValue::Bool(*value)),
        hir::ExprKind::LitUnit => Some(MemoryConstantValue::Unit),
        hir::ExprKind::EmptyList => Some(MemoryConstantValue::EmptyList),
        hir::ExprKind::LitStr(value) => Some(MemoryConstantValue::String(value.clone())),
        hir::ExprKind::LitBytes(value) => Some(MemoryConstantValue::Bytes(value.clone())),
        hir::ExprKind::QuoteSymbol(value) => Some(MemoryConstantValue::Symbol(value.clone())),
        _ => None,
    }
}
