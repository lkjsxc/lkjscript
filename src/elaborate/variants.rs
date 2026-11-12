//! Source variant shapes and declaration-ordered enum payload construction.
use super::{Builder, Result, a, err, k};
use std::collections::BTreeMap;

impl Builder<'_, '_> {
    pub(super) fn enum_definition(&self, ty: usize, span: a::Span) -> Result<usize> {
        let declaration = self.nominal_declaration(ty, span)?;
        match self.compiler.source.declarations[declaration].item {
            a::Item::Enum(index) => Ok(index),
            _ => Err(err(span, "E_TYPE", "variant requires an enum declaration")),
        }
    }

    pub(super) fn variant_name(&self, path: &a::Path) -> Option<String> {
        let owner = if let Some(declaration) = path.declaration {
            if !matches!(
                self.compiler.source.declarations[declaration].item,
                a::Item::Enum(_) | a::Item::Alias(_)
            ) {
                return None;
            }
            self.compiler.source.declarations[declaration].name.as_str()
        } else if path.name.starts_with("Self::") {
            "Self"
        } else {
            return None;
        };
        let name = path.name.strip_prefix(owner)?.strip_prefix("::")?;
        if name.contains("::") {
            None
        } else {
            Some(name.to_string())
        }
    }

    pub(super) fn variant_style(
        &self,
        ty: usize,
        tag: usize,
        style: a::VariantStyle,
        span: a::Span,
    ) -> Result<()> {
        let index = self.enum_definition(ty, span)?;
        let variant = &self.compiler.source.enums[index].variants[tag];
        if variant.style != style && !(style == a::VariantStyle::Named && variant.fields.is_empty())
        {
            return Err(err(
                span,
                "E_TYPE",
                match style {
                    a::VariantStyle::Unit => "variant requires its declared tuple or named fields",
                    a::VariantStyle::Tuple => "tuple variant syntax requires a tuple variant",
                    a::VariantStyle::Named => "named field syntax requires a named variant",
                },
            ));
        }
        Ok(())
    }

    pub(super) fn construct_variant(
        &mut self,
        path: &a::Path,
        values: &[(String, usize)],
        style: a::VariantStyle,
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<Option<(usize, usize)>> {
        let Some(name) = self.variant_name(path) else {
            return Ok(None);
        };
        let Some(mut target) = self.constructor_target(path, expected, span)? else {
            return Ok(None);
        };
        let a::Item::Enum(index) = self.compiler.source.declarations[target.declaration].item
        else {
            return Ok(None);
        };
        let Some(tag) = self.compiler.variant_positions[index].get(&name).copied() else {
            if style == a::VariantStyle::Named {
                return Err(err(span, "E_TYPE", "unknown enum variant"));
            }
            return Ok(None);
        };
        self.constrain_constructor_target(&mut target, span)?;
        let definition = self.compiler.source.enums[index].clone();
        let variant = &definition.variants[tag];
        if variant.style != style && !(style == a::VariantStyle::Named && variant.fields.is_empty())
        {
            return Err(err(
                span,
                "E_TYPE",
                match style {
                    a::VariantStyle::Unit => "variant requires its declared tuple or named fields",
                    a::VariantStyle::Tuple => "tuple constructor requires a tuple variant",
                    a::VariantStyle::Named => "named constructor requires a named variant",
                },
            ));
        }
        let mut field_indices = BTreeMap::new();
        for (index, field) in variant.fields.iter().enumerate() {
            field_indices.insert(field.name.clone(), index);
        }
        let mut positions = Vec::new();
        let mut supplied = vec![false; variant.fields.len()];
        for (name, _) in values {
            let field = field_indices
                .get(name)
                .copied()
                .ok_or_else(|| err(span, "E_TYPE", &format!("unknown variant field {name}")))?;
            if supplied[field] {
                return Err(err(
                    span,
                    "E_TYPE",
                    &format!("duplicate variant field {name}"),
                ));
            }
            supplied[field] = true;
            positions.push(field);
        }
        if supplied.iter().any(|found| !found) {
            return Err(err(span, "E_TYPE", "variant constructor is missing fields"));
        }
        let mut coercion_substitutions = target.substitutions.clone();
        // Known field types constrain unsuffixed literals without evaluating any
        // initializer out of source order.
        for ((_, expression), field) in values.iter().zip(&positions) {
            // A method spelling alone is not evidence of its resolved return type.
            if matches!(
                self.compiler.source.exprs[*expression].kind,
                a::Expr::Method { .. }
            ) {
                continue;
            }
            if let Some(hint) = self.hint(*expression) {
                self.infer_type(variant.fields[*field].ty, hint, &mut target.substitutions)?;
            }
        }
        let mut ordered = vec![None; variant.fields.len()];
        for ((_, expression), field) in values.iter().zip(positions) {
            let declared = variant.fields[field].ty;
            let value = self.constructor_argument(
                *expression,
                declared,
                &target.substitutions,
                &mut coercion_substitutions,
            )?;
            self.infer_type(declared, value.1, &mut target.substitutions)?;
            ordered[field] = Some(value);
        }
        let ty = self.finish_constructor(target, span)?;
        let k::TypeKind::Enum { variants, .. } = &self.compiler.types[ty].kind else {
            unreachable!()
        };
        let mut fields = Vec::new();
        for (value, field) in ordered.into_iter().zip(&variants[tag].fields) {
            let (local, actual) = value.expect("all variant fields supplied");
            if actual != field.ty {
                return Err(err(
                    span,
                    "E_TYPE",
                    &format!("variant field {} type differs", field.name),
                ));
            }
            fields.push(local);
        }
        let dst = self.temp(ty);
        self.emit(
            k::Op::Variant {
                dst,
                variant: tag,
                fields,
            },
            span.line,
        );
        Ok(Some((dst, ty)))
    }
}
