use crate::analyze::*;

impl Resolver<'_> {
    pub(super) fn enum_projection(
        &self,
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        field: &MatchFieldPattern,
        value: Expr,
    ) -> Result<Expr> {
        let definition = self
            .analyzer
            .enums
            .iter()
            .find(|item| item.id == enum_id)
            .ok_or_else(|| self.error("match lowering lost EnumId"))?;
        let selected = definition
            .variants
            .iter()
            .find(|item| item.id == variant)
            .ok_or_else(|| self.error("match lowering lost VariantId"))?;
        let declared = selected
            .fields
            .iter()
            .find(|item| item.name == field.name)
            .ok_or_else(|| self.error("match lowering lost variant field"))?;
        Ok(self.expression(
            field.projection.ty.clone(),
            ExprKind::EnumField {
                enum_id,
                variant,
                field: declared.id,
                layout,
                value: Box::new(value),
            },
        ))
    }

    pub(super) fn match_equal(&self, value: Expr, ty: Type, literal: ExprKind) -> Result<Expr> {
        let operation = Operation::EqualValue;
        let binding = self
            .analyzer
            .operations
            .get(&operation)
            .copied()
            .ok_or_else(|| self.error("equal-value operation is unavailable for match"))?;
        let literal = self.expression(ty.clone(), literal);
        Ok(self.expression(
            Type::Bool,
            ExprKind::Operation {
                binding,
                operation,
                resolved_signature: Type::Fn {
                    params: vec![ty, value.ty.clone()],
                    ret: Box::new(Type::Bool),
                },
                args: vec![value, literal],
            },
        ))
    }

    pub(super) fn match_and(&self, left: Expr, right: Expr) -> Result<Expr> {
        let operation = Operation::And;
        let binding = self
            .analyzer
            .operations
            .get(&operation)
            .copied()
            .ok_or_else(|| self.error("and operation is unavailable for match"))?;
        Ok(self.expression(
            Type::Bool,
            ExprKind::Operation {
                binding,
                operation,
                resolved_signature: Type::Fn {
                    params: vec![Type::Bool, Type::Bool],
                    ret: Box::new(Type::Bool),
                },
                args: vec![left, right],
            },
        ))
    }

    pub(super) fn match_if(&self, condition: Expr, then_branch: Expr, else_branch: Expr) -> Expr {
        let ty = Type::join_control(&then_branch.ty, &else_branch.ty)
            .unwrap_or_else(|| then_branch.ty.clone());
        self.expression(
            ty,
            ExprKind::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            },
        )
    }

    pub(super) fn local_scope(&self, local: &MatchLocal, value: Expr, body: Expr) -> Expr {
        self.expression(
            body.ty.clone(),
            ExprKind::Let {
                bindings: vec![LocalDefinition {
                    binding: local.binding,
                    place: local.place,
                    static_bytes: false,
                    slot: local.slot,
                    value,
                }],
                body: Box::new(body),
            },
        )
    }

    pub(super) fn match_load(&self, local: &MatchLocal) -> Expr {
        self.expression(
            local.ty.clone(),
            ExprKind::Load(BindingRef {
                binding: local.binding,
                storage: BindingStorage::Local(local.slot),
            }),
        )
    }
}
