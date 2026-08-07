use crate::analyze::*;

impl Resolver<'_> {
    pub(super) fn match_condition(&self, pattern: &MatchPattern, value: Expr) -> Result<Expr> {
        crate::stack::grow(|| self.match_condition_inner(pattern, value))
    }

    fn match_condition_inner(&self, pattern: &MatchPattern, value: Expr) -> Result<Expr> {
        match pattern {
            MatchPattern::Wildcard { .. } | MatchPattern::Binding { .. } => {
                Ok(self.expression(Type::Bool, ExprKind::LitBool(true)))
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
                let mut condition = self.expression(
                    Type::Bool,
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
                let mut condition = self.expression(Type::Bool, ExprKind::LitBool(true));
                for field in fields {
                    if matches!(field.pattern, MatchPattern::Wildcard { .. }) {
                        continue;
                    }
                    let projection = field
                        .projection
                        .as_ref()
                        .ok_or_else(|| self.error("non-wildcard product field lacks projection"))?;
                    let projected = self.expression(
                        projection.ty.clone(),
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

    pub(super) fn match_success(
        &self,
        pattern: &MatchPattern,
        value: Expr,
        body: Expr,
    ) -> Result<Expr> {
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
                        .ok_or_else(|| self.error("non-wildcard enum field lacks projection"))?;
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
                        .ok_or_else(|| self.error("non-wildcard product field lacks projection"))?;
                    let projected = self.expression(
                        projection.ty.clone(),
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
}
