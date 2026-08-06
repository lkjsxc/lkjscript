use crate::codegen::*;

pub(in crate::codegen) fn add_constant(
    chunk: &mut Chunk,
    constant: BytecodeConstant,
) -> Result<BytecodeConstId> {
    chunk.add_const(constant)
}

pub(in crate::codegen) fn intern_product_field(
    chunk: &mut Chunk,
    product: u16,
    field: u8,
) -> Result<u16> {
    let field_ref = ProductFieldRef {
        product: BytecodeProductId::new(product),
        field,
    };
    if let Some(index) = chunk
        .product_fields
        .iter()
        .position(|existing| *existing == field_ref)
    {
        return u16::try_from(index)
            .map_err(|_| Error::msg("product field descriptor index exceeds u16"));
    }
    let index = u16::try_from(chunk.product_fields.len())
        .map_err(|_| Error::msg("too many product field descriptors"))?;
    chunk.product_fields.push(field_ref);
    Ok(index)
}
