mod enum_dispatch;

use crate::ssa::*;

impl FunctionBuilder<'_> {
    fn lower_expr_inner(&mut self, expression: &Expr) -> Result<Option<ValueId>> {
        // Never controls paths; this local branch never emits an SSA value.
        let ty = if expression.ty == Type::Never {
            SsaType::Unit
        } else {
            lower_type(&expression.ty, self.product_ids)?
        };
        let value = match &expression.kind {
            ExprKind::Hole => return Err(Error::msg("complete HIR cannot contain a hole")),
            ExprKind::Match { .. } => {
                return Err(Error::msg(
                    "semantic match reached SSA without canonical HIR derivation",
                ));
            }
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
            ExprKind::LitStr(value) => self.constant(
                SsaType::Str,
                Constant::Str(value.clone()),
                expression.origin,
            )?,
            ExprKind::LitBytes(value) => self.constant(
                SsaType::Bytes,
                Constant::StaticBytes(value.clone()),
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
            ExprKind::BorrowBytes {
                place,
                loan,
                binding,
            } => {
                return self.lower_borrow(
                    *place,
                    *loan,
                    hir::BorrowKind::Shared,
                    *binding,
                    ty,
                    expression,
                );
            }
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
            kind @ (ExprKind::F64FromI64Exact(_)
            | ExprKind::F64FromI64Rounded(_)
            | ExprKind::I64FromF64Exact(_)
            | ExprKind::I64FromF64Trunc(_)) => {
                return self.lower_numeric_conversion(kind, ty, expression.origin);
            }
            ExprKind::Do(expressions) => {
                return self.lower_sequence(expressions, expression.origin);
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => return self.lower_if(condition, then_branch, else_branch, expression),
            ExprKind::While {
                loop_id,
                condition,
                body,
            } => return self.lower_while(*loop_id, condition, body, expression),
            ExprKind::Loop {
                loop_id,
                result_type,
                body,
            } => return self.lower_loop(*loop_id, result_type, body, expression),
            ExprKind::Return { value } => return self.lower_return(value),
            ExprKind::Break { loop_id, value } => return self.lower_break(*loop_id, value),
            ExprKind::Continue { loop_id } => {
                return self.lower_continue(*loop_id, expression.origin);
            }
            ExprKind::Trap { value } => return self.lower_trap(value),
            ExprKind::Exit { code } => return self.lower_exit(code),
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
            kind @ (ExprKind::EnumValue { .. }
            | ExprKind::EnumIsVariant { .. }
            | ExprKind::EnumField { .. }
            | ExprKind::EnumUnwrap { .. }) => {
                return self.lower_enum_expression(kind, ty, expression.origin);
            }
            ExprKind::MatchUnreachable { plan } => {
                return self.lower_match_unreachable(*plan, expression.origin);
            }
        };
        Ok(Some(value))
    }
}

include!("expressions/entry.rs");
include!("expressions/branches.rs");
include!("expressions/branch_merge.rs");
include!("expressions/branch_merge_conditionals.rs");
include!("expressions/control.rs");
include!("expressions/short_circuit.rs");
include!("expressions/enums.rs");
include!("expressions/enum_unwrap.rs");
include!("expressions/structural_owner.rs");
include!("expressions/structural_owner_unwrap.rs");
include!("expressions/structural_copy_unwrap.rs");
include!("expressions/structural_owner_cleanup.rs");
