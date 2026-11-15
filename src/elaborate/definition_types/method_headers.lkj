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
        location: (a::ModuleId, a::Span),
    ) -> Result<bool> {
        let (scope, span) = location;
        let Some(Shape::Nominal(declaration, receiver_arguments)) = self.shape(receiver) else {
            return Ok(false);
        };
        let head = self.source.declarations[declaration].name.as_str();
        let mut candidates = Vec::new();
        for index in self
            .implementations
            .method_candidates_by_head(Some(head), name)
        {
            if self.source.impls[index].trait_ty.is_none() {
                candidates.push(index);
            }
        }
        if candidates.len() != 1 {
            return Ok(false);
        }
        let implementation = self.source.impls[candidates[0]].clone();
        for parameter in &implementation.generics {
            if !parameter.bounds.is_empty() {
                return Ok(false);
            }
        }
        let mut methods = Vec::new();
        for index in &implementation.methods {
            if self.source.functions[*index].name == name {
                methods.push(*index);
            }
        }
        if methods.len() != 1 {
            return Ok(false);
        }
        let method = methods[0];
        let function = &self.source.functions[method];
        if !crate::resolve::visible(
            self.source,
            self.names.scope(&function.module),
            function.public,
            scope,
        ) || function.params.first().is_none_or(|p| p.name != "self")
            || function.params.len() != arguments.len() + 1
            || (!type_args.is_empty()
                && type_args.len() != type_parameter_count(&function.generics))
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
        let mut parameters = BTreeSet::<Variable>::new();
        for parameter in environment.values() {
            parameters.insert(*parameter);
        }
        let mut targets = BTreeSet::<Variable>::new();
        for argument in &target_arguments {
            targets.insert(*argument);
        }
        if targets != parameters || target_arguments.len() != parameters.len() {
            return Ok(false);
        }
        self.unify(target, receiver, span)?;
        environment.insert("Self".to_string(), receiver);
        let output = self.header_signature(method, type_args, arguments, environment, 1, span)?;
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
        location: (usize, a::ModuleId, a::Span),
    ) -> Result<bool> {
        let (id, scope, span) = location;
        if matches!(self.shape(receiver), None | Some(Shape::Rigid(_))) {
            self.watch(receiver, id);
            return Ok(false);
        }
        let head = self.shape_head(receiver);
        let head_view = match &head {
            Some(head) => Some(head.as_str()),
            None => None,
        };
        let candidates = self
            .implementations
            .method_candidates_by_head(head_view, name);
        let mut selected: Option<(usize, usize, BTreeMap<String, Variable>)> = None;
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
            if unavailable {
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
                        && type_args.len() != type_parameter_count(&function.generics))
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
        let generics = self.source.impls[candidate].generics.clone();
        self.parameter_equations(&generics, &mut environment, false)?;
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
        let Some(separator) = path.name.rfind("::") else {
            return Ok(None);
        };
        let owner = &path.name.as_str()[..separator];
        let name = &path.name.as_str()[separator + 2..];
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
        if candidates.len() != 1 {
            return Ok(None);
        }
        let (implementation, method) = candidates[0];
        let implementation = self.source.impls[implementation].clone();
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
        let no_type_args: Vec<a::TypeId> = Vec::new();
        Ok(Some(self.header_signature(
            method,
            &no_type_args,
            arguments,
            environment,
            0,
            span,
        )?))
    }
}

fn type_parameter_count(parameters: &[a::GenericParam]) -> usize {
    let mut count = 0;
    for parameter in parameters {
        if !parameter.lifetime {
            count += 1;
        }
    }
    count
}
