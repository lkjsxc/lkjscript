//! Reused source preparation still executes independent dynamic rows and joins failed owners.

use super::*;
use crate::platform::normalized_lifecycle::{PreparedApplication, prepare_repository};
use crate::platform::publication::GraphRepository;
use serde_json::{Value, json};
use std::sync::OnceLock;

fn authored_fixture() -> (tempfile::TempDir, PreparedApplication) {
    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("project"),
        &crate::platform::kernel::tests::transport_snapshot(),
        None,
    )
    .unwrap();
    let source = format!(
        r#"request base={}
declarations.begin
(units
  (module create source-batch-controls
    (external create add (visibility private) (implementation core.i64.add) (returns I64)
      (parameter create left (type I64)) (parameter create right (type I64)))
    (function create plus-forty-two (visibility private) (effect pure)
      (parameter create value (type I64)) (returns I64)
      (body (call add (local value) (i64 42))))
    (function create task (visibility private) (effect (task)) (returns Unit) (body (unit)))
    (component create command (visibility private)
      (port create pure (type (function (I64) I64)) (function plus-forty-two))
      (port create task (type (task-function () Unit (row))) (function task))))
  (target create pure (component source-batch-controls::command) (runner command) (port source-batch-controls::command::pure))
  (target create task (component source-batch-controls::command) (runner command) (port source-batch-controls::command::task)))
declarations.end
"#,
        created.current.head.revision
    );
    let request = crate::platform::control::decode_compact_change(
        "source-batch-controls.lkjc",
        source.as_bytes(),
    )
    .unwrap();
    let prepared = created
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
    created.repository.publish(&prepared.publication).unwrap();
    let application = prepare_repository(created.repository).unwrap();
    (temporary, application)
}

fn fixture() -> &'static PreparedApplication {
    static FIXTURE: OnceLock<(tempfile::TempDir, PreparedApplication)> = OnceLock::new();
    &FIXTURE.get_or_init(authored_fixture).1
}

fn quiescent(value: &Value, fields: &[&str]) {
    for field in fields {
        assert_eq!(value[*field], 0, "unreleased {field}: {value}");
    }
}

fn production_quiescent(value: &Value) {
    quiescent(
        value,
        &[
            "live_call_frames_after",
            "live_locals_after",
            "live_type_bindings_after",
            "live_operands_after",
            "live_transactions_after",
            "live_handles_after",
        ],
    );
    assert_eq!(value["capability_calls"], 0);
}

fn reference_quiescent(value: &Value) {
    quiescent(
        value,
        &[
            "live_call_frames_after",
            "live_control_frames_after",
            "live_local_scopes_after",
            "live_type_scopes_after",
            "live_effect_scopes_after",
            "live_allowances_after",
            "live_transactions_after",
            "live_handles_after",
        ],
    );
    assert_eq!(value["capability_calls"], 0);
}

fn executor_quiescent(value: &Value) {
    assert_eq!(value["dispatch_open"], false);
    quiescent(value, &["active_dispatches", "remaining_workers"]);
}

fn healthy(application: &PreparedApplication, input: i64, expected: i64) {
    let receipt = application
        .run_observed(
            &Name::new("pure").unwrap(),
            json!([input]).to_string().as_bytes(),
            NormalizedCommandPolicy::default(),
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&receipt.command.result_json).unwrap(),
        json!(expected)
    );
    assert_eq!(receipt.command.differential, "equal");
    assert!(receipt.command.production.instructions > 0);
    assert!(receipt.command.reference.expressions > 0);
    production_quiescent(&json!(receipt.command.production));
    reference_quiescent(&json!(receipt.command.reference));
    executor_quiescent(&json!(receipt.executor));
}

#[test]
fn reused_source_preparation_retains_independent_dynamic_results_and_fresh_state() {
    let application = fixture();
    let before = application.repository.view_current().unwrap().revision();
    for (input, expected) in [(-7, 35), (0, 42), (8, 50), (-7, 35)] {
        healthy(application, input, expected);
    }
    assert_eq!(
        application.repository.view_current().unwrap().revision(),
        before
    );
    application.check_authority().unwrap();
}

