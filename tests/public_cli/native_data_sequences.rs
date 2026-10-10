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
use fixture::{DataSequences, failed_join};

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
        for batch in cases.chunks(128) {
            let expected: Vec<_> = batch
                .iter()
                .map(|actions| oracle::expected(actions))
                .collect();
            data.expected(
                "data-trace-batch",
                &json!([batch]),
                &json!(expected),
                detached,
            );
        }
        for length in [31, 32, 33, 256, 1024] {
            let actions = oracle::burst(length);
            data.expected(
                "data-trace",
                &json!([actions]),
                &oracle::expected(&actions),
                detached,
            );
        }
        let extremes = vec![
            oracle::action(0, i64::MIN),
            oracle::action(0, i64::MAX),
            oracle::action(2, 0),
            oracle::action(3, -1),
            oracle::action(1, 0),
        ];
        data.expected(
            "data-trace",
            &json!([extremes]),
            &oracle::expected(&extremes),
            detached,
        );
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
        for (before, after) in [(i64::MIN, i64::MAX), (-1, 0), (0, 1), (41, -9)] {
            data.expected(
                "data-scalar",
                &json!([before, after]),
                &oracle::snapshot(&json!(before), &json!(after)),
                detached,
            );
        }
        for (before, after) in [(false, true), (true, false)] {
            data.expected(
                "data-bool",
                &json!([before, after]),
                &oracle::snapshot(&json!(before), &json!(after)),
                detached,
            );
        }
        for (before, after) in [(&original, &replacement), (&replacement, &original)] {
            data.expected(
                "data-nested",
                &json!([before, after]),
                &oracle::snapshot(before, after),
                detached,
            );
        }
        if detached {
            let records = data.expected("data-transfer", &json!([original,replacement]), &json!({"before":original,"displaced":original,"after":replacement,"side":original}), true);
            let observation: Value = serde_json::from_str(compact_field(
                compact_record(&records, "execution"),
                "production-observation",
            ))
            .unwrap();
            assert_eq!(observation["parallel_scopes"], 1);
            assert!(observation["parallel_worker_dispatches"].as_u64().unwrap() > 0);
            let records = data.expected(
                "data-shared",
                &json!([original, replacement]),
                &json!({"left":original,"right":original,"after":replacement}),
                true,
            );
            let observation: Value = serde_json::from_str(compact_field(
                compact_record(&records, "execution"),
                "production-observation",
            ))
            .unwrap();
            assert_eq!(observation["parallel_scopes"], 1);
            assert!(observation["parallel_worker_dispatches"].as_u64().unwrap() > 0);
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
        for (original, replacement) in [(7, 42), (i64::MIN, i64::MAX), (i64::MAX, i64::MIN), (0, 0)]
        {
            data.expected(
                "data-owned-replace",
                &json!([original, replacement]),
                &json!({"displaced":original,"first":replacement,"last":9,"length":2}),
                detached,
            );
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
    for detached in [false, true] {
        if detached {
            data.detach();
        }
        for index in [i64::MIN, -1, 2, i64::MAX] {
            for (target, arguments) in [
                ("data-get", json!([index])),
                ("data-replace", json!([index, 42])),
            ] {
                let (records, _) = data.run(target, &arguments, detached, false);
                let diagnostic = compact_record(&records, "diagnostic");
                assert_eq!(compact_field(diagnostic, "class"), "semantic");
                assert_eq!(
                    compact_field(diagnostic, "code"),
                    "normalized_sequence_index"
                );
                if detached {
                    failed_join(diagnostic);
                }
                data.expected("data-get", &json!([0]), &json!(7), detached);
                data.expected(
                    "data-replace",
                    &json!([1, 42]),
                    &json!({"displaced":9,"current":42,"length":2}),
                    detached,
                );
            }
        }
        let (records, _) = data.run("data-order", &json!([]), detached, false);
        let diagnostic = compact_record(&records, "diagnostic");
        assert_eq!(
            compact_field(diagnostic, "code"),
            "normalized_list_index",
            "replacement index evaluates before value"
        );
        if detached {
            failed_join(diagnostic);
        }
        let (records, _) = data.run(
            "data-empty-get",
            &json!([[1, "inadmissible-final-element"]]),
            detached,
            false,
        );
        let diagnostic = compact_record(&records, "diagnostic");
        assert_eq!(compact_field(diagnostic, "class"), "source");
        assert_eq!(
            compact_field(diagnostic, "code"),
            "normalized_json_type",
            "complete input admission precedes execution"
        );
        let (records, _) = data.run("data-empty-get", &json!([[]]), detached, false);
        let diagnostic = compact_record(&records, "diagnostic");
        assert_eq!(
            compact_field(diagnostic, "code"),
            "normalized_sequence_index"
        );
        if detached {
            failed_join(diagnostic);
        }
        data.verify_identity();
    }
    let actions = oracle::burst(256);
    for fuel in [100, 4096] {
        limited["execution"] = json!({"instruction_fuel":fuel,"maximum_call_depth":4096,"maximum_value_stack":1000000});
        data.consumer
            .input("data-limited.deployment.json", &limited.to_string());
        let (records, _) = data.run("data-limited", &json!([actions]), true, false);
        let diagnostic = compact_record(&records, "diagnostic");
        assert_eq!(compact_field(diagnostic, "class"), "resource");
        assert_eq!(
            compact_field(diagnostic, "code"),
            "normalized_instruction_steps"
        );
        failed_join(diagnostic);
        data.expected(
            "data-trace",
            &json!([actions]),
            &oracle::expected(&actions),
            true,
        );
    }
    data.verify_identity();
}
