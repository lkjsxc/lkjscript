//! Native standard FIFO composition, exact transports and independent state-machine results.
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
    for detached in [false, true] {
        if detached {
            fifo.detach();
        }
        for (mode, name) in ["cells", "buffers", "packets"].iter().enumerate() {
            for batch in cases.chunks(128) {
                let wanted: Vec<_> = batch
                    .iter()
                    .map(|actions| expected(actions, mode))
                    .collect();
                fifo.expected(
                    &format!("fifo-{name}-batch"),
                    &json!([batch]),
                    &json!(wanted),
                    detached,
                );
            }
            for length in [31, 32, 33, 256, 1024] {
                let actions = oracle::burst(length);
                fifo.expected(
                    &format!("fifo-{name}"),
                    &json!([actions]),
                    &expected(&actions, mode),
                    detached,
                );
            }
            let actions = oracle::mixed();
            fifo.expected(
                &format!("fifo-{name}"),
                &json!([actions]),
                &expected(&actions, mode),
                detached,
            );
        }
        for value in [i64::MIN, -1, 0, 1, i64::MAX] {
            fifo.expected(
                "fifo-forwarded",
                &json!([value]),
                &json!({"peek":value,"removed":value,"empty":0}),
                detached,
            );
        }
        let extremes = vec![
            action(0, i64::MIN),
            action(0, i64::MAX),
            action(2, 0),
            action(1, 0),
        ];
        fifo.expected(
            "fifo-cells",
            &json!([extremes]),
            &expected(&extremes, 0),
            detached,
        );
        fifo.verify_identity();
    }
}

#[test]
fn native_owned_fifo_traps_preserve_complete_input_admission_cleanup_and_following_invocations() {
    let fifo = Fifo::new();
    let valid = vec![action(0, 7), action(0, 9), action(2, 0), action(1, 0)];
    for detached in [false, true] {
        if detached {
            fifo.detach();
        }
        let (records, missing) = fifo.run("fifo-empty-front", &json!([[]]), detached, false);
        assert!(missing.is_none());
        let diagnostic = compact_record(&records, "diagnostic");
        assert_eq!(compact_field(diagnostic, "class"), "semantic");
        assert_eq!(
            compact_field(diagnostic, "code"),
            "normalized_sequence_index"
        );
        if detached {
            failed_join(diagnostic);
        }
        // Complete raw admission must reject the invalid final value before the empty-front trap.
        let (records, _) = fifo.run(
            "fifo-empty-front",
            &json!([[1, "invalid"]]),
            detached,
            false,
        );
        let diagnostic = compact_record(&records, "diagnostic");
        assert_eq!(compact_field(diagnostic, "class"), "source");
        assert_eq!(compact_field(diagnostic, "code"), "normalized_json_type");
        for prefix in [0, 1, 31, 32, 33] {
            let mut actions = oracle::burst(prefix);
            actions.push(action(4, 0)); // Explicit invalid-operation trap after a successful prefix.
            let (records, _) = fifo.run("fifo-cells", &json!([actions]), detached, false);
            let diagnostic = compact_record(&records, "diagnostic");
            assert_eq!(compact_field(diagnostic, "code"), "normalized_list_index");
            if detached {
                failed_join(diagnostic);
            }
            fifo.expected(
                "fifo-cells",
                &json!([valid]),
                &expected(&valid, 0),
                detached,
            );
        }
        fifo.verify_identity();
    }
}

#[test]
fn native_owned_fifo_finite_fuel_refusal_joins_detached_custody_without_partial_results() {
    let fifo = Fifo::new();
    let original = fifo.consumer.root.path().join("fifo-cells.deployment.json");
    let mut descriptor: Value = serde_json::from_slice(&std::fs::read(&original).unwrap()).unwrap();
    let actions = oracle::burst(256);
    fifo.detach();
    for fuel in [100, 4096] {
        descriptor["execution"] = json!({"instruction_fuel":fuel,"maximum_call_depth":4096,"maximum_value_stack":1000000});
        fifo.consumer
            .input("limited.deployment.json", &descriptor.to_string());
        let (records, _) = fifo.run("limited", &json!([actions]), true, false);
        let diagnostic = compact_record(&records, "diagnostic");
        assert_eq!(compact_field(diagnostic, "class"), "resource");
        assert_eq!(
            compact_field(diagnostic, "code"),
            "normalized_instruction_steps"
        );
        failed_join(diagnostic);
        fifo.expected(
            "fifo-cells",
            &json!([actions]),
            &expected(&actions, 0),
            true,
        );
    }
    fifo.verify_identity();
}
