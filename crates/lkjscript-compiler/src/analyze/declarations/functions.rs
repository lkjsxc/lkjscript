use crate::analyze::*;

impl Analyzer {
    pub(in crate::analyze) fn resolve_main_parameter_types(
        &self,
        source: SourceId,
        mut types: Vec<Type>,
    ) -> Result<Vec<Type>> {
        for ty in &mut types {
            *ty = self
                .resolve_enum_type(ty, &[])
                .map_err(|message| self.error(source, format!("main: {message}")))?;
            self.validate_product_type(ty)
                .map_err(|message| self.error(source, format!("main: {message}")))?;
        }
        Ok(types)
    }
    pub(in crate::analyze) fn resolve_main(&mut self, pending: PendingMain<'_>) -> Result<Main> {
        if pending.return_type.contains_never() {
            return Err(self.error(pending.origin, "never is not a public main return payload"));
        }
        if contains_resource_type(&pending.return_type) {
            return Err(self.error(
                pending.origin,
                "typed resources cannot escape as a main result",
            ));
        }
        let arity = pending.param_names.len();
        let mut params = Vec::with_capacity(pending.param_names.len());
        let mut scope = HashMap::new();
        let mut local_slots = HashMap::new();
        for (index, (name, ty)) in pending
            .param_names
            .iter()
            .zip(&pending.param_types)
            .enumerate()
        {
            let slot = index;
            let id = self.add_binding(
                name.clone(),
                BindingKind::Parameter,
                ty.clone(),
                Origin::Source(pending.origin),
            )?;
            scope.insert(name.clone(), id);
            local_slots.insert(id, slot);
            params.push(id);
        }
        let (body, local_count, param_places) = {
            let mut resolver = Resolver::new(
                self,
                pending.origin,
                scope,
                local_slots,
                HashSet::new(),
                params.len(),
                pending.return_type.clone(),
            )?;
            let param_places = params
                .iter()
                .map(|parameter| resolver.place(*parameter))
                .collect::<Result<Vec<_>>>()?;
            let body = resolver.resolve_expr(pending.body)?;
            let local_count = resolver.local_count();
            (body, local_count, param_places)
        };
        if body.ty != Type::Never && body.ty != pending.return_type {
            return Err(self.error(
                pending.origin,
                format!(
                    "main body type {} does not exactly equal declared return {}",
                    body.ty, pending.return_type
                ),
            ));
        }
        Ok(Main {
            origin: pending.origin,
            params,
            param_places,
            param_types: pending.param_types,
            return_type: pending.return_type,
            arity,
            local_count,
            body,
        })
    }
    pub(in crate::analyze) fn resolve_function(
        &mut self,
        binding: BindingId,
        origin: SourceId,
        parsed: ParsedFunction<'_>,
        bounds: Vec<TraitBound>,
    ) -> Result<Function> {
        if parsed.signature_params.iter().any(Type::contains_never)
            || parsed.signature_return.contains_never()
        {
            return Err(self.error(
                origin,
                "never is not permitted in parameters or public return payloads",
            ));
        }
        if parsed
            .signature_params
            .iter()
            .any(|ty| contains_resource_type(ty) && !matches!(ty, Type::Resource(_)))
            || (contains_resource_type(&parsed.signature_return)
                && !matches!(parsed.signature_return, Type::Resource(_)))
        {
            return Err(self.error(
                origin,
                "resource-bearing aggregates are unsupported function boundaries",
            ));
        }
        let arity = parsed.param_names.len();
        let mut params = Vec::with_capacity(parsed.param_names.len());
        let mut scope = HashMap::new();
        let mut local_slots = HashMap::new();
        for (index, (name, ty)) in parsed
            .param_names
            .iter()
            .zip(&parsed.param_types)
            .enumerate()
        {
            let id = self.add_binding(
                name.clone(),
                BindingKind::Parameter,
                ty.clone(),
                Origin::Source(origin),
            )?;
            scope.insert(name.clone(), id);
            local_slots.insert(id, index);
            params.push(id);
        }
        let (body, local_count, param_places) = {
            let type_variables = parsed.forall_vars.iter().cloned().collect();
            let mut resolver = Resolver::new(
                self,
                origin,
                scope,
                local_slots,
                type_variables,
                params.len(),
                parsed.signature_return.clone(),
            )?;
            let param_places = params
                .iter()
                .map(|parameter| resolver.place(*parameter))
                .collect::<Result<Vec<_>>>()?;
            let body = resolver.resolve_expr(parsed.body)?;
            let local_count = resolver.local_count();
            (body, local_count, param_places)
        };
        if body.ty != Type::Never && !Type::unify_assignable(&body.ty, &parsed.signature_return) {
            let name = self.binding(binding)?.name.clone();
            return Err(self.error(
                origin,
                format!(
                    "def {name}: body type {} not assignable to {}",
                    body.ty, parsed.signature_return
                ),
            ));
        }
        Ok(Function {
            binding,
            origin,
            params,
            param_places,
            bounds,
            arity,
            local_count,
            summary: EffectSet::PURE,
            body,
        })
    }
    pub(in crate::analyze) fn build_global_layout(
        &self,
        functions: &[Function],
    ) -> Result<Vec<BindingId>> {
        let mut layout = Vec::with_capacity(functions.len());
        let mut seen = HashSet::new();
        for function in functions {
            record_global(function.binding, &mut layout, &mut seen)?;
        }
        Ok(layout)
    }
}
