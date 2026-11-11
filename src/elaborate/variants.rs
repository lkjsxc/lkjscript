//! Source variant shapes and declaration-ordered enum payload construction.
use super::{Builder, Result, a, err, k, qualified};
use std::collections::BTreeMap;

impl Builder<'_, '_> {
    pub(super) fn enum_definition(&self, ty: usize, span: a::Span) -> Result<usize> {
        let (name, _) = self
            .compiler
            .applications
            .get(&ty)
            .ok_or_else(|| err(span, "E_TYPE", "enum declaration identity is missing"))?;
        let declaration = self
            .compiler
            .declaration_names
            .get(name)
            .ok_or_else(|| err(span, "E_TYPE", "enum declaration identity is missing"))?;
        match self.compiler.source.declarations[*declaration].item {
            a::Item::Enum(index) => Ok(index),
            _ => Err(err(span, "E_TYPE", "variant requires an enum declaration")),
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
        mut expected: Option<usize>,
        span: a::Span,
    ) -> Result<Option<(usize, usize)>> {
        let name = path.name.rsplit("::").next().unwrap();
        let direct =
            path.declaration
                .and_then(|id| match self.compiler.source.declarations[id].item {
                    a::Item::Enum(index) => Some(index),
                    _ => None,
                });
        let (index, tag) = if let Some(index) = direct {
            let definition = &self.compiler.source.enums[index];
            let prefix = format!("{}::", qualified(&definition.module, &definition.name));
            let Some(name) = path.name.strip_prefix(&prefix) else {
                return Ok(None);
            };
            let Some(tag) = definition
                .variants
                .iter()
                .position(|variant| variant.name == name)
            else {
                return Ok(None);
            };
            (index, tag)
        } else if path.declaration.is_none() && path.name.starts_with("Self::") {
            let ty = *self
                .sub
                .get("Self")
                .ok_or_else(|| err(span, "E_TYPE", "Self has no concrete type"))?;
            let index = self.enum_definition(ty, span)?;
            let Some(tag) = self.compiler.source.enums[index]
                .variants
                .iter()
                .position(|variant| variant.name == name)
            else {
                return Ok(None);
            };
            expected = Some(ty);
            (index, tag)
        } else if let Ok((ty, tag)) = self.variant(path, expected, span) {
            let index = self.enum_definition(ty, span)?;
            expected = Some(ty);
            (index, tag)
        } else {
            return Ok(None);
        };
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
        let parameters: Vec<_> = definition.generics.iter().filter(|p| !p.lifetime).collect();
        let mut substitutions = BTreeMap::new();
        if let Some(ty) = expected {
            if self.enum_definition(ty, span)? != index {
                return Err(err(
                    span,
                    "E_TYPE",
                    "variant names a different enum declaration",
                ));
            }
            let (_, arguments) = &self.compiler.applications[&ty];
            for (parameter, argument) in parameters.iter().zip(arguments) {
                substitutions.insert(parameter.name.clone(), *argument);
            }
        }
        let type_args: Vec<_> = path.args.iter().copied().filter(|id| !matches!(&self.compiler.source.types[*id].kind, a::Type::Path(p) if p.name.starts_with('\''))).collect();
        // Alias applications have already selected their concrete enum above.
        if direct.is_some() && !type_args.is_empty() {
            if type_args.len() != parameters.len() {
                return Err(err(span, "E_TYPE", "wrong number of enum type arguments"));
            }
            for (parameter, argument) in parameters.iter().zip(type_args) {
                let ty = self.compiler.resolve(argument, &self.sub)?;
                if substitutions
                    .get(&parameter.name)
                    .is_some_and(|expected| *expected != ty)
                {
                    return Err(err(
                        span,
                        "E_TYPE",
                        "explicit enum type arguments differ from expected type",
                    ));
                }
                substitutions.insert(parameter.name.clone(), ty);
            }
        }
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
                self.infer_type(variant.fields[*field].ty, hint, &mut substitutions)?;
            }
        }
        let mut ordered = vec![None; variant.fields.len()];
        for ((_, expression), field) in values.iter().zip(positions) {
            let declared = variant.fields[field].ty;
            let hint = self.compiler.resolve(declared, &substitutions).ok();
            let value = self.expr(*expression, hint)?;
            self.infer_type(declared, value.1, &mut substitutions)?;
            ordered[field] = Some(value);
        }
        let owner = qualified(&definition.module, &definition.name);
        let ty = self.compiler.named_type(
            &a::Path {
                declaration: self.compiler.declaration_names.get(&owner).copied(),
                name: owner,
                args: Vec::new(),
            },
            &substitutions,
            span,
        )?;
        if expected.is_some_and(|expected| expected != ty) {
            return Err(err(
                span,
                "E_TYPE",
                "variant constructor result type differs",
            ));
        }
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
