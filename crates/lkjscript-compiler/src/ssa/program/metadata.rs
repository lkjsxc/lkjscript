pub(super) fn lower_product_metadata(
    program: &hir::Program,
    product_ids: &HashMap<String, ProductId>,
) -> Result<Vec<ProductMetadata>> {
    program
        .products
        .iter()
        .map(|product| {
            Ok(ProductMetadata {
                id: ProductId::new(product.id.raw()),
                name: product.name.clone(),
                fields: product
                    .fields
                    .iter()
                    .map(|field| {
                        Ok(ProductField {
                            name: field.name.clone(),
                            ty: lower_type(&field.ty, product_ids)?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            })
        })
        .collect()
}

pub(super) fn lower_trait_metadata(program: &hir::Program) -> Vec<TraitMetadata> {
    program
        .traits
        .iter()
        .map(|definition| TraitMetadata {
            id: TraitId::new(definition.id.raw()),
            name: definition.name.clone(),
            role: match definition.core {
                Some(hir::CoreTrait::Copy) => TraitRole::Copy,
                Some(hir::CoreTrait::Clone) => TraitRole::Clone,
                Some(hir::CoreTrait::Drop) => TraitRole::Drop,
                Some(hir::CoreTrait::Send) => TraitRole::Send,
                Some(hir::CoreTrait::Sync) => TraitRole::Sync,
                None => TraitRole::User,
            },
            source: match definition.origin {
                hir::Origin::Source(source) => Some(source.raw()),
                hir::Origin::Builtin => None,
            },
        })
        .collect()
}

pub(super) fn lower_implementation_metadata(program: &hir::Program) -> Vec<ImplMetadata> {
    program
        .implementations
        .iter()
        .map(|implementation| ImplMetadata {
            id: ImplId::new(implementation.id.raw()),
            trait_id: TraitId::new(implementation.trait_id.raw()),
            product: ProductId::new(implementation.product.raw()),
            source: implementation.origin.raw(),
        })
        .collect()
}
