//! Generalized owned sequences through literal native authoring, exact exports and source-free execution.
use super::native_byte_buffer::{Export, dependency, export, stage};
use super::native_owned_worklists::{deployment, unchanged};
use super::*;
use serde_json::json;

#[path = "native_data_sequences/fixture.rs"]
mod fixture;
#[path = "native_data_sequences/negative.rs"]
mod negative;
#[path = "native_data_sequences/oracle.rs"]
mod oracle;
use fixture::{DataSequences, failed_join, failure_case, success_case};

#[test]
fn native_data_sequences_generic_export_finite_model_and_detached_execution() {
    let data = DataSequences::new();
    let discovery = data
        .consumer
        .cli(&["capabilities", "--section", "change"], true);
    for syntax in ["sequence-get", "sequence-replace"] {
        assert!(
            discovery.iter().any(|record| record
                .fields
                .iter()
                .any(|field| field.value.contains(syntax))),
            "discovery {syntax}"
        );
    }
    for name in [
        "data-sequence-empty",
        "data-sequence-length",
        "data-sequence-push",
        "data-sequence-pop",
        "data-sequence-get",
        "data-sequence-replace",
        "data-sequence-discard",
        "sequence-replace",
    ] {
        let owners = data.consumer.cli(
            &["package", "builtin", "query", "owners", "--name", name],
            true,
        );
        assert_eq!(
            owners
                .iter()
                .filter(|record| record.operation == "owner")
                .count(),
            1,
            "standard operation {name}"
        );
    }
    for (name, form, roles) in [
        (
            "get",
            "sequence_get",
            &["owned_sequence_index", "owned_sequence_source"][..],
        ),
        (
            "replace",
            "sequence_replace",
            &[
                "owned_sequence_index",
                "owned_sequence_value",
                "owned_sequence_source",
            ][..],
        ),
    ] {
        let module = data
            .supplier
            .cli(&["query", "find", "module", "data-sequences"], true);
        let functions = data.supplier.cli(
            &[
                "query",
                "find",
                "declaration",
                name,
                "--parent",
                compact_field(compact_record(&module, "owner"), "id"),
            ],
            true,
        );
        let records = data.supplier.cli(
            &[
                "inspect",
                "owner",
                "pure_function",
                compact_field(compact_record(&functions, "owner"), "id"),
                "--detail",
                "definition",
                "--limit",
                "1000",
            ],
            true,
        );
        let parent = records
            .iter()
            .find(|record| {
                record.operation == "definition.expression" && compact_field(record, "form") == form
            })
            .unwrap();
        let id = compact_field(parent, "id");
        let children: Vec<_> = records
            .iter()
            .filter(|record| {
                record.operation == "definition.expression"
                    && super::super::compact_field(record, "parent") == Some(id)
            })
            .map(|record| compact_field(record, "slot"))
            .collect();
        assert_eq!(
            children, roles,
            "public definition preserves evaluation order"
        );
        let expected_roles: &[&str] = if name == "get" {
            &["owned_sequence_type"]
        } else {
            &["owned_sequence_type", "owned_sequence_result_type"]
        };
        let reference_source = format!("{}/{id}", data.generic.package);
        for role in expected_roles {
            assert!(
                records
                    .iter()
                    .any(|record| record.operation == "definition.reference"
                        && super::super::compact_field(record, "source")
                            == Some(reference_source.as_str())
                        && super::super::compact_field(record, "role") == Some(*role)
                        && super::super::compact_field(record, "target-kind") == Some("type")),
                "public {form} type reference {role}"
            );
        }
    }
    let cases = oracle::exhaustive();
    for detached in [false, true] {
        if detached {
            data.detach();
        }
        let mut executions = Vec::new();
        for batch in cases.chunks(128) {
            let expected: Vec<_> = batch
                .iter()
                .map(|actions| oracle::expected(actions))
                .collect();
            data.queue_expected(
                "data-trace-batch",
                &json!([batch]),
                &json!(expected),
                &mut executions,
            );
        }
        for length in [31, 32, 33, 256, 1024] {
            let actions = oracle::burst(length);
            data.queue_expected(
                "data-trace",
                &json!([actions]),
                &oracle::expected(&actions),
                &mut executions,
            );
        }
        let extremes = vec![
            oracle::action(0, i64::MIN),
            oracle::action(0, i64::MAX),
            oracle::action(2, 0),
            oracle::action(3, -1),
            oracle::action(1, 0),
        ];
        data.queue_expected(
            "data-trace",
            &json!([extremes]),
            &oracle::expected(&extremes),
            &mut executions,
        );
        if detached {
            assert_eq!(executions.len(), cases.len().div_ceil(128) + 6);
            data.batch("data-finite-model", &executions);
            // The complete matrix shares candidate preparation; a nonempty ordinary
            // copied-binary run independently retains the source-free CLI boundary.
            let actions = oracle::burst(33);
            data.expected(
                "data-trace",
                &json!([actions]),
                &oracle::expected(&actions),
                true,
            );
        } else {
            data.source_batch("data-finite-model", &executions);
        }
        data.verify_identity();
    }
}

