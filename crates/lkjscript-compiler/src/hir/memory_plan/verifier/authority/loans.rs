use super::*;

pub(super) fn verify_loans(plan: &HirMemoryPlan, facts: &Facts<'_>) -> Result<()> {
    let borrow_facts: Vec<_> = facts
        .expressions
        .iter()
        .filter(|fact| {
            matches!(
                fact.expression.kind,
                hir::ExprKind::Borrow { .. } | hir::ExprKind::BorrowBytes { .. }
            )
        })
        .collect();
    let loan_count = u64::try_from(borrow_facts.len())
        .map_err(|_| Error::msg("explicit HIR loan count exceeds u64"))?;
    if plan.loans.len() != borrow_facts.len() || plan.work.loans != loan_count {
        return Err(Error::msg("explicit HIR loan coverage/work mismatch"));
    }
    let mut loan_entries = BTreeMap::new();
    for entry in &plan.entries {
        if let MemorySubject::Loan {
            function,
            place,
            loan,
            expression,
        } = entry.subject
        {
            if loan_entries
                .insert((function, place, loan, expression), entry)
                .is_some()
            {
                return Err(Error::msg("memory verifier loan entry key is duplicated"));
            }
        }
    }
    let mut identities = BTreeSet::new();
    for (actual, fact) in plan.loans.iter().zip(borrow_facts) {
        let (place, loan, source, kind) = loan_source(fact.expression)?;
        if !identities.insert((fact.function, loan)) {
            return Err(Error::msg("memory loan identity is duplicated"));
        }
        let binding = inferred_reference_binding(facts, fact);
        let (semantic_uses, end_after) = inferred_loan_end(facts, fact, binding)?;
        let entry = loan_entries
            .get(&(fact.function, place, loan, fact.id))
            .copied()
            .ok_or_else(|| Error::msg("memory verifier loan entry is missing"))?;
        let expected = MemoryLoanPlan {
            function: fact.function,
            place,
            loan,
            expression: fact.id,
            binding,
            kind,
            semantic_uses,
            end_after,
            entry: entry.id,
        };
        if actual != &expected {
            return Err(Error::msg(
                "independent verifier rejected explicit loan scope",
            ));
        }
        if source == u32::MAX {
            return Err(Error::msg("memory loan source is invalid"));
        }
    }
    Ok(())
}

fn loan_source(expression: &hir::Expr) -> Result<(u32, u32, u32, MemoryBorrowKind)> {
    match &expression.kind {
        hir::ExprKind::Borrow {
            place,
            loan,
            binding,
            kind,
        } => Ok((
            place.raw(),
            loan.raw(),
            binding.binding.raw(),
            match kind {
                hir::BorrowKind::Shared => MemoryBorrowKind::Shared,
                hir::BorrowKind::Mutable => MemoryBorrowKind::Exclusive,
            },
        )),
        hir::ExprKind::BorrowBytes {
            place,
            loan,
            binding,
        } => Ok((
            place.raw(),
            loan.raw(),
            binding.binding.raw(),
            MemoryBorrowKind::Shared,
        )),
        _ => Err(Error::msg("memory loan references non-borrow")),
    }
}

fn inferred_reference_binding(facts: &Facts<'_>, fact: &ExprFact<'_>) -> Option<u32> {
    let parent = fact.parent?;
    let parent = facts.expression(parent)?;
    let hir::ExprKind::Let { bindings, .. } = &parent.expression.kind else {
        return None;
    };
    let local = bindings.get(usize::try_from(fact.child_index).ok()?)?;
    matches!(fact.expression.kind, hir::ExprKind::Borrow { .. }).then_some(local.binding.raw())
}

fn inferred_loan_end(
    facts: &Facts<'_>,
    fact: &ExprFact<'_>,
    binding: Option<u32>,
) -> Result<(u64, MemoryExpressionId)> {
    if let Some(binding) = binding {
        let loads = facts.binding_loads(fact.function, binding);
        let first = loads.partition_point(|index| {
            facts
                .expressions
                .get(*index)
                .is_some_and(|candidate| candidate.id <= fact.id)
        });
        let uses = &loads[first..];
        let last = uses
            .last()
            .and_then(|index| facts.expressions.get(*index))
            .ok_or_else(|| Error::msg("memory verifier loan has no reference use"))?;
        let parent = last
            .parent
            .ok_or_else(|| Error::msg("memory verifier loan use has no call"))?;
        Ok((
            u64::try_from(uses.len()).map_err(|_| Error::msg("loan uses exceed u64"))?,
            parent,
        ))
    } else {
        Ok((
            1,
            fact.parent
                .ok_or_else(|| Error::msg("temporary loan has no enclosing call"))?,
        ))
    }
}
