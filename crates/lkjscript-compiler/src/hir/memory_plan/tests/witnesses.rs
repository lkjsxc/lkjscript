use super::super::*;
use super::fixtures::*;
use crate::hir;
use lkjscript_core::Result;

pub(super) fn witness<'a>(plan: &'a HirMemoryPlan, ty: &MemoryType) -> Result<&'a MemoryWitness> {
    let fact = fact(plan, ty)?;
    plan.witness(fact.witness)
        .ok_or_else(|| lkjscript_core::Error::msg("fixture witness must exist"))
}

#[test]
fn exact_witnesses_are_deterministic_one_to_one_and_identity_bearing() -> Result<()> {
    let definition = product(0, "record", &[("value", hir::Type::Str)]);
    let ty = hir::Type::Product(definition.name.clone());
    let hir = program(
        ty.clone(),
        product_value(&definition, vec![text("value")]),
        vec![definition],
        Vec::new(),
    );
    let first = derive(&hir)?;
    let second = derive(&hir)?;
    assert_eq!(
        first.id.to_hex(),
        "934f6110101c73d891e9515da19cba6136ee8455edb32535831be62651e9c737"
    );
    assert_eq!(
        witness(&first, &MemoryType::Product("record".into()))?
            .id
            .to_hex(),
        "4bfdc263598d12baf8b6f2a5f80e9b4783c205f4f63c01478a9e4d152d573e53"
    );
    assert_eq!(first.witnesses, second.witnesses);
    let product_witness = witness(&first, &MemoryType::Product("record".into()))?;
    assert!(product_witness.facts.capabilities.sealed_region);
    assert!(product_witness.facts.capabilities.unique);
    assert_eq!(product_witness.facts.domain, MemoryDomain::UniqueStructural);
    assert_eq!(
        product_witness.facts.copy_share,
        MemoryCopySharePlan::StructuralCopy
    );
    let mut changed = hir.clone();
    changed.products[0].fields[0].ty = hir::Type::Bool;
    let changed = derive(&changed)?;
    assert_ne!(first.id, changed.id);
    assert_ne!(
        witness(&first, &MemoryType::Product("record".into()))?.id,
        witness(&changed, &MemoryType::Product("record".into()))?.id,
        "declaration changes must change witness identity"
    );
    assert_eq!(first.id, second.id);
    let mut work_changed = first.clone();
    work_changed.work.verifier_steps = work_changed.work.verifier_steps.saturating_add(1);
    assert_ne!(first.id, compute_plan_id(&work_changed)?);
    assert_eq!(
        witness(&first, &MemoryType::Product("record".into()))?
            .facts
            .process_codec,
        MemoryProcessCodecEligibility::Eligible
    );
    assert_eq!(first.type_facts.len(), first.witnesses.len());
    assert_eq!(first.work.type_nodes, first.work.witnesses);
    for item in &first.witnesses {
        assert_eq!(item.id, item.recompute_id()?);
    }
    assert!(first
        .type_facts
        .iter()
        .all(|item| first.witness(item.witness).is_some()));
    Ok(())
}

#[test]
fn independent_verifier_rejects_forged_missing_duplicate_and_mismatched_witnesses() -> Result<()> {
    let hir = program(hir::Type::I64, fake(hir::Type::I64), Vec::new(), Vec::new());
    let plan = derive(&hir)?;

    let mut forged = plan.clone();
    forged.witnesses[0].facts.equality = MemoryEqualitySupport::Unsupported;
    assert!(verify_forged(&hir, &mut forged).is_err());

    let mut forged = plan.clone();
    forged.witnesses[0].facts.capabilities.inline = false;
    assert!(verify_forged(&hir, &mut forged).is_err());

    let mut forged = plan.clone();
    forged.witnesses[0].id = MemoryWitnessId::from_bytes([7; 32]);
    forged.type_facts[0].witness = forged.witnesses[0].id;
    assert!(verify_forged(&hir, &mut forged).is_err());

    let mut missing = plan.clone();
    missing.witnesses.clear();
    assert!(verify_forged(&hir, &mut missing).is_err());

    let mut duplicate = plan;
    duplicate.witnesses.push(duplicate.witnesses[0].clone());
    assert!(verify_forged(&hir, &mut duplicate).is_err());
    Ok(())
}

#[test]
fn generic_parameter_requires_specialization_without_claiming_a_runtime_witness() -> Result<()> {
    let ty = hir::Type::Param("t".into());
    let plan = derive(&program(ty.clone(), fake(ty), Vec::new(), Vec::new()))?;
    let witness = witness(&plan, &MemoryType::TypeParameter("t".into()))?;
    assert_eq!(
        witness.facts.requirement,
        MemoryWitnessRequirement::SpecializationRequired
    );
    assert_eq!(witness.facts.closure.class, MemoryClosureClass::Unresolved);
    assert_eq!(
        witness.facts.closure.blocker_reason,
        Some(MemoryBlockerReason::UnknownTypeParameter)
    );
    assert_eq!(witness.facts.domain, MemoryDomain::CallerDestination);
    assert_eq!(
        witness.facts.dynamic_size,
        MemoryDynamicSize::CallerWitnessRequired
    );
    assert_eq!(
        witness.facts.list_element,
        MemoryListElementEligibility::CallerWitnessRequired
    );
    assert_eq!(witness.facts.copy_share, MemoryCopySharePlan::Unsupported);
    Ok(())
}

#[test]
fn option_and_result_witnesses_close_equality_codec_and_element_eligibility() -> Result<()> {
    let mut option = enum_definition(
        10,
        "option",
        &["t"],
        vec![
            ("none", Vec::new()),
            ("some", vec![hir::Type::Param("t".into())]),
        ],
    );
    option.id = hir::EnumId::new(lkjscript_core::OPTION_ID);
    let option_ty = enum_type(&option, vec![hir::Type::I64]);
    let option_plan = derive(&program(
        option_ty.clone(),
        enum_value(&option, 1, vec![hir::Type::I64], vec![fake(hir::Type::I64)]),
        Vec::new(),
        vec![option.clone()],
    ))?;
    let option_memory = producer::memory_type(&option_ty);
    let option_witness = witness(&option_plan, &option_memory)?;
    assert_eq!(
        option_witness.facts.equality,
        MemoryEqualitySupport::EqualValue
    );
    assert_eq!(
        option_witness.facts.process_codec,
        MemoryProcessCodecEligibility::Eligible
    );
    assert_eq!(
        option_witness.facts.list_element,
        MemoryListElementEligibility::Copy
    );

    let mut result = enum_definition(
        20,
        "result",
        &["ok", "error"],
        vec![
            ("ok", vec![hir::Type::Param("ok".into())]),
            ("error", vec![hir::Type::Param("error".into())]),
        ],
    );
    result.id = hir::EnumId::new(lkjscript_core::RESULT_ID);
    let result_ty = enum_type(&result, vec![hir::Type::I64, hir::Type::Str]);
    let result_plan = derive(&program(
        result_ty.clone(),
        enum_value(
            &result,
            0,
            vec![hir::Type::I64, hir::Type::Str],
            vec![fake(hir::Type::I64)],
        ),
        Vec::new(),
        vec![result],
    ))?;
    let result_witness = witness(&result_plan, &producer::memory_type(&result_ty))?;
    assert_eq!(
        result_witness.facts.equality,
        MemoryEqualitySupport::EqualValue
    );
    assert_eq!(
        result_witness.facts.list_element,
        MemoryListElementEligibility::ImmutableValue
    );
    Ok(())
}
