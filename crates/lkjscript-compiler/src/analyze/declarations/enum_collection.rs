use crate::analyze::*;

impl Analyzer {
    pub(in crate::analyze) fn collect_enum_names(
        &mut self,
        program: &ValidatedSourceTree,
    ) -> Result<()> {
        let mut identities = HashSet::new();
        for (source_index, file) in program.files().iter().enumerate() {
            let source = SourceId::new(
                u32::try_from(source_index)
                    .map_err(|_| Error::msg("too many source files for HIR SourceId"))?,
            );
            for form in &file.forms {
                let AstExpr::Call { name, args } = form else {
                    continue;
                };
                if name != "enum" {
                    continue;
                }
                let parsed =
                    enum_declaration(args).map_err(|message| self.error(source, message))?;
                self.validate_nominal_name(source, &parsed.name, "enum")?;
                let mut parameters = HashSet::new();
                for parameter in &parsed.parameters {
                    if !super::enum_fields::valid_enum_parameter(parameter) {
                        return Err(self.error(
                            source,
                            format!("enum {}: invalid forall parameter {parameter}", parsed.name),
                        ));
                    }
                    if !parameters.insert(parameter.as_str()) {
                        return Err(self.error(
                            source,
                            format!(
                                "enum {}: duplicate forall parameter {parameter}",
                                parsed.name
                            ),
                        ));
                    }
                }
                if parsed.variants.len() > MAX_ENUM_VARIANTS {
                    return Err(self.error(
                        source,
                        format!(
                            "enum {}: too many variants ({} > {MAX_ENUM_VARIANTS})",
                            parsed.name,
                            parsed.variants.len()
                        ),
                    ));
                }
                let declaration = program
                    .declarations()
                    .iter()
                    .find(|declaration| {
                        declaration.kind() == crate::source::DeclarationKind::Enum
                            && declaration.name() == parsed.name
                            && declaration.origin().logical_path() == file.origin.logical_path
                    })
                    .ok_or_else(|| self.error(source, "enum declaration identity is missing"))?;
                let id = EnumId::new(declaration.key().digest());
                if !id.is_resolved() || !identities.insert(id) {
                    return Err(self.error(source, "stable EnumId collision"));
                }
                if self
                    .enum_headers
                    .insert(parsed.name.clone(), (id, parsed.parameters.clone()))
                    .is_some()
                {
                    return Err(self.error(
                        source,
                        format!("duplicate enum declaration {}", parsed.name),
                    ));
                }
            }
        }
        Ok(())
    }

    pub(in crate::analyze) fn collect_enums(
        &mut self,
        program: &ValidatedSourceTree,
    ) -> Result<()> {
        let mut variant_ids = HashSet::new();
        let mut field_ids = HashSet::new();
        for (source_index, file) in program.files().iter().enumerate() {
            let source = SourceId::new(
                u32::try_from(source_index)
                    .map_err(|_| Error::msg("too many source files for HIR SourceId"))?,
            );
            for form in &file.forms {
                let AstExpr::Call { name, args } = form else {
                    continue;
                };
                if name != "enum" {
                    continue;
                }
                let parsed =
                    enum_declaration(args).map_err(|message| self.error(source, message))?;
                let (id, parameters) = self
                    .enum_headers
                    .get(&parsed.name)
                    .cloned()
                    .ok_or_else(|| self.error(source, "enum header is missing"))?;
                let mut variant_names = HashSet::new();
                let mut variants = Vec::with_capacity(parsed.variants.len());
                for (variant_order, variant_form) in parsed.variants.iter().enumerate() {
                    let (variant_name, field_forms) =
                        parse_variant(variant_form).map_err(|message| {
                            self.error(source, format!("enum {}: {message}", parsed.name))
                        })?;
                    if !is_declaration_type_name(&variant_name) {
                        return Err(self.error(
                            source,
                            format!("enum {}: invalid variant name {variant_name}", parsed.name),
                        ));
                    }
                    if !variant_names.insert(variant_name.clone()) {
                        return Err(self.error(
                            source,
                            format!("enum {}: duplicate variant {variant_name}", parsed.name),
                        ));
                    }
                    if field_forms.len() > MAX_VARIANT_FIELDS {
                        return Err(self.error(
                            source,
                            format!(
                                "enum {} variant {variant_name}: too many fields",
                                parsed.name
                            ),
                        ));
                    }
                    let variant_id = VariantId::new(crate::source::enum_member_identity(
                        id.bytes(),
                        "variant",
                        &variant_name,
                    ));
                    if !variant_id.is_resolved() || !variant_ids.insert(variant_id) {
                        return Err(self.error(source, "stable VariantId collision"));
                    }
                    let fields = self.collect_variant_fields(
                        source,
                        &parsed.name,
                        variant_id,
                        &variant_name,
                        field_forms,
                        &parameters,
                        &mut field_ids,
                    )?;
                    variants.push(EnumVariant {
                        id: variant_id,
                        name: variant_name,
                        source_order: u16::try_from(variant_order)
                            .map_err(|_| self.error(source, "variant order exceeds u16"))?,
                        fields,
                    });
                }
                let recursive = variants
                    .iter()
                    .flat_map(|variant| &variant.fields)
                    .any(|field| super::enum_fields::type_contains_enum(&field.ty, id));
                let layout = super::enum_fields::enum_layout(id, recursive);
                self.enums.push(EnumDefinition {
                    id,
                    name: parsed.name,
                    origin: source,
                    type_parameters: parameters,
                    variants,
                    layout,
                });
            }
        }
        self.validate_enum_recursion()
    }
}
