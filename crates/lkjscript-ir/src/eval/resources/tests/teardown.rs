use super::*;
use crate::eval::{as_f64_exact, as_i64, value_equal, EvalValue, Flow};

#[test]
fn trap_and_exit_preserve_primary_outcomes_during_reverse_emergency_cleanup() {
    for (index, primary) in [
        EvalOutcome::Trapped("primary trap".into()),
        EvalOutcome::Exited(7),
    ]
    .into_iter()
    .enumerate()
    {
        let mut resources = session(30 + u64::try_from(index).expect("small test index"));
        resources
            .acquire_owned(ResourceKind::FileReader, true)
            .expect("acquire reader");
        let connection = resources
            .acquire_owned(ResourceKind::SqliteConnection, true)
            .expect("acquire connection");
        resources
            .prepare_statement(&connection, true)
            .expect("prepare statement");

        let (outcome, teardown) = finish_evaluation_with_report(&mut resources, primary);
        match index {
            0 => assert_eq!(outcome, EvalOutcome::Trapped("primary trap".into())),
            _ => assert_eq!(outcome, EvalOutcome::Exited(7)),
        }
        assert_eq!(teardown.ordinary_obligations, 3);
        assert_eq!(teardown.emergency_obligations.len(), 3);
        assert_eq!(teardown.cleanup_attempts.len(), 3);
        let actual_order: Vec<_> = teardown
            .cleanup_attempts
            .iter()
            .map(|attempt| {
                attempt
                    .resource
                    .kind()
                    .expect("cleanup attempt retains exact kind")
            })
            .collect();
        assert_eq!(
            actual_order,
            vec![
                ResourceKind::SqliteStatement,
                ResourceKind::SqliteConnection,
                ResourceKind::FileReader,
            ]
        );
        let emergency_slots: Vec<_> = teardown
            .emergency_obligations
            .iter()
            .map(|observation| observation.kind().expect("emergency kind"))
            .collect();
        assert_eq!(emergency_slots, actual_order);
        assert_eq!(resources.metrics.ordinary_obligations, 3);
        assert_eq!(resources.metrics.emergency_obligations, 3);
        assert_eq!(resources.metrics.cleanup_attempts, 3);
        assert_eq!(resources.metrics.resources_closed, 3);
        assert_eq!(teardown.remaining.ordinary_obligations(), 0);
        assert!(teardown.cleanup_failures.is_empty());
        assert!(teardown
            .cleanup_attempts
            .iter()
            .all(|attempt| attempt.owner.is_some() && attempt.error.is_none()));
    }
}

#[test]
fn cleanup_failures_are_ordered_bounded_and_attached_to_primary() {
    let limits = lkjscript_core::CleanupFailureLimits::new(1, 5).expect("test limits");
    let mut resources = EvalResources::with_scope_and_cleanup_limits(4, scope(32), limits)
        .expect("resource session");
    let reader = resources
        .acquire_owned(ResourceKind::FileReader, true)
        .expect("reader");
    let writer = resources
        .acquire_owned(ResourceKind::FileWriter, true)
        .expect("writer");
    for resource in [&reader, &writer] {
        resources
            .table
            .owned_mut(
                &resource.key,
                resource.kind,
                resource.provider,
                resource.scope,
            )
            .expect("owner")
            .kind = ResourceKind::TerminalSession;
    }

    let primary = EvalOutcome::Trapped("primary".into());
    let (outcome, teardown) = finish_evaluation_with_report(&mut resources, primary);
    assert_eq!(outcome.primary(), &EvalOutcome::Trapped("primary".into()));
    let failures = outcome.cleanup_failures().expect("attached failures");
    assert_eq!(failures.retained().len(), 1);
    assert_eq!(
        failures.retained()[0].subject(),
        lkjscript_core::CleanupSubject::Resource(ResourceKind::FileWriter)
    );
    assert_eq!(failures.retained()[0].message().len(), 5);
    assert_eq!(failures.omitted_failures(), 1);
    assert!(failures.omitted_message_bytes() > 0);
    assert_eq!(teardown.cleanup_attempts.len(), 2);
    assert_eq!(teardown.remaining.ordinary_obligations(), 0);
}

#[test]
fn successful_teardown_reports_unmet_ordinary_invariant_separately() {
    let mut resources = session(32);
    resources
        .acquire_owned(ResourceKind::TerminalSession, true)
        .expect("acquire terminal session");
    let primary = EvalOutcome::Returned(EvalValue::Unit);
    let (outcome, teardown) = finish_evaluation_with_report(&mut resources, primary);
    assert_eq!(outcome, EvalOutcome::Returned(EvalValue::Unit));
    assert_eq!(teardown.ordinary_obligations, 1);
    assert_eq!(teardown.emergency_obligations.len(), 1);
    assert_eq!(teardown.cleanup_attempts.len(), 1);
    assert_eq!(teardown.remaining.ordinary_obligations(), 0);
}

#[test]
fn resource_values_are_not_numbers_or_value_comparable() {
    let resources = session(33);
    let input = EvalValue::Resource(
        resources
            .standard_input
            .clone()
            .expect("borrowed standard input"),
    );
    assert!(matches!(as_i64(&input), Err(Flow::Trap(_))));
    assert!(matches!(as_f64_exact(&input), Err(Flow::Trap(_))));
    assert!(matches!(
        value_equal(&input, &input),
        Err(Flow::Trap(message)) if message == "typed resources cannot be compared as values"
    ));
    assert_ne!(input, EvalValue::I64(0));
    assert_ne!(input, EvalValue::F64(0.0));
}
