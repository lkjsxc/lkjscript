use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn resolve_call(
        &mut self,
        name: &str,
        args: &[AstExpr],
    ) -> Result<Expr> {
        match name {
            "if" => self.resolve_if(args),
            "while" => self.resolve_while(args),
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
            "fn" | "def" | "main" | "sig" | "params" | "forall" | "bounds" | "bound" | "type"
            | "import" | "name" | "product" | "fields" | "variant" | "variant-field" | "enum"
            | "variants" | "trait" | "impl" | "for" => {
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
            let (resolved_signature, ty) = operation
                .resolve_types(&argument_types)
                .map_err(|message| self.error(message))?;
            Ok(self.expression(
                ty,
                ExprKind::Operation {
                    binding: callee,
                    operation,
                    resolved_signature,
                    args: resolved_args,
                },
            ))
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

    pub(in crate::analyze) fn call_result(
        &self,
        name: &str,
        callee: BindingId,
        callable: Type,
        args: &[Expr],
    ) -> Result<(Type, Option<GenericInstantiation>)> {
        let is_generic = matches!(&callable, Type::Forall { .. });
        let generic_signature_has_ownership = is_generic && contains_ownership_type(&callable);
        let (instantiated, substitutions) = self.instantiate(name, callable, args)?;
        if is_generic
            && (generic_signature_has_ownership
                || substitutions
                    .iter()
                    .any(|substitution| contains_ownership_type(&substitution.ty)))
        {
            return Err(self.error(format!(
                "{name}: ownership/reference generic instantiation is unavailable in the initial ownership slice"
            )));
        }
        let Type::Fn { params, ret } = instantiated else {
            return Err(self.error(format!("{name} is not a function")));
        };
        if params.len() != args.len() {
            return Err(self.diagnostic(AnalysisDiagnostic::CallArity {
                name: name.to_string(),
                expected: params.len(),
                actual: args.len(),
            }));
        }
        for (parameter, argument) in params.iter().zip(args) {
            if !Type::unify_assignable(&argument.ty, parameter) {
                return Err(self.diagnostic(AnalysisDiagnostic::TypeMismatch {
                    context: format!("{name}: arg type"),
                    expected: format!("{parameter:?}"),
                    actual: format!("{:?}", argument.ty),
                }));
            }
        }
        if contains_reference_type(&ret) {
            return Err(self.error(format!(
                "{name}: user-call results cannot be lexical references in the initial ownership slice"
            )));
        }
        let instantiation = if substitutions.is_empty() {
            None
        } else {
            let bounds = self
                .analyzer
                .function_bounds
                .get(&callee)
                .cloned()
                .unwrap_or_default();
            if !bounds.is_empty() {
                for substitution in &substitutions {
                    let mut unresolved = HashSet::new();
                    collect_type_params(&substitution.ty, &mut unresolved);
                    if !unresolved.is_empty() {
                        return Err(self.error(format!(
                            concat!(
                                "{name}: forwarding bounded calls from a generic context is ",
                                "unavailable in the marker-trait slice",
                            ),
                            name = name,
                        )));
                    }
                }
            }
            let mut witnesses = Vec::with_capacity(bounds.len());
            for bound in bounds {
                let ty = substitutions
                    .iter()
                    .find(|substitution| substitution.parameter == bound.parameter)
                    .map(|substitution| substitution.ty.clone())
                    .ok_or_else(|| {
                        self.error(format!(
                            "{name}: missing substitution for bound parameter {}",
                            bound.parameter
                        ))
                    })?;
                witnesses.push(self.solve_trait_bound(name, bound.trait_id, &ty)?);
            }
            Some(GenericInstantiation {
                substitutions,
                witnesses,
            })
        };
        Ok((*ret, instantiation))
    }
}