#[test]
fn native_data_sequences_immutable_reads_survive_replacement_removal_disposal_and_transfer() {
    let data = DataSequences::new();
    let original = json!({"items":[{"label":"retained λ","values":[i64::MIN,7,i64::MAX]},{"label":"empty","values":[]}],"flags":[true,false,true]});
    let replacement = json!({"items":[{"label":"replacement","values":(0..257).collect::<Vec<_>>() }],"flags":[false]});
    for detached in [false, true] {
        if detached {
            data.detach();
        }
        let mut executions = Vec::new();
        for (before, after) in [(i64::MIN, i64::MAX), (-1, 0), (0, 1), (41, -9)] {
            data.queue_expected(
                "data-scalar",
                &json!([before, after]),
                &oracle::snapshot(&json!(before), &json!(after)),
                &mut executions,
            );
        }
        for (before, after) in [(false, true), (true, false)] {
            data.queue_expected(
                "data-bool",
                &json!([before, after]),
                &oracle::snapshot(&json!(before), &json!(after)),
                &mut executions,
            );
        }
        for (before, after) in [(&original, &replacement), (&replacement, &original)] {
            data.queue_expected(
                "data-nested",
                &json!([before, after]),
                &oracle::snapshot(before, after),
                &mut executions,
            );
        }
        if detached {
            assert_eq!(
                executions.len(),
                8,
                "all detached pure immutable-read cases"
            );
            data.batch("data-immutable-reads", &executions);
            // Task callable kind remains an ordinary foreground boundary even with
            // an empty effect row; each run retains actual structured-worker evidence.
            for (target, expected) in [
                (
                    "data-transfer",
                    json!({"before":original,"displaced":original,"after":replacement,"side":original}),
                ),
                (
                    "data-shared",
                    json!({"left":original,"right":original,"after":replacement}),
                ),
            ] {
                let records =
                    data.expected(target, &json!([original, replacement]), &expected, true);
                let observation: Value = serde_json::from_str(compact_field(
                    compact_record(&records, "execution"),
                    "production-observation",
                ))
                .unwrap();
                assert_eq!(observation["parallel_scopes"], 1);
                assert!(observation["parallel_worker_dispatches"].as_u64().unwrap() > 0);
            }
            data.expected(
                "data-nested",
                &json!([original, replacement]),
                &oracle::snapshot(&original, &replacement),
                true,
            );
        } else {
            data.source_batch("data-immutable-reads", &executions);
        }
        data.verify_identity();
    }
}

