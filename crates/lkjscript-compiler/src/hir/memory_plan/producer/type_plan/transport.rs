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
            _ if type_contains_any_parameter(ty) => {
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
            if occurrences >= 2 && function_body.is_some_and(nontrivial_owner_body) {
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
    if u64::try_from(output.len()).unwrap_or(u64::MAX) > MAX_MEMORY_WITNESS_PARAMETERS {
        return Err(Error::msg("HIR memory witness parameters exceed 16"));
    }
    Ok(output)
}

fn nontrivial_owner_body(body: &Expr) -> bool {
    !matches!(
        body.kind,
        ExprKind::Load(_) | ExprKind::Move { .. } | ExprKind::LitUnit
    )
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
    if substitutions
        .iter()
        .any(|item| unresolved_substitution(&item.ty))
    {
        return Err(Error::msg(
            "HIR direct generic call witness substitution is unresolved",
        ));
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
    if u64::try_from(output.len()).unwrap_or(u64::MAX) > MAX_MEMORY_WITNESS_ARGUMENTS {
        return Err(Error::msg("HIR memory witness arguments exceed 16"));
    }
    Ok(output)
}

fn type_contains_any_parameter(ty: &Type) -> bool {
    match ty {
        Type::Param(_) => true,
        Type::List(inner) => type_contains_any_parameter(inner),
        Type::Enum { arguments, .. } => arguments.iter().any(type_contains_any_parameter),
        Type::Fn { params, ret } => {
            params.iter().any(type_contains_any_parameter)
                || type_contains_any_parameter(ret)
        }
        Type::Forall { body, .. } => type_contains_any_parameter(body),
        _ => false,
    }
}

fn unresolved_substitution(ty: &Type) -> bool {
    match ty {
        Type::Param(_) | Type::Forall { .. } => true,
        Type::List(inner) => unresolved_substitution(inner),
        Type::Enum { arguments, .. } => arguments.iter().any(unresolved_substitution),
        Type::Fn { params, ret } => {
            params.iter().any(unresolved_substitution) || unresolved_substitution(ret)
        }
        _ => false,
    }
}
