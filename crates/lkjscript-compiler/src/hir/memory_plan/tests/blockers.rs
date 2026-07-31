use super::super::*;
use super::fixtures::*;
use crate::hir;
use lkjscript_core::Result;

#[test]
fn string_path_and_blocker_leaves_are_exact() -> Result<()> {
    for (ty, memory_ty, cutover) in [
        (
            hir::Type::Str,
            MemoryType::String,
            MemoryExecutionCutover::StructuralString,
        ),
        (
            hir::Type::Path,
            MemoryType::Path,
            MemoryExecutionCutover::StructuralPath,
        ),
    ] {
        let program = program(ty.clone(), fake(ty), Vec::new(), Vec::new());
        let plan = derive(&program)?;
        let item = fact(&plan, &memory_ty)?;
        assert_eq!(item.mode, MemoryAggregateMode::ImmutableValue);
        assert_eq!(item.closure.class, MemoryClosureClass::Deterministic);
        assert_eq!(plan.entries[0].execution_cutover, Some(cutover));
        assert!(item.drop_glue.is_some() && item.drop_path.is_some());
    }
    let ty = hir::Type::List(Box::new(hir::Type::Unit));
    let list_plan = derive(&program(ty.clone(), fake(ty), Vec::new(), Vec::new()))?;
    let list = list_plan
        .type_facts
        .last()
        .ok_or_else(|| lkjscript_core::Error::msg("list blocker fact is missing"))?;
    assert_eq!(list.mode, MemoryAggregateMode::ImmutableValue);
    assert_eq!(list.closure.class, MemoryClosureClass::RegionClosed);
    assert_eq!(
        list.closure.blocker_reason,
        Some(MemoryBlockerReason::RegionDomainBoundary)
    );
    assert_eq!(list.root_projection, MemoryRootProjection::None);
    assert_eq!(list.copy_share, MemoryCopySharePlan::RegionHandleCopy);
    let list_witness = list_plan
        .witness(list.witness)
        .ok_or_else(|| lkjscript_core::Error::msg("list witness is missing"))?;
    assert_eq!(list_witness.facts.domain, MemoryDomain::OrdinaryRegion);
    let list_storage = list_witness
        .facts
        .list
        .as_ref()
        .ok_or_else(|| lkjscript_core::Error::msg("segmented list witness is missing"))?;
    assert!(list_storage.selected);
    assert_eq!(list_storage.eligibility, MemoryListElementEligibility::Copy);
    assert_eq!(list_storage.segment_capacity, 32);

    let ty = hir::Type::Param("unknown".into());
    let parameter_plan = derive(&program(ty.clone(), fake(ty), Vec::new(), Vec::new()))?;
    let parameter = parameter_plan
        .type_facts
        .last()
        .ok_or_else(|| lkjscript_core::Error::msg("parameter witness fact is missing"))?;
    assert_eq!(parameter.closure.class, MemoryClosureClass::Unresolved);
    assert_eq!(
        parameter.closure.blocker_reason,
        Some(MemoryBlockerReason::UnknownTypeParameter)
    );
    let witness = parameter_plan
        .witness(parameter.witness)
        .ok_or_else(|| lkjscript_core::Error::msg("parameter witness is missing"))?;
    assert_eq!(
        witness.facts.requirement,
        MemoryWitnessRequirement::SpecializationRequired
    );
    assert_eq!(
        witness.facts.equality,
        MemoryEqualitySupport::CallerWitnessRequired
    );
    Ok(())
}

#[test]
fn products_without_a_structural_or_region_plan_reject_as_unresolved() {
    let blocked = product(0, "blocked", &[("value", hir::Type::Param("t".into()))]);
    let ty = hir::Type::Product(blocked.name.clone());
    let error = derive(&program(ty.clone(), fake(ty), vec![blocked], Vec::new()))
        .err()
        .unwrap_or_else(|| lkjscript_core::Error::msg("blocked product unexpectedly planned"));
    assert!(error.to_string().contains("LKJ-MEM-PRODUCT-UNRESOLVED"));
}

