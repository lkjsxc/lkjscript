mod common;

use std::collections::BTreeMap;

use common::*;
use lkjscript_resource::*;

#[test]
fn generation_ids_reject_stale_and_content_ids_are_stable() -> ResourceResult<()> {
    let mut table = GenerationTable::<TaskId>::new(1);
    let first = table.allocate()?;
    assert!(table.contains(first));
    table.release(first)?;
    assert!(!table.contains(first));
    assert_eq!(
        table.release(first).map_err(|error| error.code),
        Err("stale-id")
    );
    let second = table.allocate()?;
    assert_ne!(first, second);
    assert_eq!(TaskClassId::from_name("x"), TaskClassId::from_name("x"));
    assert_ne!(
        ResourcePlaneId::from_content(b"a"),
        ResourcePlaneId::from_content(b"b")
    );
    let a = graph(&[vec![], vec![id(0)]])?;
    let mut builder = TaskGraphBuilder::new();
    builder.add_scope(TaskScope {
        id: scope(0),
        parent: None,
    })?;
    builder
        .add_task(node(1, vec![id(0)]))?
        .add_task(node(0, vec![]))?;
    let b = TaskGraphVerifier::verify(builder.build(), GraphLimits::default())?;
    assert_eq!(a.id(), b.id());
    Ok(())
}

#[test]
fn verifier_accepts_chain_and_diamond_and_rejects_cycle() -> ResourceResult<()> {
    assert_eq!(
        graph(&[vec![], vec![id(0)], vec![id(0)], vec![id(1), id(2)]])?
            .tasks()
            .len(),
        4
    );
    let mut builder = TaskGraphBuilder::new();
    builder.add_scope(TaskScope {
        id: scope(0),
        parent: None,
    })?;
    builder
        .add_task(node(0, vec![id(1)]))?
        .add_task(node(1, vec![id(0)]))?;
    assert_eq!(
        TaskGraphVerifier::verify(builder.build(), GraphLimits::default())
            .map_err(|error| error.code),
        Err("dependency-cycle")
    );
    Ok(())
}

#[test]
fn verifier_checks_conflicts_moves_scopes_and_budgets() -> ResourceResult<()> {
    let mut conflict_a = node(0, vec![]);
    conflict_a.accesses.push(AccessRecord {
        id: AccessRecordId::new(10, 1),
        owner: owner(9),
        mode: AccessMode::Write,
        range: Some(CheckedRange::new(0, 8)?),
    });
    let mut conflict_b = node(1, vec![]);
    conflict_b.accesses.push(AccessRecord {
        id: AccessRecordId::new(11, 1),
        owner: owner(9),
        mode: AccessMode::Read,
        range: Some(CheckedRange::new(4, 12)?),
    });
    let mut builder = TaskGraphBuilder::new();
    builder
        .add_scope(TaskScope {
            id: scope(0),
            parent: None,
        })?
        .add_task(conflict_a)?
        .add_task(conflict_b)?;
    assert_eq!(
        TaskGraphVerifier::verify(builder.build(), GraphLimits::default())
            .map_err(|error| error.code),
        Err("access-conflict")
    );

    let mut moved_a = node(0, vec![]);
    moved_a.accesses.push(AccessRecord {
        id: AccessRecordId::new(20, 1),
        owner: owner(7),
        mode: AccessMode::Consume,
        range: None,
    });
    let mut moved_b = node(1, vec![id(0)]);
    moved_b.accesses.push(AccessRecord {
        id: AccessRecordId::new(21, 1),
        owner: owner(7),
        mode: AccessMode::Consume,
        range: None,
    });
    let mut builder = TaskGraphBuilder::new();
    builder
        .add_scope(TaskScope {
            id: scope(0),
            parent: None,
        })?
        .add_task(moved_a)?
        .add_task(moved_b)?;
    assert_eq!(
        TaskGraphVerifier::verify(builder.build(), GraphLimits::default())
            .map_err(|error| error.code),
        Err("owner-consume")
    );

    let mut builder = TaskGraphBuilder::new();
    builder
        .add_scope(TaskScope {
            id: scope(0),
            parent: None,
        })?
        .add_scope(TaskScope {
            id: scope(1),
            parent: Some(scope(0)),
        })?;
    let mut child = node(0, vec![]);
    child.scope = scope(1);
    let root = node(1, vec![id(0)]);
    builder.add_task(child)?.add_task(root)?;
    assert_eq!(
        TaskGraphVerifier::verify(builder.build(), GraphLimits::default())
            .map_err(|error| error.code),
        Err("scope-containment")
    );

    let bounded = graph(&[vec![]])?;
    let raw = UnverifiedTaskGraph {
        scopes: bounded.scopes().to_vec(),
        tasks: bounded.tasks().to_vec(),
    };
    let limits = GraphLimits {
        max_tasks: 1,
        max_dependencies: 1,
        max_accesses: 1,
        max_compute_units: 0,
        max_scratch_bytes: 8,
    };
    assert_eq!(
        TaskGraphVerifier::verify(raw, limits).map_err(|error| error.code),
        Err("graph-ceiling")
    );
    Ok(())
}

fn run_policy<P: SchedulingPolicy>(graph: &VerifiedTaskGraph, policy: &P) -> ResourceResult<()> {
    let report = ReferenceScheduler::run(graph, policy, 3, &BTreeMap::new(), 100)?;
    assert!(report
        .states
        .values()
        .all(|state| *state == TaskState::Retired));
    ReferenceScheduler::replay(graph, policy, 3, &BTreeMap::new(), &report.trace)
}

#[test]
fn all_policies_complete_and_replay_detects_tampering() -> ResourceResult<()> {
    let dag = graph(&[vec![], vec![id(0)], vec![id(0)], vec![id(1), id(2)]])?;
    run_policy(&dag, &Sequential)?;
    run_policy(&dag, &StaticPartition)?;
    run_policy(&dag, &GlobalFifo)?;
    run_policy(&dag, &LocalWorkStealing)?;
    run_policy(&dag, &HierarchicalLocality)?;
    run_policy(&dag, &OwnerCompute)?;
    let mut trace = ReferenceScheduler::run(&dag, &Sequential, 2, &BTreeMap::new(), 100)?.trace;
    trace.events[0].task = id(3);
    assert_eq!(
        ReferenceScheduler::replay(&dag, &Sequential, 2, &BTreeMap::new(), &trace)
            .map_err(|error| error.code),
        Err("replay-mismatch")
    );
    Ok(())
}

#[test]
fn stable_failure_is_lowest_ready_task() -> ResourceResult<()> {
    let dag = graph(&[vec![], vec![], vec![id(0), id(1)]])?;
    let failures = BTreeMap::from([(id(1), "later".to_owned()), (id(0), "first".to_owned())]);
    let report = ReferenceScheduler::run(&dag, &OwnerCompute, 2, &failures, 100)?;
    assert_eq!(report.failure, Some((id(0), "first".to_owned())));
    assert!(report
        .states
        .values()
        .all(|state| *state == TaskState::Retired));
    Ok(())
}
