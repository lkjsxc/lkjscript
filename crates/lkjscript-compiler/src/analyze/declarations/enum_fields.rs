use crate::analyze::*;

pub(super) fn valid_enum_parameter(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
        && !is_builtin_type_name(name)
}

impl Analyzer {
    pub(super) fn validate_nominal_name(
        &self,
        source: SourceId,
        name: &str,
        kind: &str,
    ) -> Result<()> {
        if !is_declaration_type_name(name) {
            return Err(self.error(source, format!("invalid {kind} declaration name {name}")));
        }
        if Operation::from_name(name).is_some()
            || is_contextual_name(name)
            || is_builtin_type_name(name)
        {
            return Err(self.error(
                source,
                format!(
                    "{kind} declaration {name} collides with a reserved operation, form, or type"
                ),
            ));
        }
        if self.product_names.contains_key(name) || self.trait_names.contains_key(name) {
            return Err(self.error(
                source,
                format!("enum declaration {name} collides with another nominal declaration"),
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::analyze) fn collect_variant_fields(
        &self,
        source: SourceId,
        enum_name: &str,
        variant_id: VariantId,
        variant_name: &str,
        forms: &[AstExpr],
        parameters: &[String],
        identities: &mut HashSet<VariantFieldId>,
    ) -> Result<Vec<EnumVariantField>> {
        let mut names = HashSet::new();
        let mut fields = Vec::with_capacity(forms.len());
        for (order, form) in forms.iter().enumerate() {
            let (name, ty) = parse_variant_field(form).map_err(|message| {
                self.error(
                    source,
                    format!("enum {enum_name} variant {variant_name}: {message}"),
                )
            })?;
            if !crate::source::is_source_identifier(&name) {
                return Err(self.error(
                    source,
                    format!("enum {enum_name} variant {variant_name}: invalid field name {name}"),
                ));
            }
            if !names.insert(name.clone()) {
                return Err(self.error(
                    source,
                    format!("enum {enum_name} variant {variant_name}: duplicate field {name}"),
                ));
            }
            let ty = self.resolve_enum_type(&ty, parameters).map_err(|message| {
                self.error(
                    source,
                    format!("enum {enum_name} variant {variant_name} field {name}: {message}"),
                )
            })?;
            if ty.contains_never() {
                return Err(self.error(
                    source,
                    format!(
                        "enum {enum_name} variant {variant_name} field {name}: Never is not a field type"
                    ),
                ));
            }
            if contains_ownership_type(&ty) {
                let message = format!(
                    "enum {enum_name} variant {variant_name} field {name}: \
                     ownership/reference types cannot be stored in enums"
                );
                return Err(self.error(source, message));
            }
            let id = VariantFieldId::new(crate::source::enum_member_identity(
                variant_id.bytes(),
                "field",
                &name,
            ));
            if !id.is_resolved() || !identities.insert(id) {
                return Err(self.error(source, "stable VariantFieldId collision"));
            }
            fields.push(EnumVariantField {
                id,
                name,
                source_order: u16::try_from(order)
                    .map_err(|_| self.error(source, "field order exceeds u16"))?,
                indirect: contains_enum_type(&ty),
                traced: is_traced_type(&ty),
                ty,
            });
        }
        Ok(fields)
    }
}

pub(super) fn enum_layout(id: EnumId, recursive: bool) -> EnumLayoutFacts {
    let identity = RuntimeLayoutId::new(crate::source::enum_member_identity(
        id.bytes(),
        "runtime-layout",
        "boxed-enum-v1",
    ));
    EnumLayoutFacts {
        identity,
        recursive,
    }
}

pub(super) fn type_contains_enum(ty: &Type, expected: EnumId) -> bool {
    match ty {
        Type::Enum { id, arguments, .. } => {
            *id == expected || arguments.iter().any(|ty| type_contains_enum(ty, expected))
        }
        Type::List(inner) => type_contains_enum(inner, expected),
        _ => false,
    }
}

fn contains_enum_type(ty: &Type) -> bool {
    match ty {
        Type::Enum { .. } => true,
        Type::List(inner) => contains_enum_type(inner),
        _ => false,
    }
}

fn is_traced_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Str
            | Type::Symbol
            | Type::Buf
            | Type::Product(_)
            | Type::Enum { .. }
            | Type::List(_)
            | Type::Fn { .. }
            | Type::Forall { .. }
    )
}
