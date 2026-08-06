impl<'a> Producer<'a> {
    fn walk_scopes(
        &mut self,
        expression: &Expr,
        expression_id: MemoryExpressionId,
        escape: MemoryEscape,
    ) -> Result<()> {
        match &expression.kind {
            ExprKind::Let { bindings, body } => {
                for (index, local) in bindings.iter().enumerate() {
                    self.add_local_place(local, expression.origin.raw())?;
                    let binding = matches!(local.value.kind, ExprKind::Borrow { .. })
                        .then_some(local.binding);
                    self.walk_expr(
                        &local.value,
                        Some(expression_id),
                        index_u32(index)?,
                        MemoryEscape::Local,
                        binding,
                    )?;
                }
                self.walk_expr(
                    body,
                    Some(expression_id),
                    index_u32(bindings.len())?,
                    escape,
                    None,
                )?;
            }
            ExprKind::MutableLocal {
                binding,
                place,
                initial,
                body,
                ..
            } => {
                let ty = self.binding_type(*binding)?.clone();
                self.add_place(
                    self.current_function,
                    *binding,
                    place.raw(),
                    &ty,
                    expression.origin.raw(),
                    true,
                )?;
                self.walk_expr(initial, Some(expression_id), 0, MemoryEscape::Local, None)?;
                self.walk_expr(body, Some(expression_id), 1, escape, None)?;
            }
            ExprKind::SetLocal { value, .. } => {
                self.walk_expr(value, Some(expression_id), 0, MemoryEscape::Local, None)?;
            }
            ExprKind::ProductField { value, .. } => {
                self.reject_partial_projection(expression)?;
                self.walk_expr(value, Some(expression_id), 0, MemoryEscape::Local, None)?;
            }
            ExprKind::ProductValue { fields, .. } | ExprKind::EnumValue { fields, .. } => {
                self.walk_sequence(fields, expression_id, MemoryEscape::Local)?;
                self.add_destination(expression, expression_id)?;
            }
            ExprKind::WithProductField {
                value, replacement, ..
            } => {
                self.reject_partial_projection(expression)?;
                self.walk_expr(value, Some(expression_id), 0, MemoryEscape::Local, None)?;
                self.walk_expr(replacement, Some(expression_id), 1, MemoryEscape::Local, None)?;
            }
            ExprKind::EnumIsVariant { value, .. } => {
                self.walk_expr(value, Some(expression_id), 0, MemoryEscape::Local, None)?;
            }
            ExprKind::EnumField { value, .. } | ExprKind::EnumUnwrap { value, .. } => {
                self.reject_partial_projection(expression)?;
                self.walk_expr(value, Some(expression_id), 0, MemoryEscape::Local, None)?;
            }
            _ => return Err(Error::msg("HIR memory scope category mismatch")),
        }
        Ok(())
    }
    fn walk_call_arguments(
        &mut self,
        arguments: &[Expr],
        parent: MemoryExpressionId,
        call: Option<MemoryCallId>,
    ) -> Result<()> {
        for (index, argument) in arguments.iter().enumerate() {
            let expression = self.walk_expr(
                argument,
                Some(parent),
                index_u32(index)?,
                MemoryEscape::Caller,
                None,
            )?;
            if let Some(call) = call {
                self.add_inferred_borrow_scope(call, index, argument, expression, parent)?;
            }
        }
        Ok(())
    }
    fn walk_sequence(
        &mut self,
        expressions: &[Expr],
        parent: MemoryExpressionId,
        final_escape: MemoryEscape,
    ) -> Result<()> {
        for (index, child) in expressions.iter().enumerate() {
            let escape = if index.saturating_add(1) == expressions.len() {
                final_escape
            } else {
                MemoryEscape::Local
            };
            self.walk_expr(child, Some(parent), index_u32(index)?, escape, None)?;
        }
        Ok(())
    }
    fn add_local_place(&mut self, local: &LocalDefinition, source: u32) -> Result<()> {
        let ty = self.binding_type(local.binding)?.clone();
        self.add_place(
            self.current_function,
            local.binding,
            local.place.raw(),
            &ty,
            source,
            !local.static_bytes,
        )
        .map(|_| ())
    }
    fn add_place(
        &mut self,
        function: MemoryFunctionId,
        binding: BindingId,
        place: u32,
        ty: &Type,
        source: u32,
        owns_obligation: bool,
    ) -> Result<MemoryEntryId> {
        if place != self.next_place {
            return Err(Error::msg(format!(
                "HIR memory-plan producer requires dense per-function PlaceIds: expected {}, got {place}",
                self.next_place,
            )));
        }
        self.next_place = self
            .next_place
            .checked_add(1)
            .ok_or_else(|| Error::msg("HIR memory-plan place identity overflow"))?;
        let entry = self.add_entry(
            MemorySubject::Place {
                function,
                place,
                binding: binding.raw(),
            },
            ty,
            0,
            MemoryEscape::Local,
            MemoryOrigin {
                source,
                expression: None,
            },
        )?;
        if owns_obligation {
            let type_fact = self.entries.get(entry.index().unwrap_or(usize::MAX))
                .ok_or_else(|| Error::msg("whole-place entry is missing"))?.type_fact;
            let fact = self.type_planner.fact(type_fact)?.clone();
            if fact.mode != MemoryAggregateMode::Copy {
                if let Some(glue) = fact.drop_glue {
                    let kind = match ty {
                        Type::Resource(kind) => MemoryObligationKind::DropResource(*kind),
                        _ => MemoryObligationKind::DropWholeValue,
                    };
                    self.add_obligation(entry, kind, Some(glue), fact.drop_path)?;
                }
            }
        }
        Ok(entry)
    }
}
