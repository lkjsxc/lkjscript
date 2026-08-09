use super::*;

impl<'a> Producer<'a> {
    pub(super) fn walk_leaf(
        &mut self,
        expression: &Expr,
        expression_id: MemoryExpressionId,
        expression_entry: MemoryEntryId,
        escape: MemoryEscape,
        loan_binding: Option<BindingId>,
    ) -> Result<()> {
        if let Some(value) = constant_value(&expression.kind) {
            return self.add_constant(
                expression_id,
                expression_entry,
                &expression.ty,
                expression,
                value,
                escape,
            );
        }
        match &expression.kind {
            ExprKind::Load(reference) => {
                self.reject_affine_load(expression, reference.binding)?;
                self.add_use(expression_id, reference.binding, MemoryUseKind::Load)?;
            }
            ExprKind::Move { binding, .. } => {
                self.add_use(expression_id, binding.binding, MemoryUseKind::Move)?;
            }
            ExprKind::BorrowBytes {
                place,
                loan,
                binding,
            } => {
                self.add_use(expression_id, binding.binding, MemoryUseKind::BorrowSource)?;
                let entry = self.add_entry(
                    MemorySubject::Loan {
                        function: self.current_function,
                        place: place.raw(),
                        loan: loan.raw(),
                        expression: expression_id,
                    },
                    &expression.ty,
                    expression.effects.bits(),
                    MemoryEscape::Local,
                    MemoryOrigin {
                        source: crate::memory_plan::source_origin(expression.origin),
                        expression: Some(expression_id),
                    },
                )?;
                self.charge_loans(1)?;
                self.loans.push(MemoryLoanPlan {
                    function: self.current_function,
                    place: place.raw(),
                    loan: loan.raw(),
                    expression: expression_id,
                    binding: loan_binding.map(BindingId::raw),
                    kind: MemoryBorrowKind::Shared,
                    semantic_uses: 0,
                    end_after: expression_id,
                    entry,
                });
                self.add_obligation(entry, MemoryObligationKind::EndBorrow, None, None)?;
            }
            ExprKind::Borrow {
                place,
                loan,
                kind,
                binding,
            } => {
                self.add_use(expression_id, binding.binding, MemoryUseKind::BorrowSource)?;
                let entry = self.add_entry(
                    MemorySubject::Loan {
                        function: self.current_function,
                        place: place.raw(),
                        loan: loan.raw(),
                        expression: expression_id,
                    },
                    &expression.ty,
                    expression.effects.bits(),
                    MemoryEscape::Local,
                    MemoryOrigin {
                        source: crate::memory_plan::source_origin(expression.origin),
                        expression: Some(expression_id),
                    },
                )?;
                self.charge_loans(1)?;
                self.loans.push(MemoryLoanPlan {
                    function: self.current_function,
                    place: place.raw(),
                    loan: loan.raw(),
                    expression: expression_id,
                    binding: loan_binding.map(BindingId::raw),
                    kind: borrow_kind(*kind),
                    semantic_uses: 0,
                    end_after: expression_id,
                    entry,
                });
                self.add_obligation(entry, MemoryObligationKind::EndBorrow, None, None)?;
            }
            _ => return Err(Error::msg("HIR memory leaf category mismatch")),
        }
        Ok(())
    }
    pub(super) fn walk_control(
        &mut self,
        expression: &Expr,
        expression_id: MemoryExpressionId,
        expression_entry: MemoryEntryId,
        escape: MemoryEscape,
    ) -> Result<()> {
        match &expression.kind {
            ExprKind::Call { callee, args, .. } => {
                self.add_use(
                    expression_id,
                    callee.binding,
                    match callee.storage {
                        BindingStorage::Function => MemoryUseKind::DirectCallTarget,
                        BindingStorage::Local(_) => MemoryUseKind::IndirectCallTarget,
                    },
                )?;
                let call = self.add_call(
                    expression_id,
                    expression_entry,
                    callee,
                    args,
                    expression,
                    escape,
                )?;
                self.walk_call_arguments(args, expression_id, Some(call))?;
            }
            ExprKind::Operation {
                operation,
                args,
                resolved_signature,
                ..
            } => {
                self.add_operation_call(
                    expression_id,
                    expression_entry,
                    *operation,
                    resolved_signature,
                    expression,
                    escape,
                )?;
                self.walk_call_arguments(args, expression_id, None)?;
            }
            ExprKind::F64FromI64Exact(value)
            | ExprKind::F64FromI64Rounded(value)
            | ExprKind::I64FromF64Exact(value)
            | ExprKind::I64FromF64Trunc(value) => {
                self.walk_expr(value, Some(expression_id), 0, MemoryEscape::Local, None)?;
            }
            ExprKind::Do(expressions) => self.walk_sequence(expressions, expression_id, escape)?,
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.walk_expr(condition, Some(expression_id), 0, MemoryEscape::Local, None)?;
                self.walk_expr(then_branch, Some(expression_id), 1, escape, None)?;
                self.walk_expr(else_branch, Some(expression_id), 2, escape, None)?;
            }
            ExprKind::While {
                condition, body, ..
            } => {
                self.walk_expr(condition, Some(expression_id), 0, MemoryEscape::Local, None)?;
                for (index, child) in body.iter().enumerate() {
                    self.walk_expr(
                        child,
                        Some(expression_id),
                        index_u64(index.saturating_add(1))?,
                        MemoryEscape::Local,
                        None,
                    )?;
                }
            }
            ExprKind::Loop { body, .. } => {
                self.walk_sequence(body, expression_id, MemoryEscape::Local)?;
            }
            ExprKind::Return { value } => {
                self.walk_expr(value, Some(expression_id), 0, MemoryEscape::Returned, None)?;
            }
            ExprKind::Break { value, .. } => {
                self.walk_expr(value, Some(expression_id), 0, MemoryEscape::Local, None)?;
            }
            ExprKind::Trap { value } => {
                self.walk_expr(value, Some(expression_id), 0, MemoryEscape::Runtime, None)?;
            }
            ExprKind::Exit { code } => {
                self.walk_expr(code, Some(expression_id), 0, MemoryEscape::Runtime, None)?;
            }
            ExprKind::Continue { .. } | ExprKind::MatchUnreachable { .. } => {}
            _ => return Err(Error::msg("HIR memory control category mismatch")),
        }
        Ok(())
    }
}

