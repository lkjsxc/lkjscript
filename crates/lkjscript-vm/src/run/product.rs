use super::*;
fn product_metadata<'a, J: RuntimeTier>(
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

fn product_field_ref<J: RuntimeTier>(vm: &Vm<'_, J>, index: usize) -> Result<ProductFieldRef> {
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

fn make_product<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let product = ProductId::new(vm.read_u16()?);
    let field_count = product_metadata(vm, product)?.fields.len();
    let mut fields = Vec::with_capacity(field_count);
    for _ in 0..field_count {
        fields.push(vm.pop()?);
    }
    fields.reverse();
    if fields.iter().copied().any(is_structural_runtime_value) {
        return Err(Error::msg(
            "legacy traced product cannot contain a structural runtime value",
        ));
    }
    let value = vm.arena.alloc(HeapObj::Product { product, fields })?;
    vm.push(value);
    Ok(())
}

fn load_product_field<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let descriptor = vm.read_u16()? as usize;
    let field_ref = product_field_ref(vm, descriptor)?;
    let value = vm.pop()?;
    if value.as_legacy_traced().is_none() {
        return Err(Error::msg("product field access expects Product"));
    }
    let field = match vm.arena.get(value)? {
        HeapObj::Product { product, fields } if *product == field_ref.product => fields
            .get(usize::from(field_ref.field))
            .copied()
            .ok_or_else(|| Error::msg("product value field count does not match metadata"))?,
        HeapObj::Product { .. } => {
            return Err(Error::msg("product field access identity mismatch"));
        }
        _ => return Err(Error::msg("product field access expects Product")),
    };
    vm.push(field);
    Ok(())
}

fn with_product_field<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let descriptor = vm.read_u16()? as usize;
    let field_ref = product_field_ref(vm, descriptor)?;
    let replacement = vm.pop()?;
    if is_structural_runtime_value(replacement) {
        return Err(Error::msg(
            "legacy traced product cannot contain a structural runtime value",
        ));
    }
    let value = vm.pop()?;
    if value.as_legacy_traced().is_none() {
        return Err(Error::msg("product field replacement expects Product"));
    }
    let mut fields = match vm.arena.get(value)? {
        HeapObj::Product { product, fields } if *product == field_ref.product => fields.clone(),
        HeapObj::Product { .. } => {
            return Err(Error::msg("product field replacement identity mismatch"));
        }
        _ => return Err(Error::msg("product field replacement expects Product")),
    };
    let field = fields
        .get_mut(usize::from(field_ref.field))
        .ok_or_else(|| Error::msg("product value field count does not match metadata"))?;
    *field = replacement;
    let updated = vm.arena.alloc(HeapObj::Product {
        product: field_ref.product,
        fields,
    })?;
    vm.push(updated);
    Ok(())
}

use lkjscript_core::Op;

pub(super) fn handles(op: u8) -> bool {
    op == Op::MakeProduct as u8
        || op == Op::LoadProductField as u8
        || op == Op::WithProductField as u8
}

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<()> {
    match op {
        x if x == Op::MakeProduct as u8 => make_product(vm),
        x if x == Op::LoadProductField as u8 => load_product_field(vm),
        x if x == Op::WithProductField as u8 => with_product_field(vm),
        _ => unreachable!("opcode family checked"),
    }
}
