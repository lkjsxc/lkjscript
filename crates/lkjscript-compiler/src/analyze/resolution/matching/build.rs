use crate::analyze::*;

impl Resolver<'_> {
    pub(super) fn match_condition(&self, pattern: &MatchPattern, value: Expr) -> Result<Expr> {
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
                for (index, field) in fields.iter().enumerate() {
                    let field_index = u8::try_from(index)
                        .map_err(|_| self.error("product match field index exceeds u8"))?;
                    let projected = self.expression(
                        field.projection.ty.clone(),
                        ExprKind::ProductField {
                            product: *product,
                            field: field_index,
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
                    let projected =
                        self.enum_projection(*enum_id, *variant, *layout, field, value.clone())?;
                    body = self.match_success(
                        &field.pattern,
                        self.match_load(&field.projection),
                        body,
                    )?;
                    body = self.local_scope(&field.projection, projected, body);
                }
                Ok(body)
            }
            MatchPattern::Product {
                product, fields, ..
            } => {
                for (index, field) in fields.iter().enumerate().rev() {
                    let field_index = u8::try_from(index)
                        .map_err(|_| self.error("product match field index exceeds u8"))?;
                    let projected = self.expression(
                        field.projection.ty.clone(),
                        ExprKind::ProductField {
                            product: *product,
                            field: field_index,
                            value: Box::new(value.clone()),
                        },
                    );
                    body = self.match_success(
                        &field.pattern,
                        self.match_load(&field.projection),
                        body,
                    )?;
                    body = self.local_scope(&field.projection, projected, body);
                }
                Ok(body)
            }
        }
    }
}
