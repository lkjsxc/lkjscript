#![allow(clippy::panic)]

use lkjscript_core::Limits;

#[test]
fn executable_retains_dense_independently_verified_hir_memory_plan() {
    let source = "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n";
    let first = crate::compile_source(source, "memory-plan.lkjscript", &Limits::default());
    let second = crate::compile_source(source, "memory-plan.lkjscript", &Limits::default());
    let (Ok(first), Ok(second)) = (first, second) else {
        panic!("memory-plan fixture must compile");
    };
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
}

#[test]
fn whole_place_drop_classes_are_pre_backend_and_identity_bearing() {
    let static_source = owned_main("unit", "unit");
    let dead_source = owned_main("byte-vector", "move/\nb\n/move");
    let static_program = crate::compile_source(
        &static_source,
        "memory-static-drop.lkjscript",
        &Limits::default(),
    );
    let dead_program = crate::compile_source(
        &dead_source,
        "memory-dead-drop.lkjscript",
        &Limits::default(),
    );
    let (Ok(static_program), Ok(dead_program)) = (static_program, dead_program) else {
        panic!("whole-place drop fixtures must compile");
    };
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
    forged.obligations[0].drop_class = Some(super::MemoryDropClass::Dead);
    let forged_id = super::compute_plan_id(&forged);
    assert!(matches!(forged_id, Ok(id) if id != static_program.memory_plan().id));
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
fn complete_numeric_scalars_are_planned_inline_without_legacy_tracing() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nf64\n/output\n/sig\n",
        "add/\n1.0\nconvert-i64-to-f64-rounded/\n2\n",
        "/convert-i64-to-f64-rounded\n/add\n/main\n",
    );
    let program = crate::compile_source(source, "inline-scalars.lkjscript", &Limits::default());
    let Ok(program) = program else {
        panic!("inline scalar fixture must compile");
    };
    let mut scalar_entries = 0usize;
    for entry in &program.memory_plan().entries {
        if matches!(entry.ty, super::MemoryType::I64 | super::MemoryType::F64) {
            scalar_entries = scalar_entries.saturating_add(1);
            assert_eq!(entry.mode.storage, super::MemoryStorage::Inline);
            assert_eq!(entry.mode.destruction, super::MemoryDestruction::Trivial);
            assert!(entry.legacy_family.is_none());
        }
    }
    assert!(scalar_entries >= 3);
}
