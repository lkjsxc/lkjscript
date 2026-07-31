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
    for (ty, reason) in [
        (
            hir::Type::List(Box::new(hir::Type::Unit)),
            MemoryBlockerReason::ListPair,
        ),
        (
            hir::Type::Param("unknown".into()),
            MemoryBlockerReason::UnknownTypeParameter,
        ),
    ] {
        let program = program(ty.clone(), fake(ty), Vec::new(), Vec::new());
        let plan = derive(&program)?;
        let item = plan
            .type_facts
            .last()
            .ok_or_else(|| lkjscript_core::Error::msg("blocker fact is missing"))?;
        assert_eq!(item.closure.class, MemoryClosureClass::LegacyClosed);
        assert_eq!(item.closure.blocker_reason, Some(reason));
    }
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
    let closure = &fact(&plan, &MemoryType::Product(recursive.name))?.closure;
    assert_eq!(closure.class, MemoryClosureClass::LegacyClosed);
    assert_eq!(
        closure.blocker_reason,
        Some(MemoryBlockerReason::RecursiveDeclarationScc)
    );

    let legacy_mixed = product(
        1,
        "legacy-mixed",
        &[
            ("next", hir::Type::Product("legacy-mixed".into())),
            ("bytes", hir::Type::Bytes),
        ],
    );
    let ty = hir::Type::Product(legacy_mixed.name.clone());
    let error = producer::derive(&program(
        ty.clone(),
        fake(ty),
        vec![legacy_mixed],
        Vec::new(),
    ))
    .err()
    .map(|error| error.to_string())
    .unwrap_or_default();
    assert!(error.contains("LegacyContainsDeterministic") && error.contains("bytes"));

    let deterministic_mixed = product(
        2,
        "deterministic-mixed",
        &[
            ("legacy", hir::Type::List(Box::new(hir::Type::Unit))),
            ("bytes", hir::Type::Bytes),
        ],
    );
    let ty = hir::Type::Product(deterministic_mixed.name.clone());
    let error = producer::derive(&program(
        ty.clone(),
        fake(ty),
        vec![deterministic_mixed],
        Vec::new(),
    ))
    .err()
    .map(|error| error.to_string())
    .unwrap_or_default();
    assert!(error.contains("DeterministicContainsLegacy") && error.contains("ListPair"));
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
