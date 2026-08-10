use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn infer_substitutions(
        &self,
        name: &str,
        callable: &Type,
        args: &[Expr],
    ) -> Result<Vec<TypeSubstitution>> {
        let Type::Forall { vars, body } = callable else {
            return Ok(Vec::new());
        };
        let Type::Fn { params, .. } = body.as_ref() else {
            return Err(self.error("forall body must be a function type"));
        };
        if params.len() != args.len() {
            return Ok(Vec::new());
        }
        let mut substitutions = HashMap::new();
        let empty_substitutions = HashMap::new();
        substitutions
            .try_reserve(vars.len())
            .map_err(|_| Error::host("generic inference allocation failed"))?;
        let mut pending = Vec::new();
        pending
            .try_reserve(params.len())
            .map_err(|_| Error::host("generic inference work allocation failed"))?;
        pending.extend(
            params
                .iter()
                .zip(args)
                .map(|(pattern, argument)| (pattern, &argument.ty)),
        );
        while let Some((pattern, got)) = pending.pop() {
            match (pattern, got) {
                (Type::Param(parameter), got)
                    if vars.iter().any(|variable| variable == parameter) =>
                {
                    if let Some(previous) = substitutions.get(parameter) {
                        if previous != got {
                            return Err(self.error(format!(
                                "{name}: type param {parameter} conflict: {previous} vs {got}"
                            )));
                        }
                    } else {
                        let inferred =
                            crate::generic_call::substitute_type(got, &empty_substitutions)
                                .map_err(|error| match error {
                                    crate::generic_call::GenericCallError::Host(message) => {
                                        Error::host(message)
                                    }
                                    other => Error::msg(other.to_string()),
                                })?;
                        substitutions.insert(parameter.clone(), inferred);
                    }
                }
                (Type::List(pattern), Type::List(got)) => {
                    pending
                        .try_reserve(1)
                        .map_err(|_| Error::host("generic inference work allocation failed"))?;
                    pending.push((pattern, got));
                }
                (
                    Type::Enum {
                        id: pattern_id,
                        arguments: patterns,
                        ..
                    },
                    Type::Enum {
                        id: got_id,
                        arguments: got_arguments,
                        ..
                    },
                ) if pattern_id == got_id && patterns.len() == got_arguments.len() => {
                    pending
                        .try_reserve(patterns.len())
                        .map_err(|_| Error::host("generic inference work allocation failed"))?;
                    pending.extend(patterns.iter().zip(got_arguments));
                }
                (pattern, got) => {
                    let assignable =
                        crate::generic_call::types_assignable(got, pattern).map_err(|error| {
                            match error {
                                crate::generic_call::GenericCallError::Host(message) => {
                                    Error::host(message)
                                }
                                other => Error::msg(other.to_string()),
                            }
                        })?;
                    if !assignable {
                        return Err(
                            self.error(format!("{name}: cannot instantiate {pattern} from {got}"))
                        );
                    }
                }
            }
        }
        let mut canonical = Vec::new();
        canonical
            .try_reserve(vars.len())
            .map_err(|_| Error::host("generic substitution allocation failed"))?;
        for parameter in vars {
            let ty = substitutions.remove(parameter).ok_or_else(|| {
                self.error(format!(
                    "{name}: cannot infer type parameter {parameter} from arguments"
                ))
            })?;
            canonical.push(TypeSubstitution {
                parameter: parameter.clone(),
                ty,
            });
        }
        Ok(canonical)
    }
}
