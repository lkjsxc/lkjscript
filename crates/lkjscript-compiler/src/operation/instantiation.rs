use crate::operation::*;

pub(in crate::operation) fn instantiate_result(
    name: &str,
    callable: Type,
    arguments: &[Type],
) -> Result<Type, String> {
    let (parameters, result) = match callable {
        Type::Forall { vars, body } => {
            let Type::Fn { params, ret } = *body else {
                return Err(format!("{name}: forall body is not a function"));
            };
            let mut substitutions = std::collections::HashMap::new();
            for (pattern, argument) in params.iter().zip(arguments) {
                bind_type_params(name, pattern, argument, &vars, &mut substitutions)?;
            }
            for variable in &vars {
                if !substitutions.contains_key(variable) {
                    return Err(format!(
                        "{name}: cannot infer type parameter {variable} from arguments"
                    ));
                }
            }
            (
                params
                    .iter()
                    .map(|parameter| parameter.subst(&substitutions))
                    .collect(),
                ret.subst(&substitutions),
            )
        }
        Type::Fn { params, ret } => (params, *ret),
        other => return Err(format!("{name} is not a function ({other:?})")),
    };
    if parameters.len() != arguments.len() {
        return Err(format!(
            "{name}: expected {} args, got {}",
            parameters.len(),
            arguments.len()
        ));
    }
    for (parameter, argument) in parameters.iter().zip(arguments) {
        if !Type::unify_assignable(argument, parameter) {
            return Err(format!(
                "{name}: arg type {argument:?} not assignable to {parameter:?}"
            ));
        }
    }
    Ok(result)
}

pub(in crate::operation) fn bind_type_params(
    name: &str,
    pattern: &Type,
    argument: &Type,
    variables: &[String],
    substitutions: &mut std::collections::HashMap<String, Type>,
) -> Result<(), String> {
    match (pattern, argument) {
        (Type::Param(parameter), argument) if variables.iter().any(|item| item == parameter) => {
            if let Some(previous) = substitutions.get(parameter) {
                if previous != argument {
                    return Err(format!(
                        "{name}: type parameter {parameter} conflict: {previous:?} vs {argument:?}"
                    ));
                }
            } else {
                substitutions.insert(parameter.clone(), argument.clone());
            }
            Ok(())
        }
        (Type::Owned(pattern), Type::Owned(argument))
        | (Type::Ref(pattern), Type::Ref(argument))
        | (Type::RefMut(pattern), Type::RefMut(argument))
        | (Type::List(pattern), Type::List(argument)) => {
            bind_type_params(name, pattern, argument, variables, substitutions)
        }
        (
            Type::Enum {
                id: pattern_id,
                arguments: patterns,
                ..
            },
            Type::Enum {
                id: argument_id,
                arguments,
                ..
            },
        ) if pattern_id == argument_id && patterns.len() == arguments.len() => {
            for (pattern, argument) in patterns.iter().zip(arguments) {
                bind_type_params(name, pattern, argument, variables, substitutions)?;
            }
            Ok(())
        }
        (pattern, argument) if Type::unify_assignable(argument, pattern) => Ok(()),
        (pattern, argument) => Err(format!(
            "{name}: cannot instantiate {pattern:?} from {argument:?}"
        )),
    }
}

pub(in crate::operation) fn callable_arity(ty: &Type) -> Option<usize> {
    match ty {
        Type::Fn { params, .. } => Some(params.len()),
        Type::Forall { body, .. } => callable_arity(body),
        _ => None,
    }
}

pub(in crate::operation) fn supports_value_equality(ty: &Type) -> bool {
    match ty {
        Type::Unit | Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Symbol => true,
        Type::Enum { id, arguments, .. }
            if matches!(
                id.bytes(),
                lkjscript_core::OPTION_ID | lkjscript_core::RESULT_ID
            ) =>
        {
            arguments.iter().all(supports_value_equality)
        }
        Type::Never
        | Type::Capability(_)
        | Type::Buf
        | Type::Owned(_)
        | Type::Ref(_)
        | Type::RefMut(_)
        | Type::Handle
        | Type::Product(_)
        | Type::Enum { .. }
        | Type::Param(_)
        | Type::List(_)
        | Type::Fn { .. }
        | Type::Forall { .. } => false,
    }
}

pub(in crate::operation) fn both_numeric(left: &Type, right: &Type) -> bool {
    matches!(left, Type::I64 | Type::F64) && matches!(right, Type::I64 | Type::F64)
}

pub(in crate::operation) fn function(params: Vec<Type>, ret: Type) -> Type {
    Type::Fn {
        params,
        ret: Box::new(ret),
    }
}

pub(in crate::operation) fn forall(vars: &[&str], body: Type) -> Type {
    Type::Forall {
        vars: vars.iter().map(|name| (*name).to_string()).collect(),
        body: Box::new(body),
    }
}

pub(in crate::operation) fn generic_result() -> Type {
    crate::types::result_type(Type::Param("T".into()), Type::Param("E".into()))
}
