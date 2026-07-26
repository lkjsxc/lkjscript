use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn instantiate(
        &self,
        name: &str,
        callable: Type,
        args: &[Expr],
    ) -> Result<(Type, Vec<TypeSubstitution>)> {
        let Type::Forall { vars, body } = callable else {
            return Ok((callable, Vec::new()));
        };
        let Type::Fn { params, ret } = *body else {
            return Err(self.error("forall body must be a function type"));
        };
        if params.len() != args.len() {
            return Ok((Type::Fn { params, ret }, Vec::new()));
        }
        let mut substitutions = HashMap::new();
        for (pattern, argument) in params.iter().zip(args) {
            self.bind_type_params(name, pattern, &argument.ty, &vars, &mut substitutions)?;
        }
        for variable in &vars {
            if !substitutions.contains_key(variable) {
                return Err(self.error(format!(
                    "{name}: cannot infer type parameter {variable} from arguments"
                )));
            }
        }
        let canonical = vars
            .iter()
            .map(|parameter| TypeSubstitution {
                parameter: parameter.clone(),
                ty: substitutions
                    .get(parameter)
                    .cloned()
                    .unwrap_or(Type::Param(parameter.clone())),
            })
            .collect();
        Ok((
            Type::Fn {
                params: params
                    .iter()
                    .map(|parameter| parameter.subst(&substitutions))
                    .collect(),
                ret: Box::new(ret.subst(&substitutions)),
            },
            canonical,
        ))
    }

    pub(in crate::analyze) fn bind_type_params(
        &self,
        function: &str,
        pattern: &Type,
        got: &Type,
        variables: &[String],
        substitutions: &mut HashMap<String, Type>,
    ) -> Result<()> {
        match (pattern, got) {
            (Type::Param(parameter), got)
                if variables.iter().any(|variable| variable == parameter) =>
            {
                if let Some(previous) = substitutions.get(parameter) {
                    if previous != got {
                        return Err(self.error(format!(
                            "{function}: type param {parameter} conflict: {previous:?} vs {got:?}"
                        )));
                    }
                } else {
                    substitutions.insert(parameter.clone(), got.clone());
                }
                Ok(())
            }
            (Type::Owned(pattern), Type::Owned(got))
            | (Type::Ref(pattern), Type::Ref(got))
            | (Type::RefMut(pattern), Type::RefMut(got))
            | (Type::List(pattern), Type::List(got)) => {
                self.bind_type_params(function, pattern, got, variables, substitutions)
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
                for (pattern, got) in patterns.iter().zip(got_arguments) {
                    self.bind_type_params(function, pattern, got, variables, substitutions)?;
                }
                Ok(())
            }
            (pattern, got) if Type::unify_assignable(got, pattern) => Ok(()),
            (pattern, got) => Err(self.error(format!(
                "{function}: cannot instantiate {pattern:?} from {got:?}"
            ))),
        }
    }
}
