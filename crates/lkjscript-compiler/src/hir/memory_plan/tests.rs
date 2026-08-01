use lkjscript_core::{Limits, Result};

mod blockers;
mod bounds;
mod call_fixture;
mod derivation;
mod fixtures;
mod lists;
mod moves;
mod nested_lists;
mod signatures;
mod verifier;
mod witnesses;

#[test]
fn executable_retains_dense_independently_verified_hir_memory_plan() -> Result<()> {
    let source = "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n";
    let first = crate::compile_source(source, "memory-plan.lkjscript", &Limits::default())?;
    let second = crate::compile_source(source, "memory-plan.lkjscript", &Limits::default())?;
    let plan = first.memory_plan();
    assert_eq!(plan.schema, super::HIR_MEMORY_PLAN_SCHEMA);
    assert_eq!(plan.id, second.memory_plan().id);
    assert_eq!(plan.functions.len(), 1);
    assert_eq!(plan.work.expressions, 1);
    assert!(plan.work.verifier_steps > 0);
    assert!(plan
        .entries
        .iter()
        .enumerate()
        .all(|(index, entry)| entry.id.raw() == u32::try_from(index).unwrap_or(u32::MAX)));
    Ok(())
}

#[test]
fn whole_place_drop_classes_are_pre_backend_and_identity_bearing() -> Result<()> {
    let static_program = crate::compile_source(
        &owned_main("unit", "unit"),
        "memory-static-drop.lkjscript",
        &Limits::default(),
    )?;
    let dead_program = crate::compile_source(
        &owned_main("byte-vector", "move/\nb\n/move"),
        "memory-dead-drop.lkjscript",
        &Limits::default(),
    )?;
    let static_obligation = static_program
        .memory_plan()
        .obligations
        .iter()
        .find(|item| item.drop_class.is_some());
    let dead_obligation = dead_program
        .memory_plan()
        .obligations
        .iter()
        .find(|item| item.drop_class.is_some());
    assert_eq!(
        static_obligation.and_then(|item| item.drop_class),
        Some(super::MemoryDropClass::Static)
    );
    assert_eq!(
        dead_obligation.and_then(|item| item.drop_class),
        Some(super::MemoryDropClass::Dead)
    );

    let mut forged = static_program.memory_plan().clone();
    if let Some(obligation) = forged.obligations.first_mut() {
        obligation.drop_class = Some(super::MemoryDropClass::Dead);
    }
    assert_ne!(
        super::compute_plan_id(&forged)?,
        static_program.memory_plan().id
    );
    Ok(())
}

fn owned_main(result: &str, body: &str) -> String {
    format!(
        concat!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\n{result}\n/output\n/sig\n",
            "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\n",
            "{body}\n/let\n/main\n"
        ),
        result = result,
        body = body
    )
}

#[test]
fn deterministic_byte_vector_uses_only_unique_structural_storage() -> Result<()> {
    let program = crate::compile_source(
        &owned_main("byte-vector", "move/\nb\n/move"),
        "unique-byte-vector-plan.lkjscript",
        &Limits::default(),
    )?;
    let entries: Vec<_> = program
        .memory_plan()
        .entries
        .iter()
        .filter(|entry| matches!(entry.ty, super::MemoryType::ByteVector))
        .collect();
    assert!(!entries.is_empty());
    for entry in entries {
        assert_eq!(entry.mode.multiplicity, super::MemoryMultiplicity::Affine);
        assert_eq!(entry.mode.aliasing, super::MemoryAliasing::Unique);
        assert_eq!(entry.mode.domain, super::MemoryDomain::UniqueStructural);
        assert_eq!(entry.mode.destruction, super::MemoryDestruction::DropGlue);
        assert_eq!(entry.drop_glue.map(|glue| glue.raw()), Some(0));
    }
    Ok(())
}

#[test]
fn complete_numeric_scalars_are_planned_inline() -> Result<()> {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nf64\n/output\n/sig\n",
        "add/\n1.0\nconvert-i64-to-f64-rounded/\n2\n",
        "/convert-i64-to-f64-rounded\n/add\n/main\n"
    );
    let program = crate::compile_source(source, "inline-scalars.lkjscript", &Limits::default())?;
    let mut scalar_entries = 0usize;
    for entry in &program.memory_plan().entries {
        if matches!(entry.ty, super::MemoryType::I64 | super::MemoryType::F64) {
            scalar_entries = scalar_entries.saturating_add(1);
            assert_eq!(entry.mode.domain, super::MemoryDomain::Inline);
            assert_eq!(entry.mode.destruction, super::MemoryDestruction::Trivial);
        }
    }
    assert!(scalar_entries >= 3);
    Ok(())
}