#[test]
fn reused_source_rows_refuse_empty_tasks_and_join_quota_failure_before_recovery() {
    let application = fixture();
    let error = require_pure_artifact_target(&application.program, &Name::new("task").unwrap())
        .unwrap_err();
    assert_eq!(error.code, "normalized_runner_grants_required");
    let mut policy = NormalizedCommandPolicy::default();
    policy.execution.instruction_steps = Some(1);
    let error = application
        .run_observed(
            &Name::new("pure").unwrap(),
            b"[8]",
            policy,
            &ExecutionControl::uncancelled(),
        )
        .err()
        .unwrap();
    assert_eq!(error.code, "normalized_instruction_steps");
    let observation = |label: &str| {
        let prefix = format!("{label}: ");
        serde_json::from_str::<Value>(
            error
                .notes
                .iter()
                .find_map(|note| note.strip_prefix(&prefix))
                .unwrap(),
        )
        .unwrap()
    };
    production_quiescent(&observation("production-observation"));
    executor_quiescent(&observation("executor-observation"));
    assert!(
        !error
            .notes
            .iter()
            .any(|note| note.starts_with("reference-observation:")),
        "uninvoked reference has no invented observation"
    );
    healthy(application, 8, 50);
}

#[test]
fn every_reused_source_row_still_checks_the_independent_reference() {
    use super::super::prepare::{NormalizedFunctionBody, NormalizedInstruction};
    let mut forged = fixture().clone();
    let program = Arc::make_mut(&mut forged.program);
    let mut changed = 0;
    for function in Arc::make_mut(&mut program.functions) {
        if let NormalizedFunctionBody::Code(code) = &mut function.body {
            for instruction in Arc::make_mut(&mut code.instructions) {
                if matches!(instruction, NormalizedInstruction::I64(42)) {
                    *instruction = NormalizedInstruction::I64(43);
                    changed += 1;
                }
            }
        }
    }
    assert!(
        changed > 0,
        "private test lane changes admitted production code only"
    );
    for input in [-7, 0, 8] {
        let error = forged
            .run_observed(
                &Name::new("pure").unwrap(),
                json!([input]).to_string().as_bytes(),
                NormalizedCommandPolicy::default(),
                &ExecutionControl::uncancelled(),
            )
            .err()
            .unwrap();
        assert_eq!(error.code, "normalized_runner_differential");
        for (label, check) in [
            ("production-observation", production_quiescent as fn(&Value)),
            ("reference-observation", reference_quiescent as fn(&Value)),
            ("executor-observation", executor_quiescent as fn(&Value)),
        ] {
            let prefix = format!("{label}: ");
            let observation: Value = serde_json::from_str(
                error
                    .notes
                    .iter()
                    .find_map(|note| note.strip_prefix(&prefix))
                    .unwrap(),
            )
            .unwrap();
            check(&observation);
        }
    }
    healthy(fixture(), 8, 50);
}

#[test]
fn reused_source_authority_does_not_follow_a_new_accepted_head() {
    let (_temporary, application) = authored_fixture();
    let source = format!(
        "request base={}\ndeclarations.begin\n(units (module create later-source))\ndeclarations.end\n",
        application.revision
    );
    let request =
        crate::platform::control::decode_compact_change("later-source.lkjc", source.as_bytes())
            .unwrap();
    let prepared = application
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
    application
        .repository
        .publish(&prepared.publication)
        .unwrap();
    assert_eq!(
        application.check_authority().unwrap_err().code,
        "lifecycle_authority_changed"
    );
    let error = application
        .run_observed(
            &Name::new("pure").unwrap(),
            b"[8]",
            NormalizedCommandPolicy::default(),
            &ExecutionControl::uncancelled(),
        )
        .err()
        .unwrap();
    assert_eq!(error.code, "lifecycle_authority_changed");
}
