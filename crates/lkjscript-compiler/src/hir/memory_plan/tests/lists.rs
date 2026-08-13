use super::super::*;
use super::fixtures::*;
use super::witnesses::witness;
use crate::hir;
use lkjscript_core::Result;

#[test]
fn list_element_eligibility_and_copy_region_selection_are_exact() -> Result<()> {
    for (ty, body, expected) in [
        (
            hir::Type::I64,
            fake(hir::Type::I64),
            MemoryListElementEligibility::Copy,
        ),
        (
            hir::Type::Str,
            text("value"),
            MemoryListElementEligibility::ImmutableValue,
        ),
        (
            hir::Type::Bytes,
            bytes(),
            MemoryListElementEligibility::UnsupportedAffine,
        ),
    ] {
        let plan = derive(&program(ty.clone(), body, Vec::new(), Vec::new()))?;
        assert_eq!(
            witness(&plan, &producer::memory_type(&ty))?
                .facts
                .list_element,
            expected
        );
    }
    let list_ty = hir::Type::List(Box::new(hir::Type::I64));
    let list_plan = derive(&program(
        list_ty.clone(),
        fake(list_ty.clone()),
        Vec::new(),
        Vec::new(),
    ))?;
    let list = witness(&list_plan, &producer::memory_type(&list_ty))?;
    assert_eq!(list.facts.domain, MemoryDomain::OrdinaryRegion);
    assert_eq!(list.facts.copy_share, MemoryCopySharePlan::RegionHandleCopy);
    assert_eq!(list.facts.root_projection, MemoryRootProjection::None);
    assert_eq!(
        list.facts.semantic_snapshot,
        MemorySemanticSnapshotEligibility::Eligible
    );
    let storage =
        list.facts.list.as_ref().ok_or_else(|| {
            lkjscript_core::Error::msg("segmented list storage witness is missing")
        })?;
    assert!(storage.selected);
    assert_eq!(storage.eligibility, MemoryListElementEligibility::Copy);
    assert_eq!(storage.segment_capacity, 32);

    for (element, products, eligibility) in [
        (
            hir::Type::Str,
            Vec::new(),
            MemoryListElementEligibility::ImmutableValue,
        ),
        {
            let product = product(0, "copy-element", &[("value", hir::Type::I64)]);
            (
                hir::Type::Product(product.id),
                vec![product],
                MemoryListElementEligibility::Copy,
            )
        },
    ] {
        let list_ty = hir::Type::List(Box::new(element));
        let plan = derive(&program(
            list_ty.clone(),
            fake(list_ty.clone()),
            products,
            Vec::new(),
        ))?;
        let fact = fact(&plan, &producer::memory_type(&list_ty))?;
        assert_eq!(fact.closure.class, MemoryClosureClass::RegionClosed);
        assert_eq!(fact.copy_share, MemoryCopySharePlan::RegionHandleCopy);
        let witness = witness(&plan, &producer::memory_type(&list_ty))?;
        assert_eq!(witness.facts.domain, MemoryDomain::OrdinaryRegion);
        let list =
            witness.facts.list.as_ref().ok_or_else(|| {
                lkjscript_core::Error::msg("selected owner-list witness is missing")
            })?;
        assert!(list.selected);
        assert_eq!(list.eligibility, eligibility);
    }
    Ok(())
}

#[test]
fn nested_products_close_transitively_over_selected_list_regions() -> Result<()> {
    let list_ty = hir::Type::List(Box::new(hir::Type::I64));
    let inner = product(0, "inner", &[("items", list_ty.clone())]);
    let inner_ty = hir::Type::Product(inner.id);
    let outer = product(
        1,
        "outer",
        &[("inner", inner_ty.clone()), ("flag", hir::Type::Bool)],
    );
    let outer_ty = hir::Type::Product(outer.id);
    let inner_value = product_value(&inner, vec![fake(list_ty)]);
    let outer_value = product_value(&outer, vec![inner_value, fake(hir::Type::Bool)]);
    let plan = derive(&program(
        outer_ty.clone(),
        outer_value,
        vec![inner, outer],
        Vec::new(),
    ))?;
    for ty in [inner_ty, outer_ty] {
        let fact = fact(&plan, &producer::memory_type(&ty))?;
        assert_eq!(fact.closure.class, MemoryClosureClass::RegionClosed);
        assert_eq!(fact.copy_share, MemoryCopySharePlan::RegionHandleCopy);
        assert_eq!(
            witness(&plan, &producer::memory_type(&ty))?
                .facts
                .semantic_snapshot,
            MemorySemanticSnapshotEligibility::Ineligible
        );
    }
    Ok(())
}

#[test]
fn product_of_selected_lists_uses_an_ordinary_region() -> Result<()> {
    let list_ty = hir::Type::List(Box::new(hir::Type::I64));
    let product = product(0, "list-record", &[("items", list_ty.clone())]);
    let ty = hir::Type::Product(product.id);
    let plan = derive(&program(
        ty.clone(),
        product_value(&product, vec![fake(list_ty)]),
        vec![product],
        Vec::new(),
    ))?;
    let fact = fact(&plan, &producer::memory_type(&ty))?;
    assert_eq!(fact.closure.class, MemoryClosureClass::RegionClosed);
    assert_eq!(fact.root_projection, MemoryRootProjection::None);
    assert_eq!(fact.copy_share, MemoryCopySharePlan::RegionHandleCopy);
    let product_witness = witness(&plan, &producer::memory_type(&ty))?;
    assert_eq!(product_witness.facts.domain, MemoryDomain::OrdinaryRegion);
    assert_eq!(
        product_witness.facts.semantic_snapshot,
        MemorySemanticSnapshotEligibility::Ineligible
    );
    let entry = plan
        .entries
        .iter()
        .find(|entry| entry.ty == producer::memory_type(&ty))
        .ok_or_else(|| lkjscript_core::Error::msg("region product entry is missing"))?;
    assert_eq!(entry.mode.domain, MemoryDomain::OrdinaryRegion);
    let destination = plan
        .destinations
        .iter()
        .find(|destination| destination.type_fact == fact.id)
        .ok_or_else(|| lkjscript_core::Error::msg("region product destination is missing"))?;
    assert_eq!(destination.kind, MemoryDestinationKind::OrdinaryRegion);
    assert_eq!(destination.execution, MemoryExecution::Current);
    Ok(())
}
