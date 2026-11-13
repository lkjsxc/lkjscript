//! Declaration headers add equations before numeric fallback. Unconditional
//! inherent heads have precedence; one visible structural candidate can also
//! constrain a callable signature. Competing candidates stay with the selector.
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

impl Infer<'_, '_> {
    /// A single visible structural candidate contributes conditional equations.
    /// Competing bounded headers stay unresolved: eligibility and inherent
    /// precedence still belong to the ordinary selector, not this graph.
    pub(super) fn contextual_header(
        &mut self,
        receiver: Variable,
        name: &str,
        type_args: &[a::TypeId],
        arguments: &[Variable],
        result: Variable,
        (id, scope, span): (usize, a::ModuleId, a::Span),
    ) -> Result<bool> {
        if matches!(self.shape(receiver), None | Some(Shape::Rigid(_))) {
            self.watch(receiver, id);
            return Ok(false);
        }
        let head = self.shape_head(receiver);
        let candidates = self
            .implementations
            .method_candidates_by_head(head.as_deref(), name);
        let mut selected = None;
        for candidate in candidates {
            let implementation = self.source.impls[candidate].clone();
            let trait_declaration =
                implementation
                    .trait_ty
                    .and_then(|ty| match &self.source.types[ty].kind {
                        a::Type::Path(path) => path.declaration,
                        _ => None,
                    });
            let unavailable = trait_declaration
                .is_some_and(|declaration| !self.names.trait_in_scope(scope, declaration, span));
            if unavailable && self.census_trait_visibility_complete(scope, span) {
                continue;
            }
            let environment = match self.implementation_environment(candidate, receiver)? {
                projection_equations::Match::No => continue,
                projection_equations::Match::Deferred => {
                    self.watch_shape(receiver, id);
                    return Ok(false);
                }
                projection_equations::Match::Yes(environment) => environment,
            };
            if unavailable {
                return Ok(false);
            }
            for method in &implementation.methods {
                let function = &self.source.functions[*method];
                if function.name != name {
                    continue;
                }
                if (trait_declaration.is_none()
                    && !crate::resolve::visible(
                        self.source,
                        self.names.scope(&function.module),
                        function.public,
                        scope,
                    ))
                    || function.params.first().is_none_or(|p| p.name != "self")
                    || function.params.len() != arguments.len() + 1
                    || (!type_args.is_empty()
                        && type_args.len()
                            != function.generics.iter().filter(|p| !p.lifetime).count())
                {
                    return Ok(false);
                }
                if selected.is_some() {
                    return Ok(false);
                }
                selected = Some((candidate, *method, environment.clone()));
            }
        }
        let Some((candidate, method, mut environment)) = selected else {
            return Ok(false);
        };
        self.parameter_equations(
            &self.source.impls[candidate].generics.clone(),
            &mut environment,
            false,
        )?;
        if let Some(bound) = self.source.impls[candidate].trait_ty {
            self.trait_equations(receiver, "Self", bound, &mut environment, false)?;
        }
        let output = self.header_signature(method, type_args, arguments, environment, 1, span)?;
        self.unify(result, output, span)?;
        Ok(true)
    }
}

impl Infer<'_, '_> {
    // A unique inherent associated header supplies equations for an omitted
    // owner type. Eligibility remains a later obligation of the normal selector.
    pub(super) fn associated_header(
        &mut self,
        path: &a::Path,
        arguments: &[Variable],
        scope: a::ModuleId,
        span: a::Span,
    ) -> Result<Option<Variable>> {
        let Some((owner, name)) = path.name.rsplit_once("::") else {
            return Ok(None);
        };
        let Some(declaration) = path.declaration else {
            return Ok(None);
        };
        if !path.args.is_empty()
            || !matches!(
                self.source.declarations[declaration].item,
                a::Item::Struct(_) | a::Item::Enum(_)
            )
        {
            return Ok(None);
        }
        let mut candidates = Vec::new();
        for index in self
            .implementations
            .method_candidates_by_head(Some(owner), name)
        {
            let implementation = &self.source.impls[index];
            if implementation.trait_ty.is_some() {
                continue;
            }
            for method in &implementation.methods {
                let function = &self.source.functions[*method];
                if function.name == name
                    && function.params.first().is_none_or(|p| p.name != "self")
                    && function.params.len() == arguments.len()
                    && crate::resolve::visible(
                        self.source,
                        self.names.scope(&function.module),
                        function.public,
                        scope,
                    )
                {
                    candidates.push((index, *method));
                }
            }
        }
        let [(implementation, method)] = candidates.as_slice() else {
            return Ok(None);
        };
        let implementation = self.source.impls[*implementation].clone();
        let method = *method;
        let mut environment = BTreeMap::new();
        for parameter in &implementation.generics {
            if !parameter.lifetime {
                environment.insert(parameter.name.clone(), self.unknown());
            }
        }
        let target = self.source_type(implementation.target, &environment)?;
        if !matches!(self.shape(target), Some(Shape::Nominal(found, _)) if found == declaration) {
            return Ok(None);
        }
        environment.insert("Self".to_string(), target);
        self.parameter_equations(&implementation.generics, &mut environment, false)?;
        Ok(Some(self.header_signature(
            method,
            &[],
            arguments,
            environment,
            0,
            span,
        )?))
    }
}