#[test]
fn wrapped_recursive_edges_are_rejected_instead_of_reentering_type_interning() -> Result<()> {
    let wrapped = product(
        0,
        "wrapped-node",
        &[(
            "children",
            hir::Type::List(Box::new(hir::Type::Product("wrapped-node".into()))),
        )],
    );
    let ty = hir::Type::Product(wrapped.name.clone());
    let error = match derive(&program(ty.clone(), fake(ty), vec![wrapped], Vec::new())) {
        Err(error) => error,
        Ok(_) => {
            return Err(lkjscript_core::Error::msg(
                "wrapped recursive type must be rejected",
            ));
        }
    };
    assert!(error.to_string().contains("LKJ-MEM-RECURSIVE-NONREGULAR"));
    Ok(())
}

#[test]
fn recursive_scc_and_both_mixed_bridge_directions_are_exact() -> Result<()> {
    let recursive = product(0, "node", &[("next", hir::Type::Product("node".into()))]);
    let recursive_ty = hir::Type::Product(recursive.name.clone());
    let hir_program = program(
        recursive_ty.clone(),
        fake(recursive_ty),
        vec![recursive.clone()],
        Vec::new(),
    );
    let plan = derive(&hir_program)?;
    let recursive_fact = fact(&plan, &MemoryType::Product(recursive.name))?;
    assert_eq!(recursive_fact.mode, MemoryAggregateMode::ImmutableValue);
    assert_eq!(
        recursive_fact.closure.class,
        MemoryClosureClass::Deterministic
    );
    assert_eq!(recursive_fact.closure.blocker_reason, None);

    let unresolved_mixed = product(
        1,
        "unresolved-mixed",
        &[
            ("next", hir::Type::Product("unresolved-mixed".into())),
            ("bytes", hir::Type::Bytes),
        ],
    );
    let ty = hir::Type::Product(unresolved_mixed.name.clone());
    let error = producer::derive(&program(
        ty.clone(),
        fake(ty),
        vec![unresolved_mixed],
        Vec::new(),
    ))
    .err()
    .map(|error| error.to_string())
    .unwrap_or_default();
    assert!(error.contains("LKJ-MEM-RECURSIVE-AFFINE"));

    let copy_child = product(2, "copy-child", &[("value", hir::Type::I64)]);
    let deterministic_mixed = product(
        3,
        "deterministic-mixed",
        &[
            (
                "unresolved",
                hir::Type::List(Box::new(hir::Type::Product(copy_child.name.clone()))),
            ),
            ("bytes", hir::Type::Bytes),
        ],
    );
    let ty = hir::Type::Product(deterministic_mixed.name.clone());
    let error = producer::derive(&program(
        ty.clone(),
        fake(ty),
        vec![copy_child, deterministic_mixed],
        Vec::new(),
    ))
    .err()
    .map(|error| error.to_string())
    .unwrap_or_default();
    assert!(
        error.contains("DeterministicContainsUnresolved")
            && error.contains("ListElementWitnessRequired")
    );
    Ok(())
}

#[test]
fn declaration_names_never_select_prelude_memory_rules() -> Result<()> {
    for definition in [
        product(0, "option", &[("value", hir::Type::Str)]),
        product(1, "result", &[("value", hir::Type::Str)]),
    ] {
        let ty = hir::Type::Product(definition.name.clone());
        let body = product_value(&definition, vec![text("value")]);
        let plan = derive(&program(ty, body, vec![definition.clone()], Vec::new()))?;
        let item = fact(&plan, &MemoryType::Product(definition.name))?;
        assert_eq!(item.mode, MemoryAggregateMode::ImmutableValue);
        assert_eq!(item.closure.class, MemoryClosureClass::Deterministic);
    }
    Ok(())
}
