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
                ty,
            });
        }
        Ok(fields)
    }
}
