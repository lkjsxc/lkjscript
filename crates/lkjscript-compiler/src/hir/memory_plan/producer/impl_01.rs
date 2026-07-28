impl<'a> Producer<'a> {
    fn build_main(&mut self, id: MemoryFunctionId) -> Result<()> {
        self.current_function = id;
        self.next_place = 0;
        let signature = self.signature(id)?.clone();
        let mut parameter_entries = Vec::with_capacity(self.program.main.params.len());
        for (index, (((binding, place), ty), mode)) in self
            .program
            .main
            .params
            .iter()
            .copied()
            .zip(self.program.main.param_places.iter().copied())
            .zip(self.program.main.param_types.iter())
            .zip(signature.parameters.iter().copied())
            .enumerate()
        {
            let entry = self.add_entry(
                MemorySubject::Parameter {
                    function: id,
                    index: u32::try_from(index)
                        .map_err(|_| Error::msg("HIR main memory parameter index exceeds u32"))?,
                    binding: binding.raw(),
                    place: place.raw(),
                },
                ty,
                0,
                MemoryEscape::Caller,
                MemoryOrigin {
                    source: self.program.main.origin.raw(),
                    expression: None,
                },
            )?;
            parameter_entries.push(entry);
            self.add_place(
                id,
                binding,
                place.raw(),
                ty,
                self.program.main.origin.raw(),
                mode == MemoryParameterMode::Consume,
            )?;
        }
        let result_entry = self.add_entry(
            MemorySubject::Result { function: id },
            &self.program.main.return_type,
            self.program.main.body.effects.bits(),
            MemoryEscape::Returned,
            MemoryOrigin {
                source: self.program.main.origin.raw(),
                expression: None,
            },
        )?;
        let body = self.walk_expr(
            &self.program.main.body,
            None,
            0,
            MemoryEscape::Returned,
            None,
        )?;
        self.functions.push(FunctionMemoryPlan {
            id,
            name: "main".into(),
            binding: None,
            source: self.program.main.origin.raw(),
            signature,
            parameter_entries,
            result_entry,
            body,
        });
        Ok(())
    }
    #[allow(clippy::too_many_lines)]
    fn walk_expr(
        &mut self,
        expression: &Expr,
        parent: Option<MemoryExpressionId>,
        child_index: u32,
        escape: MemoryEscape,
        loan_binding: Option<BindingId>,
    ) -> Result<MemoryExpressionId> {
        let expression_id = self.next_expression()?;
        self.expression_parents.insert(expression_id, parent);
        let expression_entry = self.add_entry(
            MemorySubject::Expression {
                expression: expression_id,
                parent,
                child_index,
                kind: expression_kind(&expression.kind),
            },
            &expression.ty,
            expression.effects.bits(),
            escape,
            MemoryOrigin {
                source: expression.origin.raw(),
                expression: Some(expression_id),
            },
        )?;
        match &expression.kind {
            ExprKind::LitI64(_)
            | ExprKind::LitF64(_)
            | ExprKind::LitBool(_)
            | ExprKind::LitUnit
            | ExprKind::EmptyList
            | ExprKind::LitStr(_)
            | ExprKind::LitBytes(_)
            | ExprKind::QuoteSymbol(_)
            | ExprKind::Load(_)
            | ExprKind::Move { .. }
            | ExprKind::Borrow { .. }
            | ExprKind::BorrowBytes { .. } => self.walk_leaf(
                expression,
                expression_id,
                expression_entry,
                escape,
                loan_binding,
            )?,
            ExprKind::Call { .. }
            | ExprKind::Operation { .. }
            | ExprKind::F64FromI64Exact(_)
            | ExprKind::F64FromI64Rounded(_)
            | ExprKind::I64FromF64Exact(_)
            | ExprKind::I64FromF64Trunc(_)
            | ExprKind::Do(_)
            | ExprKind::If { .. }
            | ExprKind::While { .. }
            | ExprKind::Loop { .. }
            | ExprKind::Return { .. }
            | ExprKind::Break { .. }
            | ExprKind::Continue { .. }
            | ExprKind::Trap { .. }
            | ExprKind::Exit { .. }
            | ExprKind::MatchUnreachable { .. } => self.walk_control(
                expression,
                expression_id,
                expression_entry,
                escape,
            )?,
            ExprKind::Let { .. }
            | ExprKind::MutableLocal { .. }
            | ExprKind::SetLocal { .. }
            | ExprKind::ProductValue { .. }
            | ExprKind::ProductField { .. }
            | ExprKind::WithProductField { .. }
            | ExprKind::EnumValue { .. }
            | ExprKind::EnumIsVariant { .. }
            | ExprKind::EnumField { .. }
            | ExprKind::EnumUnwrap { .. } => {
                self.walk_scopes(expression, expression_id, escape)?;
            }
        }
        Ok(expression_id)
    }
}