impl<'a> Producer<'a> {
    pub(super) fn walk_scopes(
        &mut self,
        expression: &Expr,
        expression_id: MemoryExpressionId,
        escape: MemoryEscape,
    ) -> Result<()> {
        match &expression.kind {
            ExprKind::Let { bindings, body } => {
                for (index, local) in bindings.iter().enumerate() {
                    let binding = matches!(local.value.kind, ExprKind::Borrow { .. })
                        .then_some(local.binding);
                    self.walk_expr(
                        &local.value,
                        Some(expression_id),
                        index_u64(index)?,
                        MemoryEscape::Local,
                        binding,
                    )?;
                    self.add_local_place(
                        local,
                        crate::memory_plan::source_origin(expression.origin),
                    )?;
                }
                self.walk_expr(
                    body,
                    Some(expression_id),
                    index_u64(bindings.len())?,
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
                    crate::memory_plan::source_origin(expression.origin),
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
                self.walk_expr(
                    replacement,
                    Some(expression_id),
                    1,
                    MemoryEscape::Local,
                    None,
                )?;
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
    pub(super) fn walk_call_arguments(
        &mut self,
        arguments: &[Expr],
        parent: MemoryExpressionId,
        call: Option<MemoryCallId>,
    ) -> Result<()> {
        for (index, argument) in arguments.iter().enumerate() {
            let expression = self.walk_expr(
                argument,
                Some(parent),
                index_u64(index)?,
                MemoryEscape::Caller,
                None,
            )?;
            if let Some(call) = call {
                self.add_inferred_borrow_scope(call, index, argument, expression, parent)?;
            }
        }
        Ok(())
    }
    pub(super) fn walk_sequence(
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
            self.walk_expr(child, Some(parent), index_u64(index)?, escape, None)?;
        }
        Ok(())
    }
    pub(super) fn add_local_place(
        &mut self,
        local: &LocalDefinition,
        source: MemorySourceOrigin,
    ) -> Result<()> {
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
    pub(super) fn add_place(
        &mut self,
        function: MemoryFunctionId,
        binding: BindingId,
        place: u64,
        ty: &Type,
        source: MemorySourceOrigin,
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
            let type_fact = entry
                .index()
                .and_then(|index| self.entries.get(index))
                .ok_or_else(|| Error::msg("whole-place entry is missing"))?
                .type_fact;
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
