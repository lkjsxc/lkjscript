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
        let entry = self
            .entries
            .get(obligation.entry.index().unwrap_or(usize::MAX))
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
