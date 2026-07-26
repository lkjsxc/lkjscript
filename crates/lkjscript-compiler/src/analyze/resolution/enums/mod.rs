mod prelude;

use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn resolve_enum_value(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let [type_form, variant_form, fields_form] = args else {
            return Err(self.error("variant-value expects exactly type/, variant/, and fields/"));
        };
        let unresolved = match type_form {
            AstExpr::Call { name, args } if name == "type" => parse_type_form(args),
            _ => Err("variant-value expects type/ first".into()),
        }
        .map_err(|message| self.error(message))?;
        let parameters: Vec<_> = self.type_variables.iter().cloned().collect();
        let ty = self
            .analyzer
            .resolve_enum_type(&unresolved, &parameters)
            .map_err(|message| self.error(message))?;
        if ty.contains_never() {
            return Err(self.error("Never is not an enum substitution or runtime value type"));
        }
        let Type::Enum { id, arguments, .. } = &ty else {
            return Err(self.error("variant-value type/ must name a fully instantiated enum"));
        };
        let definition = self
            .analyzer
            .enums
            .iter()
            .find(|definition| definition.id == *id)
            .cloned()
            .ok_or_else(|| self.error("variant-value references unknown EnumId"))?;
        let variant_name = match variant_form {
            AstExpr::Call { name, args } if name == "variant" => match args.as_slice() {
                [name] => symbolic_name(name),
                _ => Err("variant/ expects exactly one variant name".into()),
            },
            _ => Err("variant-value expects variant/ second".into()),
        }
        .map_err(|message| self.error(message))?;
        let variant = definition
            .variants
            .iter()
            .find(|variant| variant.name == variant_name)
            .ok_or_else(|| {
                self.error(format!(
                    "enum {} has no variant {variant_name}",
                    definition.name
                ))
            })?;
        let forms = match fields_form {
            AstExpr::Call { name, args } if name == "fields" => args,
            _ => return Err(self.error("variant-value expects fields/ third")),
        };
        if forms.len() != variant.fields.len() {
            return Err(self.error(format!(
                "variant-value {}.{}: expected {} fields, got {}",
                definition.name,
                variant.name,
                variant.fields.len(),
                forms.len()
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
            let AstExpr::Call { name, args } = form else {
                return Err(self.error("variant-value fields must be variant-field/ forms"));
            };
            let [name_form, value_form] = args.as_slice() else {
                return Err(self.error("constructor variant-field expects name/ and one value"));
            };
            if name != "variant-field" {
                return Err(self.error("variant-value fields must be variant-field/ forms"));
            }
            let field_name = declared_name_form(name_form, "constructor variant-field")
                .map_err(|message| self.error(message))?;
            if field_name != declared.name {
                return Err(self.error(format!(
                    "variant-value {}.{} field {} must be {} in declaration order, got {field_name}",
                    definition.name,
                    variant.name,
                    index + 1,
                    declared.name
                )));
            }
            let value = self.resolve_expr(value_form)?;
            let expected = declared.ty.subst(&substitutions);
            if !Type::unify_assignable(&value.ty, &expected) {
                return Err(self.error(format!(
                    "variant-value {}.{} field {field_name}: value type {:?} not assignable to {:?}",
                    definition.name, variant.name, value.ty, expected
                )));
            }
            fields.push(value);
        }
        Ok(self.expression(
            ty,
            ExprKind::EnumValue {
                enum_id: definition.id,
                variant: variant.id,
                layout: definition.layout.identity,
                fields,
            },
        ))
    }
}
