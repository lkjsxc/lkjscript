fn memory_witness_parameters(
    ty: &Type,
    function_body: Option<&Expr>,
) -> Result<Vec<MemoryWitnessParameter>> {
    let Type::Forall { vars, body } = ty else {
        return Ok(Vec::new());
    };
    let Type::Fn { params, ret } = body.as_ref() else {
        return Err(Error::msg(
            "HIR memory witness forall body is not a function",
        ));
    };
    for (index, variable) in vars.iter().enumerate() {
        if vars[..index].contains(variable) {
            return Err(Error::msg(
                "HIR memory witness declaration has duplicate type parameter",
            ));
        }
    }
    for ty in params.iter().chain(std::iter::once(ret.as_ref())) {
        match ty {
            Type::Param(name) if !vars.contains(name) => {
                return Err(Error::msg(
                    "HIR memory witness signature has an unresolved type parameter",
                ));
            }
            Type::Param(_) => {}
            _ if type_contains_any_parameter(ty)? => {
                return Err(Error::msg(
                    "HIR memory witness parameter has a nested operational use",
                ));
            }
            _ => {}
        }
    }
    let mut output = Vec::new();
    for variable in vars {
        let naked = params
            .iter()
            .any(|ty| matches!(ty, Type::Param(name) if name == variable))
            || matches!(ret.as_ref(), Type::Param(name) if name == variable);
        if naked {
            let occurrences = params.iter()
                .filter(|ty| matches!(ty, Type::Param(name) if name == variable))
                .count();
            let mut operations = vec![MemoryWitnessOperation::Transport];
            let compare =
                function_body.is_some_and(|body| body_demands_compare(body, variable));
            if compare {
                operations.push(MemoryWitnessOperation::Compare);
            }
            if occurrences >= 2
                && !(compare
                    && function_body.is_some_and(|body| body_is_compare_only(body, variable)))
            {
                operations.extend([
                    MemoryWitnessOperation::IndependentOwner,
                    MemoryWitnessOperation::Dispose,
                ]);
            }
            output.push(MemoryWitnessParameter {
                parameter: variable.clone(),
                operations,
            });
        }
    }
    Ok(output)
}

fn memory_witness_arguments(
    planner: &mut TypePlanner<'_>,
    callee_ty: &Type,
    parameters: &[MemoryWitnessParameter],
    instantiation: Option<&hir::GenericInstantiation>,
) -> Result<Vec<MemoryWitnessArgument>> {
    let Type::Forall { vars, .. } = callee_ty else {
        if instantiation.is_some() {
            return Err(Error::msg(
                "HIR non-generic direct call has witness substitutions",
            ));
        }
        return Ok(Vec::new());
    };
    let Some(instantiation) = instantiation else {
        if vars.is_empty() {
            return Ok(Vec::new());
        }
        return Err(Error::msg(
            "HIR direct generic call is missing witness substitutions",
        ));
    };
    let substitutions = &instantiation.substitutions;
    for (index, substitution) in substitutions.iter().enumerate() {
        if substitutions[..index]
            .iter()
            .any(|item| item.parameter == substitution.parameter)
        {
            return Err(Error::msg(
                "HIR direct generic call has duplicate witness substitution",
            ));
        }
    }
    if substitutions.len() < vars.len() {
        return Err(Error::msg(
            "HIR direct generic call is missing witness substitutions",
        ));
    }
    if substitutions.len() > vars.len() {
        return Err(Error::msg(
            "HIR direct generic call has extra witness substitutions",
        ));
    }
    if vars
        .iter()
        .zip(substitutions)
        .any(|(expected, actual)| expected != &actual.parameter)
    {
        return Err(Error::msg(
            "HIR direct generic call witness substitutions are reordered",
        ));
    }
    for substitution in substitutions {
        if unresolved_substitution(&substitution.ty)? {
            return Err(Error::msg(
                "HIR direct generic call witness substitution is unresolved",
            ));
        }
    }
    let mut output = Vec::with_capacity(parameters.len());
    for parameter in parameters {
        let substitution = substitutions
            .iter()
            .find(|item| item.parameter == parameter.parameter)
            .ok_or_else(|| {
                Error::msg("HIR direct generic call is missing witness substitutions")
            })?;
        let fact = planner.intern(&substitution.ty)?;
        output.push(MemoryWitnessArgument {
            parameter: parameter.parameter.clone(),
            witness: planner.fact(fact)?.witness,
        });
    }
    Ok(output)
}

fn type_contains_any_parameter(root: &Type) -> Result<bool> {
    visit_transport_type(root, |ty| matches!(ty, Type::Param(_)))
}

fn unresolved_substitution(root: &Type) -> Result<bool> {
    visit_transport_type(root, |ty| matches!(ty, Type::Param(_) | Type::Forall { .. }))
}

fn visit_transport_type(root: &Type, mut predicate: impl FnMut(&Type) -> bool) -> Result<bool> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| Error::host("HIR memory witness type traversal allocation failed"))?;
    pending.push(root);
    while let Some(ty) = pending.pop() {
        if predicate(ty) {
            return Ok(true);
        }
        match ty {
            Type::List(inner) => {
                pending.try_reserve(1).map_err(|_| {
                    Error::host("HIR memory witness type traversal allocation failed")
                })?;
                pending.push(inner);
            }
            Type::Enum { arguments, .. } => {
                pending.try_reserve(arguments.len()).map_err(|_| {
                    Error::host("HIR memory witness type traversal allocation failed")
                })?;
                pending.extend(arguments);
            }
            Type::Fn { params, ret } => {
                let additional = params
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| Error::host("HIR memory witness type child count overflow"))?;
                pending.try_reserve(additional).map_err(|_| {
                    Error::host("HIR memory witness type traversal allocation failed")
                })?;
                pending.push(ret);
                pending.extend(params);
            }
            Type::Forall { body, .. } => {
                pending.try_reserve(1).map_err(|_| {
                    Error::host("HIR memory witness type traversal allocation failed")
                })?;
                pending.push(body);
            }
            _ => {}
        }
    }
    Ok(false)
}
