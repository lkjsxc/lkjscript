use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn resolve_return(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let [value] = args else {
            return Err(self.error("return expects exactly one value"));
        };
        let value = self.resolve_expr(value)?;
        if value.ty == Type::Never {
            return Err(self.error("return value is already divergent"));
        }
        if value.ty != self.return_type {
            return Err(self.error(format!(
                "return value type {} does not exactly equal {}",
                value.ty, self.return_type
            )));
        }
        Ok(self.expression(
            Type::Never,
            ExprKind::Return {
                value: Box::new(value),
            },
        ))
    }

    pub(in crate::analyze) fn resolve_break(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let target = self
            .loops
            .last()
            .cloned()
            .ok_or_else(|| self.error("break is only valid inside a loop"))?;
        let [value] = args else {
            return Err(self.error("break expects exactly one value"));
        };
        let value = self.resolve_expr(value)?;
        if value.ty == Type::Never {
            return Err(self.error("break value is already divergent"));
        }
        if value.ty != target.result_type {
            return Err(self.error(format!(
                "break value type {} does not exactly equal loop result {}",
                value.ty, target.result_type
            )));
        }
        if target.is_while && value.ty != Type::Unit {
            return Err(self.error("while break must carry Unit"));
        }
        Ok(self.expression(
            Type::Never,
            ExprKind::Break {
                loop_id: target.id,
                value: Box::new(value),
            },
        ))
    }

    pub(in crate::analyze) fn resolve_continue(&mut self, args: &[AstExpr]) -> Result<Expr> {
        if !args.is_empty() {
            return Err(self.error("continue expects no values"));
        }
        let target = self
            .loops
            .last()
            .ok_or_else(|| self.error("continue is only valid inside a loop"))?;
        Ok(self.expression(Type::Never, ExprKind::Continue { loop_id: target.id }))
    }

    pub(in crate::analyze) fn resolve_trap(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let [value] = args else {
            return Err(self.error("trap expects exactly one Str value"));
        };
        let value = self.resolve_expr(value)?;
        if value.ty != Type::Str {
            return Err(self.error("trap value must be Str"));
        }
        Ok(self.expression(
            Type::Never,
            ExprKind::Trap {
                value: Box::new(value),
            },
        ))
    }

    pub(in crate::analyze) fn resolve_exit(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let [code] = args else {
            return Err(self.error("exit expects exactly one I64 code"));
        };
        let code = self.resolve_expr(code)?;
        if code.ty != Type::I64 {
            return Err(self.error("exit code must be I64"));
        }
        Ok(self.expression(
            Type::Never,
            ExprKind::Exit {
                code: Box::new(code),
            },
        ))
    }

    pub(in crate::analyze) fn resolve_loop(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let Some((type_form, body)) = args.split_first() else {
            return Err(self.error("loop expects type/ result and a body"));
        };
        let AstExpr::Call { name, args: atoms } = type_form else {
            return Err(self.error("loop expects type/ result first"));
        };
        if name != "type" {
            return Err(self.error("loop expects type/ result first"));
        }
        let result_type = parse_type_form(atoms)
            .map_err(|message| self.error(format!("loop result: {message}")))?;
        if result_type.contains_never() {
            return Err(self.error("Never is not a loop exit payload type"));
        }
        let loop_id = LoopId::new(self.next_loop);
        self.next_loop = self
            .next_loop
            .checked_add(1)
            .ok_or_else(|| self.error("loop identity space exhausted"))?;
        self.loops.push(LoopContext {
            id: loop_id,
            result_type: result_type.clone(),
            is_while: false,
        });
        let resolved = self.resolve_control_body(body)?;
        let _target = self.loops.pop();
        Ok(self.expression(
            result_type.clone(),
            ExprKind::Loop {
                loop_id,
                result_type,
                body: resolved,
            },
        ))
    }

    pub(super) fn resolve_control_body(&mut self, body: &[AstExpr]) -> Result<Vec<Expr>> {
        let mut output = Vec::with_capacity(body.len());
        for item in body {
            if output
                .last()
                .is_some_and(|expr: &Expr| expr.ty == Type::Never)
            {
                return Err(self.error("unreachable expression after control terminator"));
            }
            output.push(self.resolve_expr(item)?);
        }
        Ok(output)
    }
}
