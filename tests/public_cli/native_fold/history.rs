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
    let mut batch = Vec::new();
    for n in [0, 1, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 65] {
        let items = (0..n).map(item).collect::<Vec<_>>();
        for initial in initial_states() {
            for mode in 0..3 {
                batch.push(BatchCase::success(
                    format!(
                        "history-{n}-{}-{mode}",
                        initial["current"].as_array().unwrap().len()
                    ),
                    &json!([items, initial, mode]),
                    expected(&items, &initial),
                ));
            }
        }
    }
    assert_eq!(
        batch.len(),
        90,
        "every original nested-prefix row in each lane"
    );
    folds.source_batch("attached-history", "history", &batch);
    folds.detach();
    folds.batch("detached-history", "history", &batch);
    let items = vec![item(1), item(2)];
    let initial = initial_states()[1].clone();
    let (_, smoke) = folds.run("history", &json!([items, initial, 2]), true, true);
    assert_eq!(smoke.unwrap(), expected(&items, &initial));
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
    let mut batch = Vec::new();
    for mode in 0..3 {
        for (kind, input) in [
            json!([malformed_items, initial, mode]),
            json!([[], malformed_initial, mode]),
        ]
        .into_iter()
        .enumerate()
        {
            batch.push(BatchCase::failure(
                format!("invalid-history-{mode}-{kind}"),
                &input,
                "source",
                "normalized_json_type",
            ));
            batch.push(BatchCase::success(
                format!("after-invalid-history-{mode}-{kind}"),
                &json!([valid, initial, mode]),
                expected(&valid, &initial),
            ));
        }
    }
    assert_eq!(
        batch.len(),
        12,
        "every original nested admission/recovery row in each lane"
    );
    folds.source_batch("attached-history-admission", "history", &batch);
    folds.detach();
    folds.batch("detached-history-admission", "history", &batch);
    let (_, smoke) = folds.run("history", &json!([valid, initial, 2]), true, true);
    assert_eq!(smoke.unwrap(), expected(&valid, &initial));
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
