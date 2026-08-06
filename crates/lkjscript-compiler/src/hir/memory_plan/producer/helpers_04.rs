#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DropFlow {
    initialized: bool,
    conditional: bool,
}

impl Producer<'_> {
    fn finish_drop_classes(&mut self) -> Result<()> {
        let classes = self
            .obligations
            .iter()
            .map(|obligation| self.drop_class(obligation))
            .collect::<Result<Vec<_>>>()?;
        for (obligation, class) in self.obligations.iter_mut().zip(classes) {
            obligation.drop_class = class;
        }
        Ok(())
    }

    fn drop_class(&self, obligation: &MemoryObligation) -> Result<Option<MemoryDropClass>> {
        if matches!(obligation.kind, MemoryObligationKind::EndBorrow) {
            return Ok(None);
        }
        let entry = obligation
            .entry
            .index()
            .and_then(|index| self.entries.get(index))
            .ok_or_else(|| Error::msg("HIR drop obligation entry is missing"))?;
        let MemorySubject::Place { binding, .. } = entry.subject else {
            return Err(Error::msg("HIR drop obligation does not name a whole place"));
        };
        let body = function_body(self.program, obligation.function)?;
        let flow = producer_drop_flow(
            body,
            BindingId::new(binding),
            DropFlow {
                initialized: true,
                conditional: false,
            },
        )?;
        Ok(Some(if flow.conditional {
            MemoryDropClass::Conditional
        } else if flow.initialized {
            MemoryDropClass::Static
        } else {
            MemoryDropClass::Dead
        }))
    }
}

fn function_body(program: &hir::Program, function: MemoryFunctionId) -> Result<&Expr> {
    let index = function
        .index()
        .ok_or_else(|| Error::msg("HIR drop class function identity exceeds usize"))?;
    if let Some(function) = program.functions.get(index) {
        Ok(&function.body)
    } else if index == program.functions.len() {
        Ok(&program.main.body)
    } else {
        Err(Error::msg("HIR drop class function identity is missing"))
    }
}

fn producer_drop_flow(
    expression: &Expr,
    binding: BindingId,
    mut flow: DropFlow,
) -> Result<DropFlow> {
    if directly_consumes(expression, binding) {
        if !flow.initialized {
            return Err(open_drop_error());
        }
        flow.initialized = false;
        return Ok(flow);
    }
    match &expression.kind {
        ExprKind::SetLocal { target, value, .. } if *target == binding => {
            flow = producer_drop_flow(value, binding, flow)?;
            if flow.initialized {
                return Err(open_drop_error());
            }
            flow.initialized = true;
            Ok(flow)
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let entry = producer_drop_flow(condition, binding, flow)?;
            let left = producer_drop_flow(then_branch, binding, entry)?;
            let right = producer_drop_flow(else_branch, binding, entry)?;
            match (then_branch.ty == Type::Never, else_branch.ty == Type::Never) {
                (true, false) => Ok(right),
                (false, true) => Ok(left),
                (true, true) => Ok(entry),
                (false, false) if left.initialized == right.initialized => Ok(DropFlow {
                    initialized: left.initialized,
                    conditional: left.conditional || right.conditional,
                }),
                (false, false) => Ok(DropFlow {
                    initialized: false,
                    conditional: true,
                }),
            }
        }
        ExprKind::While { .. } | ExprKind::Loop { .. } => {
            let after = producer_drop_children(expression, binding, flow)?;
            if after == flow {
                Ok(flow)
            } else {
                Err(open_drop_error())
            }
        }
        _ => producer_drop_children(expression, binding, flow),
    }
}

fn producer_drop_children(
    expression: &Expr,
    binding: BindingId,
    mut flow: DropFlow,
) -> Result<DropFlow> {
    for child in children(expression) {
        flow = producer_drop_flow(child, binding, flow)?;
    }
    Ok(flow)
}

fn directly_consumes(expression: &Expr, binding: BindingId) -> bool {
    match &expression.kind {
        ExprKind::Move { binding: moved, .. } => moved.binding == binding,
        ExprKind::Operation {
            operation, args, ..
        } if consuming_operation(*operation) => args
            .iter()
            .any(|argument| expression_uses_binding(argument, binding)),
        _ => false,
    }
}

fn open_drop_error() -> Error {
    Error::msg("HIR memory plan rejects an open or multiply consumed whole place")
}

impl Producer<'_> {
    fn add_inferred_borrow_scope(
        &mut self,
        call: MemoryCallId,
        argument_index: usize,
        argument: &Expr,
        expression: MemoryExpressionId,
        end_after: MemoryExpressionId,
    ) -> Result<()> {
        let call_index = call.index().ok_or_else(|| Error::msg("call identity exceeds usize"))?;
        let mode = *self.calls.get(call_index).and_then(|item| item.parameters.get(argument_index))
            .ok_or_else(|| Error::msg("call borrow parameter is missing"))?;
        let kind = match mode {
            MemoryParameterMode::BorrowShared => MemoryBorrowKind::Shared,
            MemoryParameterMode::BorrowExclusive => MemoryBorrowKind::Exclusive,
            _ => return Ok(()),
        };
        let ExprKind::Load(reference) = argument.kind else { return Ok(()); };
        let type_id = self.type_planner.intern(&argument.ty)?;
        let fact = self.type_planner.fact(type_id)?;
        if kind == MemoryBorrowKind::Shared
            && (fact.closure.class != MemoryClosureClass::Deterministic
                || fact.mode != MemoryAggregateMode::ImmutableValue) {
            return Ok(());
        }
        let place = *self
            .places_by_binding
            .get(&(self.current_function, reference.binding.raw()))
            .ok_or_else(|| Error::msg("inferred direct-call borrow lost source place"))?;
        let id = MemoryBorrowScopeId::new(u64::try_from(self.borrow_scopes.len())
            .map_err(|_| Error::msg("HIR memory-plan borrow scope identity exceeds u64"))?);
        let entry_id = *self
            .expression_entries
            .get(&expression)
            .ok_or_else(|| Error::msg("inferred direct-call borrow lost argument entry"))?;
        let entry = entry_id
            .index()
            .and_then(|index| self.entries.get_mut(index))
            .filter(|entry| entry.id == entry_id)
            .ok_or_else(|| Error::msg("inferred direct-call borrow argument entry is stale"))?;
        entry.borrow_scope = Some(id);
        entry.copy_share = if kind == MemoryBorrowKind::Shared {
            MemoryCopySharePlan::BorrowShared
        } else { MemoryCopySharePlan::BorrowExclusive };
        self.calls[call_index].borrow_scopes[argument_index] = Some(id);
        self.borrow_scopes.push(MemoryBorrowScopePlan { id, function: self.current_function,
            call, argument_index: index_u32(argument_index)?, source_expression: expression,
            binding: reference.binding.raw(), place, kind, semantic_uses: 1, end_after });
        Ok(())
    }
}
