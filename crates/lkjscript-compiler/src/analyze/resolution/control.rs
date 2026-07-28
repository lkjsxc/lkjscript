use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn resolve_do(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let expressions = self.resolve_control_body(args)?;
        let ty = expressions
            .last()
            .map_or(Type::Unit, |expression| expression.ty.clone());
        Ok(self.expression(ty, ExprKind::Do(expressions)))
    }

    pub(in crate::analyze) fn resolve_if(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let [condition, then_branch, else_branch] = args else {
            return Err(self.error("if expects condition, then, and else"));
        };
        let condition = self.resolve_expr(condition)?;
        if condition.ty != Type::Bool {
            return Err(self.error("if condition must be Bool"));
        }
        let then_branch = self.resolve_expr(then_branch)?;
        let else_branch = self.resolve_expr(else_branch)?;
        let ty = Type::join_control(&then_branch.ty, &else_branch.ty).ok_or_else(|| {
            self.error(format!(
                "if reachable branches must have the same type: {} vs {}",
                then_branch.ty, else_branch.ty
            ))
        })?;
        Ok(self.expression(
            ty,
            ExprKind::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            },
        ))
    }

    pub(in crate::analyze) fn resolve_while(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let Some((condition, body)) = args.split_first() else {
            return Err(self.error("while needs a condition"));
        };
        let condition = self.resolve_expr(condition)?;
        if !Type::unify_assignable(&condition.ty, &Type::Bool) {
            return Err(self.error("while condition must be Bool"));
        }
        let loop_id = LoopId::new(self.next_loop);
        self.next_loop = self
            .next_loop
            .checked_add(1)
            .ok_or_else(|| self.error("loop identity space exhausted"))?;
        self.loops.push(LoopContext {
            id: loop_id,
            result_type: Type::Unit,
            is_while: true,
        });
        let resolved_body = self.resolve_control_body(body)?;
        let _target = self.loops.pop();
        Ok(self.expression(
            Type::Unit,
            ExprKind::While {
                loop_id,
                condition: Box::new(condition),
                body: resolved_body,
            },
        ))
    }

    pub(in crate::analyze) fn resolve_let(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let Some((body, bindings)) = args.split_last() else {
            return Err(self.error("let needs body"));
        };
        self.scopes.push(HashMap::new());
        let saved_slot = self.next_slot;
        let mut resolved_bindings = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let (name, value) = match binding {
                AstExpr::Call { name, args } if name == "bind" => match args.as_slice() {
                    [name, value] => (
                        symbolic_name(name).map_err(|message| self.error(message))?,
                        value,
                    ),
                    _ => return Err(self.error("bind needs name and value")),
                },
                _ => return Err(self.error("let bindings must be bind/…/bind")),
            };
            let value = self.resolve_expr(value)?;
            if value.ty == Type::Never {
                return Err(self.error("divergent expression cannot initialize a let storage slot"));
            }
            if contains_resource_type(&value.ty) && !matches!(value.ty, Type::Resource(_)) {
                return Err(self.error(
                    "resource-bearing aggregates cannot be stored in this typed resource slice",
                ));
            }
            let slot = u8::try_from(self.next_slot)
                .map_err(|_| self.error("let needs more than 255 bytecode local slots"))?;
            self.next_slot = self
                .next_slot
                .checked_add(1)
                .ok_or_else(|| self.error("local slot count overflow"))?;
            self.max_slots = self.max_slots.max(self.next_slot);
            let static_bytes = matches!(&value.kind, ExprKind::LitBytes(_))
                || matches!(
                    &value.kind,
                    ExprKind::Load(reference)
                        if self
                            .analyzer
                            .binding(reference.binding)?
                            .kind
                            == BindingKind::StaticBytesLocal
                );
            let binding_id = self.analyzer.add_binding(
                name.clone(),
                if static_bytes {
                    BindingKind::StaticBytesLocal
                } else {
                    BindingKind::ImmutableLocal
                },
                value.ty.clone(),
                Origin::Source(self.origin),
            )?;
            if self
                .scopes
                .last()
                .is_some_and(|scope| scope.contains_key(&name))
            {
                return Err(self.error(format!("duplicate let binding {name}")));
            }
            let Some(scope) = self.scopes.last_mut() else {
                return Err(self.error("missing lexical scope while resolving let"));
            };
            scope.insert(name, binding_id);
            self.local_slots.insert(binding_id, slot);
            let place = self.allocate_place(binding_id)?;
            resolved_bindings.push(LocalDefinition {
                binding: binding_id,
                place,
                static_bytes,
                slot,
                value,
            });
        }
        let body = self.resolve_expr(body)?;
        self.next_slot = saved_slot;
        let _removed_scope = self.scopes.pop();
        let ty = body.ty.clone();
        Ok(self.expression(
            ty,
            ExprKind::Let {
                bindings: resolved_bindings,
                body: Box::new(body),
            },
        ))
    }
}
