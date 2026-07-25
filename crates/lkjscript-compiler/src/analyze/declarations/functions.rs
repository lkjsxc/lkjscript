use crate::analyze::*;

impl Analyzer {
    pub(in crate::analyze) fn resolve_main(&mut self, pending: PendingMain<'_>) -> Result<Main> {
        let (body, local_count) = {
            let mut resolver = Resolver::new(
                self,
                pending.origin,
                HashMap::new(),
                HashMap::new(),
                HashSet::new(),
                0,
            );
            let body = resolver.resolve_expr(pending.body)?;
            let local_count = resolver.local_count()?;
            (body, local_count)
        };
        if body.ty != pending.return_type {
            return Err(self.error(
                pending.origin,
                format!(
                    "main body type {:?} does not exactly equal declared return {:?}",
                    body.ty, pending.return_type
                ),
            ));
        }
        Ok(Main {
            origin: pending.origin,
            return_type: pending.return_type,
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
        let arity = u8::try_from(parsed.param_names.len()).map_err(|_| {
            self.error(
                origin,
                "function has too many parameters for bytecode arity",
            )
        })?;
        let mut params = Vec::with_capacity(parsed.param_names.len());
        let mut scope = HashMap::new();
        let mut local_slots = HashMap::new();
        for (index, (name, ty)) in parsed
            .param_names
            .iter()
            .zip(&parsed.param_types)
            .enumerate()
        {
            let _slot = u8::try_from(index)
                .map_err(|_| self.error(origin, "function has too many parameter local slots"))?;
            let id = self.add_binding(
                name.clone(),
                BindingKind::Parameter,
                ty.clone(),
                Origin::Source(origin),
            )?;
            scope.insert(name.clone(), id);
            local_slots.insert(
                id,
                u8::try_from(index).map_err(|_| {
                    self.error(origin, "function has too many parameter local slots")
                })?,
            );
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
            );
            let param_places = params
                .iter()
                .map(|parameter| resolver.place(*parameter))
                .collect::<Result<Vec<_>>>()?;
            let body = resolver.resolve_expr(parsed.body)?;
            let local_count = resolver.local_count()?;
            (body, local_count, param_places)
        };
        if !Type::unify_assignable(&body.ty, &parsed.signature_return) {
            let name = self.binding(binding)?.name.clone();
            return Err(self.error(
                origin,
                format!(
                    "def {name}: body type {:?} not assignable to {:?}",
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