#[test]
fn native_data_sequences_owned_replacement_preserves_displaced_custody() {
    let data = DataSequences::new();
    for detached in [false, true] {
        if detached {
            data.detach();
        }
        let mut executions = Vec::new();
        for (original, replacement) in [(7, 42), (i64::MIN, i64::MAX), (i64::MAX, i64::MIN), (0, 0)]
        {
            data.queue_expected(
                "data-owned-replace",
                &json!([original, replacement]),
                &json!({"displaced":original,"first":replacement,"last":9,"length":2}),
                &mut executions,
            );
        }
        if detached {
            assert_eq!(executions.len(), 4, "all detached owned replacements");
            data.batch("data-owned-replacements", &executions);
            data.expected(
                "data-owned-replace",
                &json!([7, 42]),
                &json!({"displaced":7,"first":42,"last":9,"length":2}),
                true,
            );
        } else {
            data.source_batch("data-owned-replacements", &executions);
        }
        data.verify_identity();
    }
}

#[test]
fn native_data_sequences_bounds_and_fuel_refusals_join_cleanup_and_recover() {
    let data = DataSequences::new();
    let descriptor_path = data.consumer.root.path().join("data-trace.deployment.json");
    let mut limited: Value =
        serde_json::from_slice(&std::fs::read(descriptor_path).unwrap()).unwrap();
    let mut executions = Vec::new();
    for index in [i64::MIN, -1, 2, i64::MAX] {
        for (target, arguments) in [
            ("data-get", json!([index])),
            ("data-replace", json!([index, 42])),
        ] {
            executions.push(failure_case(
                target,
                &arguments,
                "semantic",
                "normalized_sequence_index",
                executions.len(),
            ));
            executions.push(success_case(
                "data-get",
                &json!([0]),
                &json!(7),
                executions.len(),
            ));
            executions.push(success_case(
                "data-replace",
                &json!([1, 42]),
                &json!({"displaced":9,"current":42,"length":2}),
                executions.len(),
            ));
        }
    }
    executions.push(failure_case(
        "data-order",
        &json!([]),
        "semantic",
        "normalized_list_index",
        executions.len(),
    ));
    executions.push(failure_case(
        "data-empty-get",
        &json!([[1, "inadmissible-final-element"]]),
        "source",
        "normalized_json_type",
        executions.len(),
    ));
    executions.push(failure_case(
        "data-empty-get",
        &json!([[]]),
        "semantic",
        "normalized_sequence_index",
        executions.len(),
    ));
    assert_eq!(
        executions.len(),
        27,
        "every attached refusal and recovery row"
    );
    data.source_batch("data-refusals-and-recovery", &executions);
    data.detach();
    let actions = oracle::burst(256);
    for fuel in [100, 4096] {
        limited["execution"] = json!({"instruction_fuel":fuel,"maximum_call_depth":4096,"maximum_value_stack":1000000});
        assert_eq!(limited["target"], "data-trace");
        let mut refused = failure_case(
            "data-trace",
            &json!([actions]),
            "resource",
            "normalized_instruction_steps",
            executions.len(),
        );
        refused.execution = Some(limited["execution"].clone());
        executions.push(refused);
        executions.push(success_case(
            "data-trace",
            &json!([actions]),
            &oracle::expected(&actions),
            executions.len(),
        ));
    }
    assert_eq!(
        executions.len(),
        31,
        "every detached refusal and recovery row"
    );
    let observations = data.batch("data-refusals-and-recovery", &executions);
    let mut runtime_refusals = 0;
    for records in &observations {
        if let Some(diagnostic) = records
            .iter()
            .find(|record| record.operation == "diagnostic")
            && matches!(compact_field(diagnostic, "class"), "semantic" | "resource")
        {
            failed_join(diagnostic);
            runtime_refusals += 1;
        }
    }
    assert_eq!(
        runtime_refusals, 12,
        "every executed refusal joined cleanup"
    );
    let (records, _) = data.run("data-get", &json!([2]), true, false);
    let diagnostic = compact_record(&records, "diagnostic");
    assert_eq!(compact_field(diagnostic, "class"), "semantic");
    assert_eq!(
        compact_field(diagnostic, "code"),
        "normalized_sequence_index"
    );
    failed_join(diagnostic);
    data.expected(
        "data-replace",
        &json!([1, 42]),
        &json!({"displaced":9,"current":42,"length":2}),
        true,
    );
    data.verify_identity();
}
