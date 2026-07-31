use super::*;

pub(super) fn verify_work(plan: &HirMemoryPlan, facts: &Facts<'_>) -> Result<()> {
    let expected_entries = u64::try_from(facts.expressions.len())
        .ok()
        .and_then(|value| value.checked_add(facts.parameters))
        .and_then(|value| value.checked_add(facts.places))
        .and_then(|value| value.checked_add(plan.work.functions))
        .and_then(|value| value.checked_add(facts.loans))
        .and_then(|value| value.checked_add(facts.constants))
        .and_then(|value| value.checked_add(facts.calls))
        .ok_or_else(|| Error::msg("HIR memory verifier entry count overflow"))?;
    if plan.work.functions != u64::try_from(facts.bodies.len()).unwrap_or(u64::MAX)
        || plan.work.expressions != u64::try_from(facts.expressions.len()).unwrap_or(u64::MAX)
        || plan.work.entries != expected_entries
        || plan.work.uses != facts.uses
        || plan.work.loans != facts.loans
        || plan.work.constants != facts.constants
        || plan.work.calls != facts.calls
    {
        return Err(Error::msg(format!(
            concat!(
                "HIR memory-plan work mismatch: entries expected {}, got {}; functions expected {}, got {}; ",
                "expressions expected {}, got {}; uses expected {}, got {}; loans expected {}, got {}; ",
                "constants expected {}, got {}; calls expected {}, got {}; obligations expected {}, got {}"
            ),
            expected_entries,
            plan.work.entries,
            facts.bodies.len(),
            plan.work.functions,
            facts.expressions.len(),
            plan.work.expressions,
            facts.uses,
            plan.work.uses,
            facts.loans,
            plan.work.loans,
            facts.constants,
            plan.work.constants,
            facts.calls,
            plan.work.calls,
            plan.obligations.len(),
            plan.work.obligations,
        )));
    }
    Ok(())
}

pub(super) fn verify_expressions(plan: &HirMemoryPlan, facts: &Facts<'_>) -> Result<()> {
    let mut found = BTreeMap::new();
    for entry in &plan.entries {
        if let MemorySubject::Expression {
            expression,
            parent,
            child_index,
            ref kind,
        } = entry.subject
        {
            if found
                .insert(expression, (entry, parent, child_index, kind))
                .is_some()
            {
                return Err(Error::msg(
                    "HIR memory plan duplicates an expression result",
                ));
            }
        }
    }
    for fact in &facts.expressions {
        let (entry, parent, child_index, kind) = found
            .get(&fact.id)
            .copied()
            .ok_or_else(|| Error::msg("HIR memory plan omits an expression result"))?;
        if parent != fact.parent
            || child_index != fact.child_index
            || entry.effects != fact.expression.effects.bits()
            || entry.origin.source != fact.expression.origin.raw()
            || entry.origin.expression != Some(fact.id)
            || *kind != verified_expression_kind(&fact.expression.kind)
            || !type_matches(&fact.expression.ty, &entry.ty)
        {
            return Err(Error::msg("HIR memory expression fact mismatch"));
        }
    }
    if found.len() != facts.expressions.len() {
        return Err(Error::msg("HIR memory plan has a stale expression result"));
    }
    for (index, body) in facts.bodies.iter().enumerate() {
        let function = plan
            .function(MemoryFunctionId::new(index_u32(index)?))
            .ok_or_else(|| Error::msg("HIR memory function body is missing"))?;
        if function.body != *body {
            return Err(Error::msg("HIR memory function body identity mismatch"));
        }
    }
    Ok(())
}
