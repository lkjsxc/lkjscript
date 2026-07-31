use crate::memory_plan::{MemoryClosureClass, MemoryType};
use crate::ssa::*;

pub(in crate::ssa) fn lower_region_products(
    plan: &HirMemoryPlan,
    products: &HashMap<String, ProductId>,
) -> Result<Vec<RegionProductMetadata>> {
    let mut output = Vec::new();
    for fact in &plan.type_facts {
        let MemoryType::Product(name) = &fact.ty else {
            continue;
        };
        if fact.closure.class != MemoryClosureClass::RegionClosed {
            continue;
        }
        let product = *products
            .get(name)
            .ok_or_else(|| Error::msg("region product has no SSA ProductId"))?;
        let identity = lkjscript_ir::runtime_product_contract_identity(
            lkjscript_ir::MemoryPlanId::new(plan.id.as_bytes()),
            name,
        )
        .map_err(|error| Error::msg(error.to_string()))?;
        output.push(RegionProductMetadata { product, identity });
    }
    output.sort_by_key(|item| item.product);
    output.dedup_by_key(|item| item.product);
    if output.len() > lkjscript_ir::MAX_REGION_PRODUCTS {
        return Err(Error::msg(
            "region product metadata exceeds bounded maximum",
        ));
    }
    Ok(output)
}
