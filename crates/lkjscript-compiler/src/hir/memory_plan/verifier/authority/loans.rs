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
    if plan.loans.len() != borrow_facts.len()
        || plan.work.loans != u64::try_from(borrow_facts.len()).unwrap_or(u64::MAX)
    {
        return Err(Error::msg("explicit HIR loan coverage/work mismatch"));
    }
    let mut identities = BTreeSet::new();
    for (actual, fact) in plan.loans.iter().zip(borrow_facts) {
        let (place, loan, source, kind) = loan_source(fact.expression)?;
        if !identities.insert((fact.function, loan)) {
            return Err(Error::msg("memory loan identity is duplicated"));
        }
        let binding = inferred_reference_binding(facts, fact);
        let (semantic_uses, end_after) = inferred_loan_end(facts, fact, binding)?;
        let entry = plan
            .entries
            .iter()
            .find(|entry| {
                matches!(entry.subject,
            MemorySubject::Loan { function, place: item_place, loan: item_loan, expression }
                if function == fact.function && item_place == place && item_loan == loan
                    && expression == fact.id)
            })
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
    let parent = facts.expressions.iter().find(|item| item.id == parent)?;
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
) -> Result<(u32, MemoryExpressionId)> {
    if let Some(binding) = binding {
        let uses: Vec<_> = facts
            .expressions
            .iter()
            .filter(|item| {
                item.function == fact.function
                    && item.id > fact.id
                    && matches!(item.expression.kind,
                hir::ExprKind::Load(reference) if reference.binding.raw() == binding)
            })
            .collect();
        let last = uses
            .last()
            .ok_or_else(|| Error::msg("memory verifier loan has no reference use"))?;
        let parent = last
            .parent
            .ok_or_else(|| Error::msg("memory verifier loan use has no call"))?;
        Ok((
            u32::try_from(uses.len()).map_err(|_| Error::msg("loan uses exceed u32"))?,
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
