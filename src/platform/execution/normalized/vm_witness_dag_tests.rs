//! Runtime admission counts shared nodes while checking each retained edge binding.
use super::*;
use crate::platform::execution::normalized::tests::owned_implementation_scheme_tests::prepared_duplicate_dag;

#[test]
fn runtime_mixed_witness_admission_requires_complete_exact_formal_categories() {
    use crate::platform::execution::normalized::tests::mixed_storage_contract_tests::prepared_mixed;
    use crate::platform::kernel::TypeParameterConstraints as C;
    let base = prepared_mixed();
    let control = ExecutionControl::uncancelled();
    for fault in 0..5 {
        let mut program = base.clone();
        let node = &mut Arc::make_mut(&mut program.implementation_applications)[0];
        match fault {
            1 => Arc::make_mut(node).implementation_type_parameter_constraints = Arc::from([]),
            2 => {
                Arc::make_mut(node).implementation_type_parameter_constraints =
                    Arc::from([C::Owned])
            }
            3 => Arc::make_mut(node).contract_type_parameter_constraints = Arc::from([]),
            4 => Arc::make_mut(node).contract_type_parameter_constraints = Arc::from([C::Owned]),
            _ => {}
        }
        let node = &program.implementation_applications[0];
        let result = validate_runtime_implementation(
            &program,
            node,
            &control,
            0,
            &mut 0,
            &mut BTreeSet::new(),
            &mut |_| Ok(()),
        );
        assert_eq!(result.is_ok(), fault == 0, "fault {fault}");
    }
}

#[test]
fn runtime_admission_visits_a_depth_24_binary_dag_linearly() {
    let program = prepared_duplicate_dag(24);
    let root = program.implementation_applications.last().unwrap();
    let control = ExecutionControl::uncancelled();
    let mut nodes = 0;
    let mut visited = BTreeSet::new();
    let mut bytes = 0;
    validate_runtime_implementation(
        &program,
        root,
        &control,
        0,
        &mut nodes,
        &mut visited,
        &mut |added| {
            bytes += added;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(nodes, 49, "each edge is checked; each node unfolds once");
    assert_eq!(visited.len(), 25);
    assert_eq!(
        bytes,
        25 * (std::mem::size_of::<u32>() + 3 * std::mem::size_of::<usize>()) as u64
    );
    // All supplying root parameters share one invocation admission ledger.
    validate_runtime_implementation(
        &program,
        root,
        &control,
        0,
        &mut nodes,
        &mut visited,
        &mut |_| panic!("a repeated root must not grow the unique-node ledger"),
    )
    .unwrap();
    assert_eq!(nodes, 50);
}

#[test]
fn runtime_dag_admission_reserves_before_memo_growth_and_rejects_changed_edges() {
    let program = prepared_duplicate_dag(24);
    let root = program.implementation_applications.last().unwrap();
    let control = ExecutionControl::uncancelled();
    let mut visited = BTreeSet::new();
    let error = validate_runtime_implementation(
        &program,
        root,
        &control,
        0,
        &mut 0,
        &mut visited,
        &mut |_| {
            Err(ExecutionError::resource(
                "test_reservation",
                "refused memo storage",
            ))
        },
    )
    .unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Resource);
    assert!(visited.is_empty());

    let mut changed = root.clone();
    Arc::make_mut(&mut Arc::make_mut(&mut changed).implementations)[1] =
        program.implementation_applications[0].clone();
    assert!(
        validate_runtime_implementation(
            &program,
            &changed,
            &control,
            0,
            &mut 0,
            &mut visited,
            &mut |_| Ok(()),
        )
        .is_err()
    );
    assert!(
        visited.is_empty(),
        "binding refusal precedes memo insertion"
    );
}

#[test]
fn runtime_dag_depth_checks_the_full_shared_suffix_before_memo_skip() {
    let maximum = crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH;
    let program = prepared_duplicate_dag(maximum);
    let root = program.implementation_applications.last().unwrap();
    let control = ExecutionControl::uncancelled();
    let mut visited = BTreeSet::new();
    validate_runtime_implementation(
        &program,
        root,
        &control,
        0,
        &mut 0,
        &mut visited,
        &mut |_| Ok(()),
    )
    .unwrap();
    assert_eq!(visited.len(), maximum + 1);
    // The suffix was visited on a shorter path. A new incoming edge still
    // puts its complete shared suffix one past the depth capacity.
    assert!(
        validate_runtime_implementation(
            &program,
            root,
            &control,
            1,
            &mut 0,
            &mut visited,
            &mut |_| panic!("depth refusal precedes memo allocation"),
        )
        .is_err()
    );
    let mut forged = root.clone();
    Arc::make_mut(&mut forged).depth = 0;
    assert!(
        validate_runtime_implementation(
            &program,
            &forged,
            &control,
            0,
            &mut 0,
            &mut visited,
            &mut |_| Ok(()),
        )
        .is_err()
    );
}

#[test]
fn runtime_witness_handles_reject_equal_copies_and_foreign_preparations_before_memo_skip() {
    let program = prepared_duplicate_dag(24);
    let other = prepared_duplicate_dag(24);
    let root = program.implementation_applications.last().unwrap();
    let control = ExecutionControl::uncancelled();
    let cloned = Arc::clone(root);
    let mut visited = BTreeSet::new();
    validate_runtime_implementation(
        &program,
        &cloned,
        &control,
        0,
        &mut 0,
        &mut visited,
        &mut |_| Ok(()),
    )
    .unwrap();
    for foreign in [
        Arc::new((**root).clone()),
        other.implementation_applications.last().unwrap().clone(),
    ] {
        assert_eq!(foreign.identity, root.identity);
        assert!(!Arc::ptr_eq(&foreign, root));
        let before = visited.clone();
        assert!(
            validate_runtime_implementation(
                &program,
                &foreign,
                &control,
                0,
                &mut 0,
                &mut visited,
                &mut |_| panic!("foreign handle must reject before reservation")
            )
            .is_err()
        );
        assert_eq!(visited, before);
    }
}
