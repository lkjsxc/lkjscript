use crate::analyze::*;

impl Resolver<'_> {
    pub(super) fn parse_variant_pattern(
        &mut self,
        args: &[AstExpr],
        expected: &Type,
    ) -> Result<MatchPattern> {
        let [type_form, variant_form, fields_form] = args else {
            return Err(self.error("variant-pattern expects type/, variant/, and fields/"));
        };
        let ty = self.resolve_pattern_type(type_form)?;
        if &ty != expected {
            return Err(self.error(format!(
                "variant-pattern type {ty} does not exactly equal scrutinee type {expected}",
            )));
        }
        let Type::Enum { id, arguments, .. } = &ty else {
            return Err(self.error("variant-pattern type must be an instantiated enum"));
        };
        let definition = self
            .analyzer
            .enums
            .iter()
            .find(|item| item.id == *id)
            .cloned()
            .ok_or_else(|| self.error("variant-pattern EnumId is unknown"))?;
        let variant_name = one_named_value(variant_form, "variant", "variant-pattern variant")
            .map_err(|message| self.error(message))?;
        let variant = definition
            .variants
            .iter()
            .find(|item| item.name == variant_name)
            .ok_or_else(|| {
                self.error(format!(
                    "enum {} has no variant {variant_name}",
                    definition.name,
                ))
            })?;
        let forms = fields_children(fields_form).map_err(|message| self.error(message))?;
        if forms.len() != variant.fields.len() {
            return Err(self.error(format!(
                "variant-pattern {}.{} expected {} fields, got {}",
                definition.name,
                variant.name,
                variant.fields.len(),
                forms.len(),
            )));
        }
        let substitutions: HashMap<_, _> = definition
            .type_parameters
            .iter()
            .cloned()
            .zip(arguments.iter().cloned())
            .collect();
        let mut fields = Vec::with_capacity(forms.len());
        for (index, (form, declared)) in forms.iter().zip(&variant.fields).enumerate() {
            let (name, nested) = named_pattern(form, "variant-field-pattern")
                .map_err(|message| self.error(message))?;
            if name != declared.name {
                return Err(self.error(format!(
                    "variant-pattern field {} must be {} in declaration order, got {name}",
                    index + 1,
                    declared.name,
                )));
            }
            let field_ty = declared.ty.subst(&substitutions);
            let projection = self.allocate_hidden_match_local(field_ty.clone())?;
            let pattern = self.parse_match_pattern(nested, &field_ty)?;
            fields.push(MatchFieldPattern {
                name,
                projection,
                pattern,
            });
        }
        Ok(MatchPattern::Variant {
            ty,
            enum_id: definition.id,
            variant: variant.id,
            layout: definition.layout.identity,
            fields,
        })
    }

    pub(super) fn parse_product_pattern(
        &mut self,
        args: &[AstExpr],
        expected: &Type,
    ) -> Result<MatchPattern> {
        let [type_form, fields_form] = args else {
            return Err(self.error("product-pattern expects type/ and fields/"));
        };
        let ty = self.resolve_pattern_type(type_form)?;
        if &ty != expected {
            return Err(self.error(format!(
                "product-pattern type {ty} does not exactly equal scrutinee type {expected}",
            )));
        }
        let Type::Product(name) = &ty else {
            return Err(self.error("product-pattern type must name a Product"));
        };
        let definition = self
            .analyzer
            .product_by_name(name)
            .map_err(|_| self.error(format!("unknown product type {name}")))?
            .clone();
        let forms = fields_children(fields_form).map_err(|message| self.error(message))?;
        if forms.len() != definition.fields.len() {
            return Err(self.error(format!(
                "product-pattern {} expected {} fields, got {}",
                name,
                definition.fields.len(),
                forms.len(),
            )));
        }
        let mut fields = Vec::with_capacity(forms.len());
        for (index, (form, declared)) in forms.iter().zip(&definition.fields).enumerate() {
            let (field_name, nested) = named_pattern(form, "product-field-pattern")
                .map_err(|message| self.error(message))?;
            if field_name != declared.name {
                return Err(self.error(format!(
                    "product-pattern field {} must be {} in declaration order, got {field_name}",
                    index + 1,
                    declared.name,
                )));
            }
            let projection = self.allocate_hidden_match_local(declared.ty.clone())?;
            let pattern = self.parse_match_pattern(nested, &declared.ty)?;
            fields.push(MatchFieldPattern {
                name: field_name,
                projection,
                pattern,
            });
        }
        Ok(MatchPattern::Product {
            ty,
            product: definition.id,
            fields,
        })
    }
}

fn fields_children(form: &AstExpr) -> std::result::Result<&[AstExpr], String> {
    match form {
        AstExpr::Call { name, args } if name == "fields" => Ok(args),
        _ => Err("pattern expects fields/ marker".into()),
    }
}

fn one_named_value(
    form: &AstExpr,
    marker: &str,
    context: &str,
) -> std::result::Result<String, String> {
    match form {
        AstExpr::Call { name, args } if name == marker => match args.as_slice() {
            [value] => symbolic_name(value),
            _ => Err(format!("{context} expects one name")),
        },
        _ => Err(format!("{context} expects {marker}/")),
    }
}

fn named_pattern<'a>(
    form: &'a AstExpr,
    marker: &str,
) -> std::result::Result<(String, &'a AstExpr), String> {
    let AstExpr::Call { name, args } = form else {
        return Err(format!("fields must be {marker}/ forms"));
    };
    if name != marker {
        return Err(format!("fields must be {marker}/ forms"));
    }
    let [name, pattern] = args.as_slice() else {
        return Err(format!("{marker} expects name/ and one pattern"));
    };
    Ok((declared_name_form(name, marker)?, pattern))
}
