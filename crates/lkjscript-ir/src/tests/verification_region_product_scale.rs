use super::fixtures::one_block_program;
use crate::{
    runtime_product_contract_identity, verify, ProductField, ProductId, ProductMetadata,
    RegionProductMetadata,
};

#[test]
fn region_product_metadata_crosses_the_former_sixteen_thousand_limit() -> crate::Result<()> {
    const PRODUCTS: u64 = 16_385;
    let mut program = one_block_program();
    program.memory.plan = crate::MemoryPlanId::new([1; 32]);
    program
        .products
        .try_reserve(
            usize::try_from(PRODUCTS)
                .map_err(|_| crate::IrError::new("region-product test count exceeds host usize"))?,
        )
        .map_err(|_| crate::IrError::new("region-product test allocation failed"))?;
    program
        .region_products
        .try_reserve(
            usize::try_from(PRODUCTS)
                .map_err(|_| crate::IrError::new("region-product test count exceeds host usize"))?,
        )
        .map_err(|_| crate::IrError::new("region-product test allocation failed"))?;
    for raw in 0..PRODUCTS {
        let product = ProductId::new(raw);
        let name = format!("region-product-{raw}");
        let identity = runtime_product_contract_identity(program.memory.plan, &name)?;
        program.products.push(ProductMetadata {
            id: product,
            identity,
            name,
            fields: Vec::<ProductField>::new(),
        });
        program
            .region_products
            .push(RegionProductMetadata { product, identity });
    }
    let verified = verify(program)?;
    assert_eq!(verified.program().region_products.len(), 16_385);
    Ok(())
}
