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
    facts
        .expression(expression)
        .is_some_and(|item| matches!(item.expression.kind, hir::ExprKind::LitBytes(_)))
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

fn expected_expression_escape(
    facts: &Facts<'_>,
    mut id: MemoryExpressionId,
) -> Result<MemoryEscape> {
    loop {
        let fact = expression_fact(facts, id)
            .ok_or_else(|| Error::msg("memory authority escape lost expression"))?;
        let Some(parent) = fact.parent else {
            return Ok(MemoryEscape::Returned);
        };
        let parent_fact = expression_fact(facts, parent)
            .ok_or_else(|| Error::msg("memory authority escape lost parent"))?;
        use hir::ExprKind as K;
        match &parent_fact.expression.kind {
            K::Call { .. } | K::Operation { .. } => return Ok(MemoryEscape::Caller),
            K::Return { .. } => return Ok(MemoryEscape::Returned),
            K::Trap { .. } | K::Exit { .. } => return Ok(MemoryEscape::Runtime),
            K::If { .. } if fact.child_index > 0 => id = parent,
            K::Do(items)
                if usize::try_from(fact.child_index).ok() == items.len().checked_sub(1) =>
            {
                id = parent;
            }
            K::Let { bindings, .. }
                if usize::try_from(fact.child_index).ok() == Some(bindings.len()) =>
            {
                id = parent;
            }
            K::MutableLocal { .. } if fact.child_index == 1 => id = parent,
            _ => return Ok(MemoryEscape::Local),
        }
    }
}

fn expression_fact<'a>(facts: &'a Facts<'a>, id: MemoryExpressionId) -> Option<&'a ExprFact<'a>> {
    facts
        .expressions
        .get(id.index()?)
        .filter(|fact| fact.id == id)
}
