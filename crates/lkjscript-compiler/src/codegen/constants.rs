use crate::codegen::*;

pub(in crate::codegen) fn add_constant(
    chunk: &mut Chunk,
    constant: BytecodeConstant,
) -> Result<BytecodeConstId> {
    chunk.add_const(constant)
}

pub(in crate::codegen) fn intern_product_field(
    chunk: &mut Chunk,
    product: u64,
    field: u64,
) -> Result<u64> {
    let field_ref = ProductFieldRef {
        product: BytecodeProductId::new(product),
        field,
    };
    chunk.intern_product_field(field_ref)
}
