use crate::analyze::*;

pub(super) fn lower(
    plan: &MatchPlan,
    scrutinee_value: Expr,
    bodies: Vec<Expr>,
    enums: &[EnumDefinition],
) -> Result<Expr> {
    if plan.origin == Origin::Builtin {
        return Err(Error::msg("ordinary match plan has builtin origin"));
    }
    if scrutinee_value.ty != plan.scrutinee.ty || bodies.len() != plan.arms.len() {
        return Err(Error::msg(
            "semantic match scrutinee or arm count is inconsistent with its plan",
        ));
    }
    for (arm, body) in plan.arms.iter().zip(&bodies) {
        if arm.body_type != body.ty {
            return Err(Error::msg(
                "semantic match arm body type is inconsistent with its plan",
            ));
        }
    }

    let builder = MatchLowerer {
        origin: plan.origin,
        enums,
    };
    let mut lowered = builder.scalar(Type::Never, ExprKind::MatchUnreachable { plan: plan.id });
    for (arm, body) in plan.arms.iter().zip(bodies).rev() {
        let value = builder.match_load(&plan.scrutinee);
        let condition = builder.match_condition(&arm.pattern, value.clone())?;
        let success = builder.match_success(&arm.pattern, value, body)?;
        lowered = builder.match_if(condition, success, lowered)?;
    }
    lowered = builder.local_scope(&plan.scrutinee, scrutinee_value, lowered);
    if lowered.ty != plan.result_type {
        return Err(Error::msg(
            "semantic match lowering result type differs from its plan",
        ));
    }
    Ok(lowered)
}

struct MatchLowerer<'a> {
    origin: Origin,
    enums: &'a [EnumDefinition],
}

