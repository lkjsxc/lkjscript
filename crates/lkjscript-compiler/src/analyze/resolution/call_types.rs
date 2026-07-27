use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn call_result(
        &self,
        name: &str,
        callee: BindingId,
        callable: Type,
        args: &[Expr],
    ) -> Result<(Type, Option<GenericInstantiation>)> {
        let is_generic = matches!(&callable, Type::Forall { .. });
        let generic_signature_has_ownership = is_generic && contains_ownership_type(&callable);
        let (instantiated, substitutions) = self.instantiate(name, callable, args)?;
        if is_generic
            && (generic_signature_has_ownership
                || substitutions
                    .iter()
                    .any(|substitution| contains_ownership_type(&substitution.ty)))
        {
            return Err(self.error(format!(
                "{name}: ownership/reference generic instantiation is unavailable in the initial ownership slice"
            )));
        }
        let Type::Fn { params, ret } = instantiated else {
            return Err(self.error(format!("{name} is not a function")));
        };
        if params.len() != args.len() {
            return Err(self.diagnostic(AnalysisDiagnostic::CallArity {
                name: name.to_string(),
                expected: params.len(),
                actual: args.len(),
            }));
        }
        for (parameter, argument) in params.iter().zip(args) {
            if !Type::unify_assignable(&argument.ty, parameter) {
                return Err(self.diagnostic(AnalysisDiagnostic::TypeMismatch {
                    context: format!("{name}: arg type"),
                    expected: format!("{parameter}"),
                    actual: format!("{}", argument.ty),
                }));
            }
        }
        if contains_reference_type(&ret) {
            return Err(self.error(format!(
                "{name}: user-call results cannot be lexical references in the initial ownership slice"
            )));
        }
        let instantiation = self.call_instantiation(name, callee, substitutions)?;
        Ok((*ret, instantiation))
    }

    fn call_instantiation(
        &self,
        name: &str,
        callee: BindingId,
        substitutions: Vec<TypeSubstitution>,
    ) -> Result<Option<GenericInstantiation>> {
        if substitutions.is_empty() {
            return Ok(None);
        }
        let bounds = self
            .analyzer
            .function_bounds
            .get(&callee)
            .cloned()
            .unwrap_or_default();
        if !bounds.is_empty() {
            for substitution in &substitutions {
                let mut unresolved = HashSet::new();
                collect_type_params(&substitution.ty, &mut unresolved);
                if !unresolved.is_empty() {
                    return Err(self.error(format!(
                        "{name}: forwarding bounded calls from a generic context is \
                         unavailable in the marker-trait slice"
                    )));
                }
            }
        }
        let mut witnesses = Vec::with_capacity(bounds.len());
        for bound in bounds {
            let ty = substitutions
                .iter()
                .find(|substitution| substitution.parameter == bound.parameter)
                .map(|substitution| substitution.ty.clone())
                .ok_or_else(|| {
                    self.error(format!(
                        "{name}: missing substitution for bound parameter {}",
                        bound.parameter
                    ))
                })?;
            witnesses.push(self.solve_trait_bound(name, bound.trait_id, &ty)?);
        }
        Ok(Some(GenericInstantiation {
            substitutions,
            witnesses,
        }))
    }
}
