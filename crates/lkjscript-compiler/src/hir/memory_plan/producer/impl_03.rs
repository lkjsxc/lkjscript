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
            ExprKind::SetLocal { value, .. } | ExprKind::ProductField { value, .. } => {
                self.walk_expr(value, Some(expression_id), 0, MemoryEscape::Local, None)?;
            }
            ExprKind::ProductValue { fields, .. } | ExprKind::EnumValue { fields, .. } => {
                self.walk_sequence(fields, expression_id, MemoryEscape::Local)?;
            }
            ExprKind::WithProductField {
                value, replacement, ..
            } => {
                self.walk_expr(value, Some(expression_id), 0, MemoryEscape::Local, None)?;
                self.walk_expr(replacement, Some(expression_id), 1, MemoryEscape::Local, None)?;
            }
            ExprKind::EnumIsVariant { value, .. }
            | ExprKind::EnumField { value, .. }
            | ExprKind::EnumUnwrap { value, .. } => {
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
    ) -> Result<()> {
        for (index, argument) in arguments.iter().enumerate() {
            self.walk_expr(
                argument,
                Some(parent),
                index_u32(index)?,
                MemoryEscape::Caller,
                None,
            )?;
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
            return Err(Error::msg(
                "HIR memory-plan producer requires dense per-function PlaceIds",
            ));
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
            if let Some((kind, glue)) = obligation_for_type(ty) {
                self.add_obligation(entry, kind, Some(glue))?;
            }
        }
        Ok(entry)
    }
}
