use super::model::context_only_form;
use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn resolve_call(
        &mut self,
        name: &str,
        args: &[AstExpr],
    ) -> Result<Expr> {
        match name {
            "if" => self.resolve_if(args),
            "match" => self.resolve_match(args),
            "while" => self.resolve_while(args),
            "loop" if self.analyzer.edition2 => self.resolve_loop(args),
            "return" if self.analyzer.edition2 => self.resolve_return(args),
            "break" if self.analyzer.edition2 => self.resolve_break(args),
            "continue" if self.analyzer.edition2 => self.resolve_continue(args),
            "trap" if self.analyzer.edition2 => self.resolve_trap(args),
            "exit" if self.analyzer.edition2 => self.resolve_exit(args),
            "do" => self.resolve_do(args),
            "let" => self.resolve_let(args),
            "var" => self.resolve_var(args),
            "quote" => self.resolve_quote(args),
            "set" => self.resolve_set(args),
            "move" => self.resolve_move(args),
            "borrow" => self.resolve_borrow(args, BorrowKind::Shared),
            "borrow-mut" => self.resolve_borrow(args, BorrowKind::Mutable),
            "empty-list" => self.resolve_empty_list(args),
            "none" => self.resolve_none(args),
            "product-value" => self.resolve_product_value(args),
            "variant-value" => self.resolve_enum_value(args),
            "field" => self.resolve_product_field(args),
            "with-field" => self.resolve_with_product_field(args),
            "bind" => Err(self.error("bind is only valid inside let")),
            name if context_only_form(name) => {
                Err(self.error(format!("{name} is only valid in its declaration context")))
            }
            _ => self.resolve_plain_call(name, args),
        }
    }

    pub(in crate::analyze) fn resolve_plain_call(
        &mut self,
        name: &str,
        args: &[AstExpr],
    ) -> Result<Expr> {
        let callee = self.lookup_call(name).ok_or_else(|| {
            self.diagnostic(AnalysisDiagnostic::UnknownName {
                usage: NameUse::Call,
                name: name.to_string(),
            })
        })?;
        let (kind, callee_type) = {
            let binding = self.analyzer.binding(callee)?;
            (binding.kind.clone(), binding.ty.clone())
        };
        let expected = callable_arity(&callee_type)
            .ok_or_else(|| self.error(format!("{name} is not a function ({callee_type:?})")))?;
        if expected != args.len() {
            return Err(self.diagnostic(AnalysisDiagnostic::CallArity {
                name: name.to_string(),
                expected,
                actual: args.len(),
            }));
        }
        let _arity = u8::try_from(args.len())
            .map_err(|_| self.error(format!("{name}: too many call arguments")))?;
        let mut resolved_args = Vec::with_capacity(args.len());
        for argument in args {
            resolved_args.push(self.resolve_expr(argument)?);
        }

        if let BindingKind::BuiltinOperation(operation) = kind {
            let argument_types: Vec<_> = resolved_args
                .iter()
                .map(|argument| argument.ty.clone())
                .collect();
            if self.analyzer.edition2
                && matches!(
                    operation,
                    Operation::Add
                        | Operation::Subtract
                        | Operation::Multiply
                        | Operation::Divide
                        | Operation::Less
                        | Operation::LessEqual
                        | Operation::Greater
                        | Operation::GreaterEqual
                )
                && argument_types[0] != argument_types[1]
            {
                return Err(self.error(format!(
                    "{}: Edition 2 numeric operands must have one exact type",
                    operation.name()
                )));
            }
            let (resolved_signature, ty) = operation
                .resolve_types(&argument_types)
                .map_err(|message| self.error(message))?;
            let conversion = resolved_args.first().cloned().map(Box::new);
            let kind = match (operation, conversion) {
                (Operation::F64FromI64Exact, Some(value)) => ExprKind::F64FromI64Exact(value),
                (Operation::F64FromI64Rounded, Some(value)) => ExprKind::F64FromI64Rounded(value),
                (Operation::I64FromF64Exact, Some(value)) => ExprKind::I64FromF64Exact(value),
                (Operation::I64FromF64Trunc, Some(value)) => ExprKind::I64FromF64Trunc(value),
                _ => ExprKind::Operation {
                    binding: callee,
                    operation,
                    resolved_signature,
                    args: resolved_args,
                },
            };
            Ok(self.expression(ty, kind))
        } else {
            if resolved_args
                .iter()
                .any(|argument| matches!(argument.ty, Type::RefMut(_)))
            {
                return Err(
                    self.error("RefMut forwarding is unsupported in the initial ownership slice")
                );
            }
            let (ty, instantiation) =
                self.call_result(name, callee, callee_type, &resolved_args)?;
            let callee = self.binding_ref(callee)?;
            Ok(self.expression(
                ty,
                ExprKind::Call {
                    callee,
                    args: resolved_args,
                    instantiation,
                },
            ))
        }
    }
}
