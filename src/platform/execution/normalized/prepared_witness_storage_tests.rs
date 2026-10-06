//! Matched concrete closure measurements, independent of logical path unfolding.
use super::*;
use crate::platform::execution::normalized::tests::owned_implementation_scheme_tests::duplicate_dag_fixture;
use std::collections::BTreeSet;
use std::time::Instant;

fn retained_slots(program: &NormalizedProgram) -> usize {
    let mut arrays = BTreeSet::new();
    let mut slots = program.implementation_applications.len();
    for items in program
        .functions
        .iter()
        .map(|function| &function.implementation_arguments)
        .chain(
            program
                .implementation_applications
                .iter()
                .map(|node| &node.implementations),
        )
    {
        if !items.is_empty() && arrays.insert(items.as_ptr() as usize) {
            slots += items.len();
        }
    }
    slots
}

#[test]
fn concrete_witness_storage_matched_depths_preserve_exact_dispatch() {
    for depth in [0, 1, 4, 8, 12, 24] {
        let start = Instant::now();
        let (source, program) = duplicate_dag_fixture(depth);
        let elapsed = start.elapsed().as_nanos();
        let both = named(&source, "Both");
        assert_eq!(
            program
                .implementation_applications
                .iter()
                .filter(|node| node.implementation == both)
                .count(),
            depth
        );
        let slots = retained_slots(&program);
        let unique_node_bytes = program.implementation_applications.len()
            * (std::mem::size_of::<NormalizedImplementationApplication>()
                + 2 * std::mem::size_of::<usize>());
        for witness in program
            .functions
            .iter()
            .flat_map(|f| f.implementation_arguments.iter())
            .chain(
                program
                    .implementation_applications
                    .iter()
                    .flat_map(|node| node.implementations.iter()),
            )
        {
            assert!(Arc::ptr_eq(
                witness,
                &program.implementation_applications[witness.identity as usize]
            ));
        }

        println!(
            "witness-storage {}",
            serde_json::json!({
                "depth": depth, "source_and_preparation_ns": elapsed,
                "steps": program.work.type_derivation_steps,
                "reserved_bytes": program.work.type_metadata_bytes,
                "unique_nodes": program.implementation_applications.len(),
                "retained_slots": slots,
                "slot_bytes": std::mem::size_of::<NormalizedImplementationArgument>(),
                "node_and_slot_bytes": unique_node_bytes + slots * std::mem::size_of::<NormalizedImplementationArgument>(),
            })
        );
        let declaration = named(&source, "dag-main");
        let control = crate::platform::execution::ExecutionControl::uncancelled();
        let policy = crate::platform::execution::normalized::NormalizedRunPolicy::foreground();
        let (value, work) =
            crate::platform::execution::normalized::vm::NormalizedVm::for_test(&program, policy)
                .invoke(declaration, Vec::new(), None, &control)
                .unwrap();
        assert_eq!(
            value,
            crate::platform::execution::normalized::value::NormalizedValue::I64(17)
        );
        assert_eq!(
            (work.live_call_frames_after, work.live_handles_after),
            (0, 0)
        );
        let (value, work) =
            crate::platform::execution::normalized::reference::NormalizedReferenceInterpreter::new(
                &source, &program, policy,
            )
            .invoke(declaration, Vec::new(), None, &control)
            .unwrap();
        assert_eq!(
            value,
            crate::platform::execution::normalized::value::NormalizedValue::I64(17)
        );
        assert_eq!(
            (work.live_call_frames_after, work.live_handles_after),
            (0, 0)
        );
    }
}

fn named(source: &crate::platform::kernel::KernelSnapshot, name: &str) -> DeclarationReference {
    source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (
                OwnerKey::Declaration(declaration),
                crate::platform::kernel::OwnerRecord::Declaration(record),
            ) if record.name.as_str() == name => Some(DeclarationReference {
                package: source.root.package_id,
                declaration: *declaration,
            }),
            _ => None,
        })
        .unwrap()
}
