use super::*;

pub(super) fn verified_witness_parameters(
    ty: &Type,
    function_body: Option<&hir::Expr>,
) -> Result<Vec<MemoryWitnessParameter>> {
    let Type::Forall { vars, body } = ty else {
        return Ok(Vec::new());
    };
    let Type::Fn { params, ret } = body.as_ref() else {
        return Err(Error::msg(
            "memory verifier found non-function witness forall body",
        ));
    };
    for (index, variable) in vars.iter().enumerate() {
        if vars[..index].contains(variable) {
            return Err(Error::msg(
                "memory verifier found duplicate witness type parameter",
            ));
        }
    }
    for ty in params.iter().chain(std::iter::once(ret.as_ref())) {
        match ty {
            Type::Param(name) if !vars.contains(name) => {
                return Err(Error::msg(
                    "memory verifier found unresolved witness type parameter",
                ));
            }
            Type::Param(_) => {}
            _ if verified_type_contains_any_parameter(ty)? => {
                return Err(Error::msg(
                    "memory verifier rejected nested witness parameter use",
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
            let occurrences = params
                .iter()
                .filter(|ty| matches!(ty, Type::Param(name) if name == variable))
                .count();
            let mut operations = vec![MemoryWitnessOperation::Transport];
            let compare =
                function_body.is_some_and(|body| verifier_demands_compare(body, variable));
            if compare {
                operations.push(MemoryWitnessOperation::Compare);
            }
            if occurrences >= 2
                && !(compare
                    && function_body.is_some_and(|body| verifier_compare_only(body, variable)))
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

pub(super) fn verified_witness_arguments(
    types: &mut VerifiedTypes<'_>,
    callee_ty: &Type,
    parameters: &[MemoryWitnessParameter],
    instantiation: Option<&hir::GenericInstantiation>,
) -> Result<Vec<MemoryWitnessArgument>> {
    let Type::Forall { vars, .. } = callee_ty else {
        if instantiation.is_some() {
            return Err(Error::msg(
                "memory verifier found substitutions on a non-generic call",
            ));
        }
        return Ok(Vec::new());
    };
    let Some(instantiation) = instantiation else {
        if vars.is_empty() {
            return Ok(Vec::new());
        }
        return Err(Error::msg(
            "memory verifier found missing direct generic substitutions",
        ));
    };
    let substitutions = &instantiation.substitutions;
    for (index, substitution) in substitutions.iter().enumerate() {
        if substitutions[..index]
            .iter()
            .any(|item| item.parameter == substitution.parameter)
        {
            return Err(Error::msg(
                "memory verifier found duplicate direct generic substitution",
            ));
        }
    }
    if substitutions.len() != vars.len()
        || vars
            .iter()
            .zip(substitutions)
            .any(|(expected, actual)| expected != &actual.parameter)
    {
        return Err(Error::msg(
            "memory verifier rejected direct generic substitution order/count",
        ));
    }
    for substitution in substitutions {
        if verified_unresolved_substitution(&substitution.ty)? {
            return Err(Error::msg(
                "memory verifier found unresolved direct generic substitution",
            ));
        }
    }
    let mut output = Vec::with_capacity(parameters.len());
    for parameter in parameters {
        let substitution = substitutions
            .iter()
            .find(|item| item.parameter == parameter.parameter)
            .ok_or_else(|| Error::msg("memory verifier lost direct generic substitution"))?;
        let id = types.intern(&substitution.ty)?;
        output.push(MemoryWitnessArgument {
            parameter: parameter.parameter.clone(),
            witness: types.expected(id)?.witness,
        });
    }
    Ok(output)
}

fn verified_type_contains_any_parameter(root: &Type) -> Result<bool> {
    verified_visit_witness_type(root, |ty| matches!(ty, Type::Param(_)))
}

fn verified_unresolved_substitution(root: &Type) -> Result<bool> {
    verified_visit_witness_type(root, |ty| {
        matches!(ty, Type::Param(_) | Type::Forall { .. })
    })
}

fn verified_visit_witness_type(
    root: &Type,
    mut predicate: impl FnMut(&Type) -> bool,
) -> Result<bool> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| Error::host("memory verifier witness type traversal allocation failed"))?;
    pending.push(root);
    while let Some(ty) = pending.pop() {
        if predicate(ty) {
            return Ok(true);
        }
        match ty {
            Type::List(inner) => {
                pending.try_reserve(1).map_err(|_| {
                    Error::host("memory verifier witness type traversal allocation failed")
                })?;
                pending.push(inner);
            }
            Type::Enum { arguments, .. } => {
                pending.try_reserve(arguments.len()).map_err(|_| {
                    Error::host("memory verifier witness type traversal allocation failed")
                })?;
                pending.extend(arguments);
            }
            Type::Fn { params, ret } => {
                let additional = params.len().checked_add(1).ok_or_else(|| {
                    Error::host("memory verifier witness type child count overflow")
                })?;
                pending.try_reserve(additional).map_err(|_| {
                    Error::host("memory verifier witness type traversal allocation failed")
                })?;
                pending.push(ret);
                pending.extend(params);
            }
            Type::Forall { body, .. } => {
                pending.try_reserve(1).map_err(|_| {
                    Error::host("memory verifier witness type traversal allocation failed")
                })?;
                pending.push(body);
            }
            _ => {}
        }
    }
    Ok(false)
}
