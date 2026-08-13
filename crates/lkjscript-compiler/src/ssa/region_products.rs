use crate::memory_plan::{MemoryClosureClass, MemoryType};
use crate::ssa::*;

pub(in crate::ssa) fn lower_region_products(
    program: &hir::Program,
    plan: &HirMemoryPlan,
    products: &HashMap<crate::hir::ProductId, ProductId>,
) -> Result<Vec<RegionProductMetadata>> {
    let mut output = Vec::new();
    for fact in &plan.type_facts {
        let MemoryType::Product(id) = &fact.ty else {
            continue;
        };
        if fact.closure.class != MemoryClosureClass::RegionClosed {
            continue;
        }
        let product = *products
            .get(id)
            .ok_or_else(|| Error::msg("region product has no SSA ProductId"))?;
        let definition = program
            .products
            .get(
                id.index()
                    .ok_or_else(|| Error::msg("region product identity exceeds host index"))?,
            )
            .filter(|definition| definition.id == *id)
            .ok_or_else(|| Error::msg("region product declaration is missing"))?;
        let identity = lkjscript_ir::RuntimeLayoutId::new(definition.identity);
        output.push(RegionProductMetadata { product, identity });
    }
    output.sort_by_key(|item| item.product);
    output.dedup_by_key(|item| item.product);
    Ok(output)
}
