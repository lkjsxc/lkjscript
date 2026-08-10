use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn call_result(
        &self,
        name: &str,
        callee: BindingId,
        callable: Type,
        args: &[Expr],
    ) -> Result<(Type, Option<GenericInstantiation>)> {
        let substitutions = self.infer_substitutions(name, &callable, args)?;
        let mut argument_types = Vec::new();
        argument_types
            .try_reserve(args.len())
            .map_err(|_| Error::host("generic call argument type allocation failed"))?;
        argument_types.extend(args.iter().map(|argument| argument.ty.clone()));
        let bounds = self
            .analyzer
            .function_bounds
            .get(&callee)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let facts = crate::generic_call::GenericFacts {
            traits: &self.analyzer.traits,
            products: &self.analyzer.products,
            implementations: &self.analyzer.implementations,
            product_names: &self.analyzer.product_names,
            implementation_index: &self.analyzer.implementation_index,
        };
        match crate::generic_call::resolve_exact(
            &callable,
            substitutions,
            &argument_types,
            bounds,
            &facts,
        ) {
            Ok(exact) => Ok((exact.result, exact.instantiation)),
            Err(crate::generic_call::GenericCallError::Arity { expected, actual }) => Err(self
                .diagnostic(AnalysisDiagnostic::CallArity {
                    name: name.to_string(),
                    expected,
                    actual,
                })),
            Err(crate::generic_call::GenericCallError::TypeMismatch {
                expected, actual, ..
            }) => Err(self.diagnostic(AnalysisDiagnostic::TypeMismatch {
                context: format!("{name}: arg type"),
                expected: expected.to_string(),
                actual: actual.to_string(),
            })),
            Err(crate::generic_call::GenericCallError::Host(message)) => Err(Error::host(message)),
            Err(error) => Err(self.error(format!("{name}: {error}"))),
        }
    }
}
