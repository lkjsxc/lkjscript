use super::*;

pub(super) fn verify_authority_calls(
    program: &hir::Program,
    plan: &HirMemoryPlan,
    facts: &Facts<'_>,
    types: &mut VerifiedTypes<'_>,
) -> Result<()> {
    let mut function_indices = HashMap::new();
    for (index, function) in program.functions.iter().enumerate() {
        if function_indices.insert(function.binding, index).is_some() {
            return Err(Error::msg("HIR direct-call function binding is duplicated"));
        }
    }
    let mut calls = BTreeMap::new();
    for call in &plan.calls {
        if calls.insert(call.expression, call).is_some() {
            return Err(Error::msg("HIR call plan expression is duplicated"));
        }
    }
    let mut places = BTreeMap::new();
    for entry in &plan.entries {
        if let MemorySubject::Place {
            function,
            place,
            binding,
        } = entry.subject
        {
            if places.insert((function, binding), place).is_some() {
                return Err(Error::msg("HIR call source place is duplicated"));
            }
        }
    }
    let mut expected_scopes = BTreeMap::new();
    for fact in &facts.expressions {
        let call_expression = matches!(
            fact.expression.kind,
            hir::ExprKind::Call { .. } | hir::ExprKind::Operation { .. }
        );
        let Some(call) = calls.get(&fact.id).copied() else {
            if call_expression {
                return Err(Error::msg("HIR call plan is missing"));
            }
            continue;
        };
        if !call_expression {
            return Err(Error::msg("stale HIR call plan"));
        }
        let (target, witness_arguments, parameters, result, direct, arguments) =
            verified_call_signature(program, plan, &function_indices, fact, types)?;
        if call.function != fact.function
            || call.target != target
            || call.witness_arguments != witness_arguments
            || call.parameters != parameters
            || call.result != result
            || call.borrow_scopes.len() != arguments.len()
        {
            return Err(Error::msg(
                "independent verifier rejected exact call memory signature",
            ));
        }
        for (index, (argument, mode)) in arguments.iter().zip(parameters).enumerate() {
            let child = child_fact(facts, fact.id, index)?;
            let expected = if direct {
                inferred_scope_source(types, argument, mode)?
            } else {
                None
            };
            match (expected, call.borrow_scopes[index]) {
                (None, None) => {}
                (Some((binding, kind)), Some(id)) if direct => {
                    let place = *places
                        .get(&(fact.function, binding))
                        .ok_or_else(|| Error::msg("inferred call borrow lost source place"))?;
                    let expected = MemoryBorrowScopePlan {
                        id,
                        function: fact.function,
                        call: call.id,
                        argument_index: index_u64(index)?,
                        source_expression: child.id,
                        binding,
                        place,
                        kind,
                        semantic_uses: 1,
                        end_after: fact.id,
                    };
                    if expected_scopes.insert(id, expected).is_some() {
                        return Err(Error::msg("call borrow scope identity is duplicated"));
                    }
                }
                _ => {
                    return Err(Error::msg(
                        "independent verifier rejected direct-call borrow scope",
                    ))
                }
            }
        }
    }
    verify_scope_records(plan, &expected_scopes)?;
    Ok(())
}

fn verify_scope_records(
    plan: &HirMemoryPlan,
    expected: &BTreeMap<MemoryBorrowScopeId, MemoryBorrowScopePlan>,
) -> Result<()> {
    let expected_count = u64::try_from(expected.len())
        .map_err(|_| Error::msg("borrow-scope record count exceeds u64"))?;
    if expected.len() != plan.borrow_scopes.len() || plan.work.borrow_scopes != expected_count {
        return Err(Error::msg(
            "direct-call borrow scope coverage/work mismatch",
        ));
    }
    for (index, scope) in plan.borrow_scopes.iter().enumerate() {
        if scope.id.raw() != index_u64(index)? || expected.get(&scope.id) != Some(scope) {
            return Err(Error::msg(
                "independent verifier rejected dense borrow-scope record",
            ));
        }
    }
    let mut scopes_by_expression = BTreeMap::new();
    for (id, scope) in expected {
        if scopes_by_expression
            .insert(scope.source_expression, *id)
            .is_some()
        {
            return Err(Error::msg("call borrow source expression is duplicated"));
        }
    }
    for entry in &plan.entries {
        let scope = match entry.subject {
            MemorySubject::Expression { expression, .. } => {
                scopes_by_expression.get(&expression).copied()
            }
            _ => None,
        };
        if entry.borrow_scope != scope {
            return Err(Error::msg(
                "memory entry direct-call borrow identity mismatch",
            ));
        }
        if let Some(id) = scope {
            let copy = if expected[&id].kind == MemoryBorrowKind::Shared {
                MemoryCopySharePlan::BorrowShared
            } else {
                MemoryCopySharePlan::BorrowExclusive
            };
            if entry.copy_share != copy {
                return Err(Error::msg("memory entry borrow copy/share plan mismatch"));
            }
        }
    }
    Ok(())
}
