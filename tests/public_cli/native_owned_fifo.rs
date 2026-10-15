//! Native nominal standard FIFO composition, exact transports and independent state-machine results.
use super::native_byte_buffer::{Export, dependency, export, stage};
use super::native_owned_worklists::{deployment, unchanged};
use super::*;
use serde_json::json;

#[path = "native_owned_fifo/execution.rs"]
mod execution;
#[path = "native_owned_fifo/fixture.rs"]
mod fixture;
#[path = "native_owned_fifo/negative.rs"]
mod negative;
#[path = "native_owned_fifo/oracle.rs"]
mod oracle;
use execution::failed_join;
use fixture::Fifo;
use oracle::{action, expected};

#[test]
fn native_owned_fifo_exact_generic_library_exhaustive_interleavings_and_source_free_results() {
    let fifo = Fifo::new();
    let cases = oracle::exhaustive();
    let mut matrix = Vec::new();
    for (mode, name) in ["cells", "buffers", "packets"].iter().enumerate() {
        for batch in cases.chunks(128) {
            let wanted: Vec<_> = batch
                .iter()
                .map(|actions| expected(actions, mode))
                .collect();
            matrix.push((format!("fifo-{name}-batch"), json!([batch]), json!(wanted)));
        }
        for length in [31, 32, 33, 256, 1024] {
            let actions = oracle::burst(length);
            matrix.push((
                format!("fifo-{name}"),
                json!([actions]),
                expected(&actions, mode),
            ));
        }
        let actions = oracle::mixed();
        matrix.push((
            format!("fifo-{name}"),
            json!([actions]),
            expected(&actions, mode),
        ));
    }
    for value in [i64::MIN, -1, 0, 1, i64::MAX] {
        matrix.push((
            "fifo-forwarded".to_owned(),
            json!([value]),
            json!({"peek":value,"removed":value,"empty":0}),
        ));
    }
    let extremes = vec![
        action(0, i64::MIN),
        action(0, i64::MAX),
        action(2, 0),
        action(1, 0),
    ];
    matrix.push((
        "fifo-cells".to_owned(),
        json!([extremes]),
        expected(&extremes, 0),
    ));
    assert_eq!(matrix.len(), 45, "all carrier, boundary and extrema rows");
    let rows: Vec<_> = matrix
        .iter()
        .enumerate()
        .map(|(index, (target, input, wanted))| {
            fifo.success_case(format!("{target}-{index}"), target, input, wanted)
        })
        .collect();
    // The source reader also retains an actual ordinary differential process boundary.
    fifo.consumer
        .source_batch("fifo-matrix", "fifo-cells", &rows);
    fifo.verify_identity();
    fifo.detach();
    fifo.batch("fifo-matrix", &rows);
    // Keep an ordinary one-shot process boundary with nonempty owned state after source deletion.
    let smoke = vec![action(0, 7), action(0, 9), action(2, 0), action(1, 0)];
    fifo.expected("fifo-cells", &json!([smoke]), &expected(&smoke, 0), true);
    fifo.verify_identity();
}

#[test]
fn native_owned_fifo_traps_preserve_complete_input_admission_cleanup_and_following_invocations() {
    let fifo = Fifo::new();
    let valid = vec![action(0, 7), action(0, 9), action(2, 0), action(1, 0)];
    let mut cases = vec![
        fifo.failure_case(
            "empty-front",
            "fifo-empty-front",
            &json!([[]]),
            "semantic",
            "normalized_sequence_index",
        ),
        fifo.failure_case(
            "complete-admission",
            "fifo-empty-front",
            &json!([[1, "invalid"]]),
            "source",
            "normalized_json_type",
        ),
    ];
    let mut failed_join_rows = vec![0];
    for prefix in [0, 1, 31, 32, 33] {
        let mut actions = oracle::burst(prefix);
        actions.push(action(4, 0)); // Explicit invalid-operation trap after a successful prefix.
        failed_join_rows.push(cases.len());
        cases.push(fifo.failure_case(
            format!("trap-after-{prefix}"),
            "fifo-cells",
            &json!([actions]),
            "semantic",
            "normalized_list_index",
        ));
        cases.push(fifo.success_case(
            format!("recovery-after-{prefix}"),
            "fifo-cells",
            &json!([valid]),
            &expected(&valid, 0),
        ));
    }
    assert_eq!(cases.len(), 12, "all trap, admission and recovery rows");
    // The invalid final argument must win over the empty-front trap in the second row.
    fifo.consumer
        .source_batch("fifo-traps", "fifo-cells", &cases);
    fifo.verify_identity();
    fifo.detach();
    let rows = fifo.batch("fifo-traps", &cases);
    for index in failed_join_rows {
        failed_join(compact_record(&rows[index], "diagnostic"));
    }
    fifo.verify_identity();
}

#[test]
fn native_owned_fifo_finite_fuel_refusal_joins_detached_custody_without_partial_results() {
    let fifo = Fifo::new();
    let actions = oracle::burst(256);
    fifo.detach();
    let mut cases = Vec::new();
    for fuel in [100, 4096] {
        let mut limited = fifo.failure_case(
            format!("fuel-{fuel}"),
            "fifo-cells",
            &json!([actions]),
            "resource",
            "normalized_instruction_steps",
        );
        limited.execution = Some(
            json!({"instruction_fuel":fuel,"maximum_call_depth":4096,"maximum_value_stack":1000000}),
        );
        cases.push(limited);
        cases.push(fifo.success_case(
            format!("after-fuel-{fuel}"),
            "fifo-cells",
            &json!([actions]),
            &expected(&actions, 0),
        ));
    }
    assert_eq!(
        cases.len(),
        4,
        "both fuel refusals and following recoveries"
    );
    let rows = fifo.batch("fifo-fuel", &cases);
    for index in [0, 2] {
        failed_join(compact_record(&rows[index], "diagnostic"));
    }
    fifo.verify_identity();
}
