use super::*;

pub(super) fn verified_witness_parameters(ty: &Type) -> Result<Vec<MemoryWitnessParameter>> {
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
            _ if verified_type_contains_any_parameter(ty) => {
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
            output.push(MemoryWitnessParameter {
                parameter: variable.clone(),
                operations: vec![MemoryWitnessOperation::Transport],
            });
        }
    }
    if u64::try_from(output.len()).unwrap_or(u64::MAX) > MAX_MEMORY_WITNESS_PARAMETERS {
        return Err(Error::msg("memory verifier witness parameters exceed 16"));
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
    if substitutions
        .iter()
        .any(|item| verified_unresolved_substitution(&item.ty))
    {
        return Err(Error::msg(
            "memory verifier found unresolved direct generic substitution",
        ));
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
    if u64::try_from(output.len()).unwrap_or(u64::MAX) > MAX_MEMORY_WITNESS_ARGUMENTS {
        return Err(Error::msg("memory verifier witness arguments exceed 16"));
    }
    Ok(output)
}

fn verified_type_contains_any_parameter(ty: &Type) -> bool {
    match ty {
        Type::Param(_) => true,
        Type::List(inner) => verified_type_contains_any_parameter(inner),
        Type::Enum { arguments, .. } => arguments.iter().any(verified_type_contains_any_parameter),
        Type::Fn { params, ret } => {
            params.iter().any(verified_type_contains_any_parameter)
                || verified_type_contains_any_parameter(ret)
        }
        Type::Forall { body, .. } => verified_type_contains_any_parameter(body),
        _ => false,
    }
}

fn verified_unresolved_substitution(ty: &Type) -> bool {
    match ty {
        Type::Param(_) | Type::Forall { .. } => true,
        Type::List(inner) => verified_unresolved_substitution(inner),
        Type::Enum { arguments, .. } => arguments.iter().any(verified_unresolved_substitution),
        Type::Fn { params, ret } => {
            params.iter().any(verified_unresolved_substitution)
                || verified_unresolved_substitution(ret)
        }
        _ => false,
    }
}
