use super::*;

impl VerifiedTypes<'_> {
    pub(crate) fn verified_recursive_fields(
        &self,
        key: &VerifiedDeclarationKey,
    ) -> Result<Vec<(Type, MemoryTypePathElement)>> {
        match key {
            VerifiedDeclarationKey::Product(id) => {
                let item = self.product_definition(*id)?;
                item.fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        Ok((
                            field.ty.clone(),
                            MemoryTypePathElement::ProductField {
                                index: index_u64(index)?,
                                field: field.identity,
                            },
                        ))
                    })
                    .collect()
            }
            VerifiedDeclarationKey::Enum(id) => {
                let item = self.enum_definition(*id)?;
                let mut fields = Vec::new();
                for (variant_index, variant) in item.variants.iter().enumerate() {
                    for (field_index, field) in variant.fields.iter().enumerate() {
                        fields.push((
                            field.ty.clone(),
                            MemoryTypePathElement::EnumVariantField {
                                variant_index: index_u64(variant_index)?,
                                variant: variant.id.bytes(),
                                field_index: index_u64(field_index)?,
                                field: field.id.bytes(),
                            },
                        ));
                    }
                }
                Ok(fields)
            }
        }
    }

    pub(crate) fn verified_recursive_substitutions(
        &self,
        declaration: &VerifiedDeclarationKey,
        root: &VerifiedDeclarationKey,
        arguments: &[Type],
    ) -> Result<HashMap<String, Type>> {
        if declaration != root {
            return Ok(HashMap::new());
        }
        let VerifiedDeclarationKey::Enum(id) = declaration else {
            return Ok(HashMap::new());
        };
        let item = self.enum_definition(*id)?;
        if item.type_parameters.len() != arguments.len() {
            return Err(Error::msg("memory verifier recursive enum arity mismatch"));
        }
        Ok(item
            .type_parameters
            .iter()
            .cloned()
            .zip(arguments.iter().cloned())
            .collect())
    }
}
