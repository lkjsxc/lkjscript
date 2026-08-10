use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn resolve_var(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let [name_form, type_form, initial_ast, body_ast] = args else {
            return Err(
                self.error("var expects name/, type/, initial expression, and body expression")
            );
        };
        let name = declared_name_form(name_form, "var").map_err(|message| self.error(message))?;
        let AstExpr::Call {
            name: type_tag,
            args: type_args,
        } = type_form
        else {
            return Err(self.error("var expects type/…/type second"));
        };
        if type_tag != "type" {
            return Err(self.error("var expects type/…/type second"));
        }
        let declared_type = parse_type_form(type_args)
            .map_err(|message| self.error(format!("var {name}: {message}")))?;
        if let Some(reason) = crate::ownership::mutable_local_storage_restriction(&declared_type) {
            return Err(self.error(format!("var {name}: {reason}")));
        }
        self.analyzer
            .validate_product_type(&declared_type)
            .map_err(|message| self.error(format!("var {name}: {message}")))?;
        let mut parameters = HashSet::new();
        collect_type_params(&declared_type, &mut parameters);
        if let Some(parameter) = parameters
            .into_iter()
            .find(|parameter| !self.type_variables.contains(*parameter))
        {
            return Err(self.error(format!(
                "var {name}: type parameter {parameter} is not declared by forall"
            )));
        }

        // The initializer is deliberately resolved before the new binding exists.
        let initial = self.resolve_expr(initial_ast)?;
        if initial.ty != declared_type {
            return Err(self.error(format!(
                "var {name}: initializer type {} does not exactly equal {declared_type}",
                initial.ty
            )));
        }

        let saved_slot = self.next_slot;
        let slot = self.next_slot;
        self.next_slot = self
            .next_slot
            .checked_add(1)
            .ok_or_else(|| self.error("local slot count overflow"))?;
        self.max_slots = self.max_slots.max(self.next_slot);
        let binding = self.analyzer.add_binding(
            name.clone(),
            BindingKind::MutableLocal,
            declared_type,
            Origin::Source(self.origin),
        )?;
        self.local_slots.insert(binding, slot);
        let place = self.allocate_place(binding)?;
        self.scopes.push(HashMap::from([(name, binding)]));
        let body = self.resolve_expr(body_ast)?;
        let _removed_scope = self.scopes.pop();
        self.next_slot = saved_slot;
        let ty = body.ty.clone();
        Ok(self.expression(
            ty,
            ExprKind::MutableLocal {
                binding,
                place,
                slot,
                initial: Box::new(initial),
                body: Box::new(body),
            },
        ))
    }

    pub(in crate::analyze) fn resolve_set(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let (target_name, value_ast) = match args {
            [target, value] => (
                symbolic_name(target).map_err(|message| self.error(message))?,
                value,
            ),
            _ => return Err(self.error("set needs name and value")),
        };
        let target = self.lookup_lexical(&target_name).ok_or_else(|| {
            if self.analyzer.globals.contains_key(&target_name)
                || Operation::from_name(&target_name).is_some()
            {
                self.error(format!(
                    "set target {target_name} is not a function-local mutable var"
                ))
            } else {
                self.error(format!("unknown set target {target_name}"))
            }
        })?;
        let (kind, target_type) = {
            let binding = self.analyzer.binding(target)?;
            (binding.kind.clone(), binding.ty.clone())
        };
        if kind != BindingKind::MutableLocal {
            return Err(self.error(format!(
                "set target {target_name} is not a function-local mutable var"
            )));
        }
        let slot =
            self.local_slots.get(&target).copied().ok_or_else(|| {
                self.error(format!("set target {target_name} has no HIR local slot"))
            })?;
        let value = self.resolve_expr(value_ast)?;
        if value.ty == Type::Never {
            return Err(self.error(format!(
                "set target {target_name}: divergent value cannot fill a storage slot"
            )));
        }
        if value.ty != target_type {
            return Err(self.error(format!(
                "set target {target_name}: value type {} does not exactly equal {target_type}",
                value.ty
            )));
        }
        Ok(self.expression(
            Type::Unit,
            ExprKind::SetLocal {
                target,
                slot,
                value: Box::new(value),
            },
        ))
    }
}
