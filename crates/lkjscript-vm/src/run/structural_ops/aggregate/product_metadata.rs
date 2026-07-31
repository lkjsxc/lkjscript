use super::*;

pub(super) fn product_metadata<'a, J: RuntimeTier>(
    vm: &'a Vm<'_, J>,
    product: ProductId,
) -> Result<&'a lkjscript_core::ProductMetadata> {
    let metadata = vm
        .chunk
        .products()
        .get(product.index())
        .filter(|metadata| metadata.id == product)
        .ok_or_else(|| Error::msg("product metadata index or identity is invalid"))?;
    if metadata.fields.len() > MAX_PRODUCT_FIELDS {
        return Err(Error::msg("product metadata exceeds field limit"));
    }
    Ok(metadata)
}

pub(super) fn product_field_ref<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    index: usize,
) -> Result<ProductFieldRef> {
    let field_ref = vm
        .chunk
        .product_fields()
        .get(index)
        .copied()
        .ok_or_else(|| Error::msg("product field descriptor index out of range"))?;
    let metadata = product_metadata(vm, field_ref.product)?;
    if usize::from(field_ref.field) >= metadata.fields.len() {
        return Err(Error::msg("product field index out of range"));
    }
    Ok(field_ref)
}
