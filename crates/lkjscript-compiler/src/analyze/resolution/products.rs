use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn resolve_product_value(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let Some((name_expression, field_forms)) = args.split_first() else {
            return Err(self.error("product-value expects a product name"));
        };
        let product_name = symbolic_name(name_expression)
            .map_err(|_| self.error("product-value name must be a symbol"))?;
        let product = self
            .analyzer
            .product_by_name(&product_name)
            .map_err(|_| self.error(format!("unknown product type {product_name}")))?
            .clone();
        if field_forms.len() != product.fields.len() {
            return Err(self.error(format!(
                "product-value {product_name}: expected {} fields, got {}",
                product.fields.len(),
                field_forms.len()
            )));
        }
        let mut fields = Vec::with_capacity(field_forms.len());
        for (index, (field_form, declared)) in field_forms.iter().zip(&product.fields).enumerate() {
            let AstExpr::Call { name, args } = field_form else {
                return Err(self.error(format!(
                    "product-value {product_name}: field {} must be field/…/field",
                    declared.name
                )));
            };
            if name != "field" {
                return Err(self.error(format!(
                    "product-value {product_name}: field {} must be field/…/field",
                    declared.name
                )));
            }
            let [name_expression, value_expression] = args.as_slice() else {
                return Err(self.error(format!(
                    "product-value {product_name}: constructor field expects name and value"
                )));
            };
            let field_name = symbolic_name(name_expression)
                .map_err(|_| self.error("constructor field name must be a symbol"))?;
            if field_name != declared.name {
                return Err(self.error(format!(
                    "product-value {product_name}: field {} must be {} in declaration order, got {field_name}",
                    index + 1,
                    declared.name
                )));
            }
            let value = self.resolve_expr(value_expression)?;
            if !Type::unify_assignable(&value.ty, &declared.ty) {
                return Err(self.error(format!(
                    "product-value {product_name} field {field_name}: value type {} not assignable to {}",
                    value.ty, declared.ty
                )));
            }
            fields.push(value);
        }
        Ok(self.expression(
            Type::Product(product.id),
            ExprKind::ProductValue {
                product: product.id,
                fields,
            },
        ))
    }

    pub(in crate::analyze) fn resolve_product_field(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let [value_expression, name_expression] = args else {
            return Err(self.error("field expects a product value and field name"));
        };
        let value = self.resolve_expr(value_expression)?;
        let Type::Product(product_id) = &value.ty else {
            return Err(self.error("field value must have a concrete Product type"));
        };
        let product_id = *product_id;
        let field_name = symbolic_name(name_expression)
            .map_err(|_| self.error("field name must be a symbol"))?;
        let product = self
            .analyzer
            .product_by_id(product_id)
            .map_err(|_| self.error("unknown product type identity"))?
            .clone();
        let (field_index, field) = product
            .fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.name == field_name)
            .ok_or_else(|| {
                self.error(format!(
                    "product {} has no field {field_name}",
                    product.name
                ))
            })?;
        let field_index = u64::try_from(field_index)
            .map_err(|_| self.error("product field index exceeds u64"))?;
        Ok(self.expression(
            field.ty.clone(),
            ExprKind::ProductField {
                product: product.id,
                field: field_index,
                value: Box::new(value),
            },
        ))
    }

    pub(in crate::analyze) fn resolve_with_product_field(
        &mut self,
        args: &[AstExpr],
    ) -> Result<Expr> {
        let [value_expression, name_expression, replacement_expression] = args else {
            return Err(
                self.error("with-field expects a product value, field name, and replacement")
            );
        };
        let value = self.resolve_expr(value_expression)?;
        let Type::Product(product_id) = &value.ty else {
            return Err(self.error("with-field value must have a concrete Product type"));
        };
        let product_id = *product_id;
        let field_name = symbolic_name(name_expression)
            .map_err(|_| self.error("with-field name must be a symbol"))?;
        let product = self
            .analyzer
            .product_by_id(product_id)
            .map_err(|_| self.error("unknown product type identity"))?
            .clone();
        let (field_index, field) = product
            .fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.name == field_name)
            .ok_or_else(|| {
                self.error(format!(
                    "product {} has no field {field_name}",
                    product.name
                ))
            })?;
        let field_index = u64::try_from(field_index)
            .map_err(|_| self.error("product field index exceeds u64"))?;
        let replacement = self.resolve_expr(replacement_expression)?;
        if !Type::unify_assignable(&replacement.ty, &field.ty) {
            return Err(self.error(format!(
                "with-field {}.{field_name}: replacement type {} not assignable to {}",
                product.name, replacement.ty, field.ty
            )));
        }
        Ok(self.expression(
            Type::Product(product.id),
            ExprKind::WithProductField {
                product: product.id,
                field: field_index,
                value: Box::new(value),
                replacement: Box::new(replacement),
            },
        ))
    }
}
