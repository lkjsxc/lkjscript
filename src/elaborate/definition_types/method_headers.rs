//! Unconditional inherent headers add equations before numeric fallback.
//! This is deliberately not method selection: bounded, specialized, overloaded,
//! and trait candidates retain the existing definition/body selector.
use super::*;
use std::collections::BTreeSet;

impl Infer<'_, '_> {
    pub(super) fn inherent_header(
        &mut self,
        receiver: Variable,
        name: &str,
        type_args: &[a::TypeId],
        arguments: &[Variable],
        result: Variable,
        (scope, span): (a::ModuleId, a::Span),
    ) -> Result<bool> {
        let Some(Shape::Nominal(declaration, receiver_arguments)) = self.shape(receiver) else {
            return Ok(false);
        };
        let head = &self.source.declarations[declaration].name;
        let candidates = self
            .implementations
            .method_candidates_by_head(Some(head), name)
            .into_iter()
            .filter(|index| self.source.impls[*index].trait_ty.is_none())
            .collect::<Vec<_>>();
        let [implementation] = candidates.as_slice() else {
            return Ok(false);
        };
        let implementation = self.source.impls[*implementation].clone();
        if implementation
            .generics
            .iter()
            .any(|parameter| !parameter.bounds.is_empty())
        {
            return Ok(false);
        }
        let methods = implementation
            .methods
            .iter()
            .copied()
            .filter(|index| self.source.functions[*index].name == name)
            .collect::<Vec<_>>();
        let [method] = methods.as_slice() else {
            return Ok(false);
        };
        let function = &self.source.functions[*method];
        if !crate::resolve::visible(
            self.source,
            self.names.scope(&function.module),
            function.public,
            scope,
        ) || function.params.first().is_none_or(|p| p.name != "self")
            || function.params.len() != arguments.len() + 1
            || (!type_args.is_empty()
                && type_args.len() != function.generics.iter().filter(|p| !p.lifetime).count())
        {
            return Ok(false);
        }
        let mut environment = BTreeMap::new();
        for parameter in &implementation.generics {
            if !parameter.lifetime {
                environment.insert(parameter.name.clone(), self.unknown());
            }
        }
        let target = self.source_type(implementation.target, &environment)?;
        let Some(Shape::Nominal(target_declaration, target_arguments)) = self.shape(target) else {
            return Ok(false);
        };
        if target_declaration != declaration || target_arguments.len() != receiver_arguments.len() {
            return Ok(false);
        }
        // Owner<A, B> with each impl parameter used once covers the entire
        // canonical receiver head. Owner<Option<A>>, fixed arguments, repeated
        // parameters and unresolved prerequisites cannot establish this fact.
        let parameters = environment.values().copied().collect::<BTreeSet<_>>();
        if target_arguments.iter().copied().collect::<BTreeSet<_>>() != parameters
            || target_arguments.len() != parameters.len()
        {
            return Ok(false);
        }
        self.unify(target, receiver, span)?;
        environment.insert("Self".to_string(), receiver);
        let output = self.header_signature(*method, type_args, arguments, environment, 1, span)?;
        self.unify(result, output, span)?;
        Ok(true)
    }
}
