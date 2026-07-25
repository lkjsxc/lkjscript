use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_expr(&mut self, expression: &Expr) -> Result<Option<ValueId>> {
        let ty = lower_type(&expression.ty, self.product_ids)?;
        let value = match &expression.kind {
            ExprKind::LitI64(value) => {
                self.constant(SsaType::I64, Constant::I64(*value), expression.origin)?
            }
            ExprKind::LitF64(value) => {
                self.constant(SsaType::F64, Constant::F64(*value), expression.origin)?
            }
            ExprKind::LitBool(value) => {
                self.constant(SsaType::Bool, Constant::Bool(*value), expression.origin)?
            }
            ExprKind::LitUnit => self.constant(SsaType::Unit, Constant::Unit, expression.origin)?,
            ExprKind::EmptyList => self.constant(ty, Constant::EmptyList, expression.origin)?,
            ExprKind::LitNone => self.constant(ty, Constant::None, expression.origin)?,
            ExprKind::LitStr(value) => self.constant(
                SsaType::Str,
                Constant::Str(value.clone()),
                expression.origin,
            )?,
            ExprKind::QuoteSymbol(value) => self.constant(
                SsaType::Symbol,
                Constant::Symbol(value.clone()),
                expression.origin,
            )?,
            ExprKind::Load(binding) => return self.lower_load(*binding, expression),
            ExprKind::Move { place, binding } => {
                return self.lower_move(*place, *binding, ty, expression);
            }
            ExprKind::Borrow {
                place,
                loan,
                kind,
                binding,
            } => return self.lower_borrow(*place, *loan, *kind, *binding, ty, expression),
            ExprKind::Call {
                callee,
                args,
                instantiation,
            } => {
                return self.lower_call(
                    *callee,
                    args,
                    instantiation.as_ref(),
                    ty,
                    expression.origin,
                );
            }
            ExprKind::Operation {
                operation,
                resolved_signature,
                args,
                ..
            } => {
                return self.lower_operation(*operation, resolved_signature, args, ty, expression);
            }
            ExprKind::Do(expressions) => {
                return self.lower_sequence(expressions, expression.origin);
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => return self.lower_if(condition, then_branch, else_branch, expression),
            ExprKind::While { condition, body } => {
                return self.lower_while(condition, body, expression);
            }
            ExprKind::Let { bindings, body } => return self.lower_let(bindings, body),
            ExprKind::MutableLocal {
                binding,
                place,
                slot,
                initial,
                body,
            } => {
                return self.lower_mutable_local(
                    *binding,
                    *place,
                    *slot,
                    initial,
                    body,
                    expression.origin,
                );
            }
            ExprKind::SetLocal {
                target,
                slot,
                value,
            } => return self.lower_set_local(*target, *slot, value, expression.origin),
            ExprKind::ProductValue { product, fields } => {
                return self.lower_product_value(
                    ProductId::new(product.raw()),
                    fields,
                    expression.origin,
                );
            }
            ExprKind::ProductField {
                product,
                field,
                value,
            } => {
                return self.lower_product_field(
                    ProductId::new(product.raw()),
                    *field,
                    value,
                    ty,
                    expression.origin,
                );
            }
            ExprKind::WithProductField {
                product,
                field,
                value,
                replacement,
            } => {
                return self.lower_product_update(
                    ProductId::new(product.raw()),
                    *field,
                    value,
                    replacement,
                    expression.origin,
                );
            }
        };
        Ok(Some(value))
    }
}
