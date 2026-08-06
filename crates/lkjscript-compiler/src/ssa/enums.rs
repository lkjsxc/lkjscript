use crate::ssa::*;

pub(in crate::ssa) fn lower_enums(
    definitions: &[hir::EnumDefinition],
    products: &HashMap<String, ProductId>,
) -> Result<Vec<EnumMetadata>> {
    definitions
        .iter()
        .map(|definition| lower_enum(definition, products))
        .collect()
}

fn lower_enum(
    definition: &hir::EnumDefinition,
    products: &HashMap<String, ProductId>,
) -> Result<EnumMetadata> {
    let mut tag_order: Vec<_> = definition
        .variants
        .iter()
        .map(|variant| variant.id)
        .collect();
    tag_order.sort_by_key(|id| id.bytes());
    let variants = definition
        .variants
        .iter()
        .map(|variant| {
            let tag = tag_order
                .iter()
                .position(|id| id == &variant.id)
                .ok_or_else(|| Error::msg("enum physical tag plan lost VariantId"))?;
            Ok(EnumVariantMetadata {
                id: lkjscript_ir::VariantId::new(variant.id.bytes()),
                name: variant.name.clone(),
                source_order: variant.source_order,
                physical_tag: u64::try_from(tag)
                    .map_err(|_| Error::msg("enum physical tag exceeds u64"))?,
                fields: variant
                    .fields
                    .iter()
                    .map(|field| {
                        let ty = lower_type(&field.ty, products)?;
                        Ok(EnumFieldMetadata {
                            id: lkjscript_ir::VariantFieldId::new(field.id.bytes()),
                            name: field.name.clone(),
                            ty,
                            indirect: field.indirect,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(EnumMetadata {
        id: lkjscript_ir::EnumId::new(definition.id.bytes()),
        name: definition.name.clone(),
        type_parameters: definition.type_parameters.clone(),
        variants,
        layout: EnumLayoutFacts {
            identity: lkjscript_ir::RuntimeLayoutId::new(definition.layout.identity.bytes()),
            recursive: definition.layout.recursive,
        },
    })
}
