use super::*;

pub(super) fn verified_allocation_failure(effects: u16) -> MemoryAllocationFailure {
    let allocates = effects & hir::EffectSet::ALLOCATES.bits() != 0;
    let trap = effects & hir::EffectSet::MAY_TRAP.bits() != 0;
    let outcome = effects & hir::EffectSet::MAY_EXIT.bits() != 0 || allocates;
    match (trap, outcome) {
        (false, false) => MemoryAllocationFailure::Impossible,
        (true, false) => MemoryAllocationFailure::Trap,
        (false, true) => MemoryAllocationFailure::StructuredOutcome,
        (true, true) => MemoryAllocationFailure::TrapOrOutcome,
    }
}

pub(super) fn is_static_bytes(entry: &MemoryPlanEntry, facts: &Facts<'_>) -> bool {
    if !matches!(entry.ty, MemoryType::Bytes) {
        return false;
    }
    let expression = match entry.subject {
        MemorySubject::Expression { expression, .. }
        | MemorySubject::Constant { expression, .. } => expression,
        _ => return false,
    };
    facts.expressions.iter().any(|item| {
        item.id == expression && matches!(item.expression.kind, hir::ExprKind::LitBytes(_))
    })
}

pub(super) fn expected_entry_escape(
    entry: &MemoryPlanEntry,
    facts: &Facts<'_>,
) -> Result<MemoryEscape> {
    match entry.subject {
        MemorySubject::Parameter { .. } => Ok(MemoryEscape::Caller),
        MemorySubject::Result { .. } => Ok(MemoryEscape::Returned),
        MemorySubject::Place { .. } | MemorySubject::Loan { .. } => Ok(MemoryEscape::Local),
        MemorySubject::Expression { expression, .. }
        | MemorySubject::Constant { expression, .. }
        | MemorySubject::Call { expression, .. } => expected_expression_escape(facts, expression),
    }
}

fn expected_expression_escape(facts: &Facts<'_>, id: MemoryExpressionId) -> Result<MemoryEscape> {
    let fact = facts
        .expressions
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| Error::msg("memory authority escape lost expression"))?;
    let Some(parent) = fact.parent else {
        return Ok(MemoryEscape::Returned);
    };
    let parent_fact = facts
        .expressions
        .iter()
        .find(|item| item.id == parent)
        .ok_or_else(|| Error::msg("memory authority escape lost parent"))?;
    use hir::ExprKind as K;
    Ok(match &parent_fact.expression.kind {
        K::Call { .. } | K::Operation { .. } => MemoryEscape::Caller,
        K::Return { .. } => MemoryEscape::Returned,
        K::Trap { .. } | K::Exit { .. } => MemoryEscape::Runtime,
        K::If { .. } if fact.child_index > 0 => expected_expression_escape(facts, parent)?,
        K::Do(items) if usize::try_from(fact.child_index).ok() == items.len().checked_sub(1) => {
            expected_expression_escape(facts, parent)?
        }
        K::Let { bindings, .. }
            if usize::try_from(fact.child_index).ok() == Some(bindings.len()) =>
        {
            expected_expression_escape(facts, parent)?
        }
        K::MutableLocal { .. } if fact.child_index == 1 => {
            expected_expression_escape(facts, parent)?
        }
        _ => MemoryEscape::Local,
    })
}