impl MatchLowerer<'_> {
    fn match_condition(&self, pattern: &MatchPattern, value: Expr) -> Result<Expr> {
        crate::stack::grow(|| self.match_condition_inner(pattern, value))
    }

    fn match_condition_inner(&self, pattern: &MatchPattern, value: Expr) -> Result<Expr> {
        match pattern {
            MatchPattern::Wildcard { .. } | MatchPattern::Binding { .. } => {
                Ok(self.scalar(Type::Bool, ExprKind::LitBool(true)))
            }
            MatchPattern::Bool(literal) => {
                self.match_equal(value, Type::Bool, ExprKind::LitBool(*literal))
            }
            MatchPattern::I64(literal) => {
                self.match_equal(value, Type::I64, ExprKind::LitI64(*literal))
            }
            MatchPattern::Variant {
                enum_id,
                variant,
                layout,
                fields,
                ..
            } => {
                let value_effects = value.effects;
                let mut condition = self.expression(
                    Type::Bool,
                    value_effects.union(EffectSet::READS_MEMORY),
                    ExprKind::EnumIsVariant {
                        enum_id: *enum_id,
                        variant: *variant,
                        layout: *layout,
                        value: Box::new(value.clone()),
                    },
                );
                for field in fields {
                    if matches!(
                        field.pattern,
                        MatchPattern::Wildcard { .. } | MatchPattern::Binding { .. }
                    ) {
                        continue;
                    }
                    let projected =
                        self.enum_projection(*enum_id, *variant, *layout, field, value.clone())?;
                    let nested = self.match_condition(&field.pattern, projected)?;
                    condition = self.match_and(condition, nested)?;
                }
                Ok(condition)
            }
            MatchPattern::Product {
                product, fields, ..
            } => {
                let mut condition = self.scalar(Type::Bool, ExprKind::LitBool(true));
                for field in fields {
                    if matches!(field.pattern, MatchPattern::Wildcard { .. }) {
                        continue;
                    }
                    let projection = field
                        .projection
                        .as_ref()
                        .ok_or_else(|| Error::msg("non-wildcard product field lacks projection"))?;
                    let value_effects = value.effects;
                    let projected = self.expression(
                        projection.ty.clone(),
                        value_effects.union(EffectSet::READS_MEMORY),
                        ExprKind::ProductField {
                            product: *product,
                            field: field.field_index,
                            value: Box::new(value.clone()),
                        },
                    );
                    let nested = self.match_condition(&field.pattern, projected)?;
                    condition = self.match_and(condition, nested)?;
                }
                Ok(condition)
            }
        }
    }

    fn match_success(&self, pattern: &MatchPattern, value: Expr, body: Expr) -> Result<Expr> {
        crate::stack::grow(|| self.match_success_inner(pattern, value, body))
    }

    fn match_success_inner(
        &self,
        pattern: &MatchPattern,
        value: Expr,
        mut body: Expr,
    ) -> Result<Expr> {
        match pattern {
            MatchPattern::Wildcard { .. } | MatchPattern::Bool(_) | MatchPattern::I64(_) => {
                Ok(body)
            }
            MatchPattern::Binding { local } => Ok(self.local_scope(local, value, body)),
            MatchPattern::Variant {
                enum_id,
                variant,
                layout,
                fields,
                ..
            } => {
                for field in fields.iter().rev() {
                    if matches!(field.pattern, MatchPattern::Wildcard { .. }) {
                        continue;
                    }
                    let projection = field
                        .projection
                        .as_ref()
                        .ok_or_else(|| Error::msg("non-wildcard enum field lacks projection"))?;
                    let projected =
                        self.enum_projection(*enum_id, *variant, *layout, field, value.clone())?;
                    body = self.match_success(&field.pattern, self.match_load(projection), body)?;
                    body = self.local_scope(projection, projected, body);
                }
                Ok(body)
            }
            MatchPattern::Product {
                product, fields, ..
            } => {
                for field in fields.iter().rev() {
                    if matches!(field.pattern, MatchPattern::Wildcard { .. }) {
                        continue;
                    }
                    let projection = field
                        .projection
                        .as_ref()
                        .ok_or_else(|| Error::msg("non-wildcard product field lacks projection"))?;
                    let value_effects = value.effects;
                    let projected = self.expression(
                        projection.ty.clone(),
                        value_effects.union(EffectSet::READS_MEMORY),
                        ExprKind::ProductField {
                            product: *product,
                            field: field.field_index,
                            value: Box::new(value.clone()),
                        },
                    );
                    body = self.match_success(&field.pattern, self.match_load(projection), body)?;
                    body = self.local_scope(projection, projected, body);
                }
                Ok(body)
            }
        }
    }

    fn enum_projection(
        &self,
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        field: &MatchFieldPattern,
        value: Expr,
    ) -> Result<Expr> {
        let definition = self
            .enums
            .iter()
            .find(|item| item.id == enum_id)
            .ok_or_else(|| Error::msg("match lowering lost EnumId"))?;
        let selected = definition
            .variants
            .iter()
            .find(|item| item.id == variant)
            .ok_or_else(|| Error::msg("match lowering lost VariantId"))?;
        let declared = usize::try_from(field.field_index)
            .ok()
            .and_then(|index| selected.fields.get(index))
            .filter(|item| item.name == field.name)
            .ok_or_else(|| Error::msg("match lowering lost variant field"))?;
        let projection = field
            .projection
            .as_ref()
            .ok_or_else(|| Error::msg("wildcard field requested a projection"))?;
        let effects = value.effects.union(EffectSet::READS_MEMORY);
        Ok(self.expression(
            projection.ty.clone(),
            effects,
            ExprKind::EnumField {
                enum_id,
                variant,
                field: declared.id,
                field_index: field.field_index,
                layout,
                value: Box::new(value),
            },
        ))
    }

    fn match_equal(&self, value: Expr, ty: Type, literal: ExprKind) -> Result<Expr> {
        let operation = Operation::EqualValue;
        let literal = self.scalar(ty.clone(), literal);
        let effects = value
            .effects
            .union(literal.effects)
            .union(operation.effects());
        Ok(self.expression(
            Type::Bool,
            effects,
            ExprKind::Operation {
                operation,
                resolved_signature: Type::Fn {
                    params: vec![ty, value.ty.clone()],
                    ret: Box::new(Type::Bool),
                },
                args: vec![value, literal],
            },
        ))
    }

    fn match_and(&self, left: Expr, right: Expr) -> Result<Expr> {
        let operation = Operation::And;
        let effects = left.effects.union(right.effects).union(operation.effects());
        Ok(self.expression(
            Type::Bool,
            effects,
            ExprKind::Operation {
                operation,
                resolved_signature: Type::Fn {
                    params: vec![Type::Bool, Type::Bool],
                    ret: Box::new(Type::Bool),
                },
                args: vec![left, right],
            },
        ))
    }

    fn match_if(&self, condition: Expr, then_branch: Expr, else_branch: Expr) -> Result<Expr> {
        let ty = Type::join_control(&then_branch.ty, &else_branch.ty)
            .ok_or_else(|| Error::msg("match lowering produced incompatible branch types"))?;
        let effects = condition
            .effects
            .union(then_branch.effects)
            .union(else_branch.effects);
        Ok(self.expression(
            ty,
            effects,
            ExprKind::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            },
        ))
    }

    fn local_scope(&self, local: &MatchLocal, value: Expr, body: Expr) -> Expr {
        let effects = value.effects.union(body.effects);
        self.expression(
            body.ty.clone(),
            effects,
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

    fn match_load(&self, local: &MatchLocal) -> Expr {
        self.scalar(
            local.ty.clone(),
            ExprKind::Load(BindingRef {
                binding: local.binding,
                storage: BindingStorage::Local(local.slot),
            }),
        )
    }

    fn scalar(&self, ty: Type, kind: ExprKind) -> Expr {
        self.expression(ty, EffectSet::PURE, kind)
    }

    fn expression(&self, ty: Type, effects: EffectSet, kind: ExprKind) -> Expr {
        Expr {
            ty,
            effects,
            origin: self.origin,
            kind,
        }
    }
}
