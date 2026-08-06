impl<'a> Producer<'a> {
    fn walk_leaf(
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
                        source: expression.origin.raw(),
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
                        source: expression.origin.raw(),
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
    fn walk_control(
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
