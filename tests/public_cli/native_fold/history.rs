use super::*;

fn item(index: usize) -> Value {
    let index = i64::try_from(index).unwrap();
    json!({"name":format!("row-{}", index % 5),
        "values":(0..index % 7).map(|j| index * 13 - j * 7).collect::<Vec<_>>()})
}

fn initial_states() -> [Value; 2] {
    [
        json!({"current":[],"prefixes":[]}),
        json!({"current":[item(71), item(72)],"prefixes":[[],[item(89)]]}),
    ]
}

fn expected(items: &[Value], initial: &Value) -> Value {
    let start = initial["current"].as_array().unwrap();
    // Independent prefix slicing. Never evaluate or copy the supplier's state machine.
    let prefixes = initial["prefixes"]
        .as_array()
        .unwrap()
        .iter()
        .cloned()
        .chain(
            (1..=items.len())
                .map(|n| Value::Array(start.iter().chain(&items[..n]).cloned().collect())),
        )
        .collect::<Vec<_>>();
    json!({"current":start.iter().chain(items).cloned().collect::<Vec<_>>(),"prefixes":prefixes})
}

#[test]
fn native_blocked_folds_retain_every_nested_prefix_across_blocks_and_source_removal() {
    let folds = Folds::history();
    for detached in [false, true] {
        if detached {
            folds.detach();
        }
        for n in [0, 1, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 65] {
            let items = (0..n).map(item).collect::<Vec<_>>();
            for initial in initial_states() {
                for mode in 0..3 {
                    let (records, result) =
                        folds.run("history", &json!([items, initial, mode]), detached, true);
                    assert_eq!(result.unwrap(), expected(&items, &initial));
                    if !detached {
                        assert_eq!(
                            compact_field(compact_record(&records, "execution"), "differential"),
                            "equal"
                        );
                    }
                }
            }
        }
    }
    folds.verify_identity();
}

#[test]
fn native_blocked_folds_admit_nested_input_and_unused_initial_history_completely() {
    let folds = Folds::history();
    let valid = (0..9).map(item).collect::<Vec<_>>();
    let initial = initial_states()[1].clone();
    let mut malformed_items = valid.clone();
    malformed_items[8]["values"] = json!([1, "not-an-integer"]);
    let mut malformed_initial = initial.clone();
    malformed_initial["prefixes"][1][0]["values"] = json!(["invalid-unused-prefix"]);
    for detached in [false, true] {
        if detached {
            folds.detach();
        }
        for mode in 0..3 {
            for input in [
                json!([malformed_items, initial, mode]),
                json!([[], malformed_initial, mode]),
            ] {
                let (records, result) = folds.run("history", &input, detached, false);
                assert!(result.is_none());
                let diagnostic = compact_record(&records, "diagnostic");
                assert_eq!(compact_field(diagnostic, "class"), "source");
                assert_eq!(compact_field(diagnostic, "code"), "normalized_json_type");
                let (_, recovered) =
                    folds.run("history", &json!([valid, initial, mode]), detached, true);
                assert_eq!(recovered.unwrap(), expected(&valid, &initial));
            }
        }
    }
    folds.verify_identity();
}

#[test]
fn nested_prefix_oracle_distinguishes_order_and_retained_versions() {
    let initial = initial_states()[1].clone();
    let result = expected(&[item(1), item(2)], &initial);
    assert_eq!(result["prefixes"][2].as_array().unwrap().len(), 3);
    assert_eq!(result["prefixes"][3].as_array().unwrap().len(), 4);
    assert_eq!(result["prefixes"][0], initial["prefixes"][0]);
    assert_ne!(result, expected(&[item(2), item(1)], &initial));
    assert_eq!(expected(&[], &initial), initial);
}
