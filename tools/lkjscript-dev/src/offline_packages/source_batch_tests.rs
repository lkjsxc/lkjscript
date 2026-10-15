//! Hostile parser fixtures exercise evidence admission; they are not execution receipts.
#![allow(clippy::unwrap_used, reason = "closed evidence parser fixtures")]
use super::*;
use serde_json::json;

fn record(operation: &str, fields: &[(&str, String)]) -> String {
    format!(
        "{operation} {}\n",
        fields
            .iter()
            .map(|(key, value)| format!("{key}={}", serde_json::to_string(value).unwrap()))
            .collect::<Vec<_>>()
            .join(" ")
    )
}

fn observed() -> Value {
    json!({"instructions":7,"expressions":11,"capability_calls":0,
        "live_call_frames_after":0,"live_locals_after":0,"live_type_bindings_after":0,
        "live_operands_after":0,"live_transactions_after":0,"live_handles_after":0,
        "live_control_frames_after":0,"live_local_scopes_after":0,"live_type_scopes_after":0,
        "live_effect_scopes_after":0,"live_allowances_after":0})
}

fn joined() -> Value {
    json!({"dispatch_open":false,"active_dispatches":0,"remaining_workers":0})
}

fn success(target: &str, observation: &Value, executor: &Value) -> String {
    [
        record(
            "result",
            &[("status", "success".into()), ("command", "run".into())],
        ),
        record(
            "authority",
            &[
                ("repository", "repository".into()),
                ("package", "package".into()),
                ("revision", "revision".into()),
                ("state", "state".into()),
            ],
        ),
        record(
            "artifact",
            &[
                ("manifest", "manifest".into()),
                ("bundle", "bundle".into()),
                ("bytes", "100".into()),
                ("packages", "3".into()),
                ("closure-objects", "17".into()),
            ],
        ),
        record(
            "execution",
            &[
                ("target", target.into()),
                ("differential", "equal".into()),
                ("value-location", "batch-row".into()),
                ("production-instructions", "7".into()),
                ("reference-expressions", "11".into()),
                ("production-observation", observation.to_string()),
                ("reference-observation", observation.to_string()),
                ("executor-observation", executor.to_string()),
            ],
        ),
    ]
    .concat()
}

fn failure(notes: Vec<String>) -> String {
    record(
        "result",
        &[("status", "failure".into()), ("command", "run".into())],
    ) + &record(
        "diagnostic",
        &[
            ("class", "semantic".into()),
            ("code", "normalized_integer_overflow".into()),
            ("notes", json!(notes).to_string()),
        ],
    )
}

fn fixture() -> (Vec<Case>, Vec<Value>, Binding) {
    let cases = vec![
        Case::success("first", "first", json!([1]), json!({"answer":[3,5]})),
        Case {
            id: "trap".into(),
            target: "overflow".into(),
            arguments: "[9223372036854775807,1]".into(),
            expected: Err("normalized_integer_overflow".into()),
        },
        Case::success("recovery", "second", json!([2]), json!({"answer":[3,5]})),
    ];
    let stdout = success("first", &observed(), &joined());
    let binding =
        Binding::new(&parse_records("parser-fixture", stdout.as_bytes()).unwrap()).unwrap();
    let notes = vec![
        format!("production-observation: {}", observed()),
        format!("executor-observation: {}", joined()),
    ];
    let outputs = vec![
        json!({"id":"first","exit_code":0,"stdout":stdout,"result_json":"{\"answer\":[3,5]}"}),
        json!({"id":"trap","exit_code":2,"stdout":failure(notes),"result_json":null}),
        json!({"id":"recovery","exit_code":0,"stdout":success("second", &observed(), &joined()),"result_json":"{\"answer\":[3,5]}"}),
    ];
    (cases, outputs, binding)
}

fn encoded(outputs: &[Value]) -> Vec<u8> {
    outputs
        .iter()
        .map(|row| format!("{row}\n"))
        .collect::<String>()
        .into_bytes()
}

#[test]
fn complete_rows_keep_distinct_targets_full_values_and_post_trap_recovery() {
    let (cases, outputs, binding) = fixture();
    let actual = rows(&encoded(&outputs), &cases, &binding).unwrap();
    assert_eq!(actual.len(), 3);
    assert_eq!(actual[0].result, Some(json!({"answer":[3,5]})));
    assert!(actual[1].result.is_none());
    assert_eq!(actual[2].result, actual[0].result);
    let request = String::from_utf8(requests(&cases).unwrap()).unwrap();
    let request: Vec<Value> = request
        .lines()
        .map(|row| serde_json::from_str(row).unwrap())
        .collect();
    assert_eq!(
        request[1],
        json!({"id":"trap","target":"overflow","arguments_json":"[9223372036854775807,1]"})
    );
    assert!(
        request
            .iter()
            .all(|row| row.as_object().unwrap().len() == 3)
    );
}

#[test]
fn missing_duplicate_reordered_and_unknown_rows_reject() {
    let (cases, outputs, binding) = fixture();
    for bad in [
        vec![outputs[0].clone(), outputs[2].clone()],
        vec![outputs[0].clone(), outputs[0].clone(), outputs[2].clone()],
        vec![outputs[2].clone(), outputs[1].clone(), outputs[0].clone()],
    ] {
        assert!(rows(&encoded(&bad), &cases, &binding).is_err());
    }
    let mut bad = outputs.clone();
    bad[1]["id"] = json!("foreign");
    assert!(rows(&encoded(&bad), &cases, &binding).is_err());
    let mut duplicate_cases = cases;
    duplicate_cases[1].id = "first".into();
    assert!(requests(&duplicate_cases).is_err());
}

#[test]
fn closed_response_and_complete_nested_json_reject_ambiguous_evidence() {
    let (cases, outputs, binding) = fixture();
    let mut bytes = encoded(&outputs);
    bytes.pop();
    assert!(rows(&bytes, &cases, &binding).is_err());
    for key in ["id", "exit_code", "stdout", "result_json"] {
        let mut bad = outputs.clone();
        bad[1].as_object_mut().unwrap().remove(key);
        assert!(rows(&encoded(&bad), &cases, &binding).is_err(), "{key}");
    }
    for result in [
        json!(null),
        json!("{\"answer\":[3,5],\"answer\":[3,5]}"),
        json!("{\"answer\":[5,3]}"),
    ] {
        let mut bad = outputs.clone();
        bad[0]["result_json"] = result;
        assert!(rows(&encoded(&bad), &cases, &binding).is_err());
    }
    let mut bad = outputs.clone();
    bad[0]["extra"] = json!(true);
    assert!(rows(&encoded(&bad), &cases, &binding).is_err());
    let bytes = String::from_utf8(encoded(&outputs)).unwrap().replacen(
        "\"id\":\"first\"",
        "\"id\":\"first\",\"id\":\"first\"",
        1,
    );
    assert!(rows(bytes.as_bytes(), &cases, &binding).is_err());
}

#[test]
fn equal_values_cannot_hide_wrong_target_binding_or_missing_tier_work() {
    let (cases, outputs, binding) = fixture();
    for stdout in [
        success("second", &observed(), &joined()),
        success("first", &observed(), &joined()).replace("bundle=\"bundle\"", "bundle=\"foreign\""),
        success("first", &observed(), &joined())
            .replace("differential=\"equal\"", "differential=\"other\""),
    ] {
        let mut bad = outputs.clone();
        bad[0]["stdout"] = json!(stdout);
        assert!(rows(&encoded(&bad), &cases, &binding).is_err());
    }
    let mut empty_work = observed();
    empty_work["expressions"] = json!(0);
    let mut bad = outputs.clone();
    bad[0]["stdout"] = json!(success("first", &empty_work, &joined()));
    assert!(rows(&encoded(&bad), &cases, &binding).is_err());
}

#[test]
fn success_and_trap_cleanup_are_genuine_and_required() {
    let (cases, outputs, binding) = fixture();
    for key in [
        "live_operands_after",
        "live_transactions_after",
        "capability_calls",
    ] {
        let mut leaked = observed();
        leaked[key] = json!(1);
        let mut bad = outputs.clone();
        bad[0]["stdout"] = json!(success("first", &leaked, &joined()));
        assert!(rows(&encoded(&bad), &cases, &binding).is_err());
        let mut bad = outputs.clone();
        bad[1]["stdout"] = json!(failure(vec![
            format!("production-observation: {leaked}"),
            format!("executor-observation: {}", joined())
        ]));
        assert!(rows(&encoded(&bad), &cases, &binding).is_err());
    }
    for notes in [vec![], vec![format!("production-observation: {}", observed())],
        vec![format!("executor-observation: {}", joined())],
        vec![format!("production-observation: {}", observed()), "executor-observation: {\"dispatch_open\":true,\"active_dispatches\":0,\"remaining_workers\":0}".into()]] {
        let mut bad = outputs.clone();
        bad[1]["stdout"] = json!(failure(notes));
        assert!(rows(&encoded(&bad), &cases, &binding).is_err());
    }
    for (key, value) in [("exit_code", json!(0)), ("result_json", json!("3"))] {
        let mut bad = outputs.clone();
        bad[1][key] = value;
        assert!(rows(&encoded(&bad), &cases, &binding).is_err());
    }
    let mut no_work = observed();
    no_work["instructions"] = json!(0);
    for extra in [
        "invented note".to_owned(),
        format!("reference-observation: {}", observed()),
    ] {
        let mut bad = outputs.clone();
        bad[1]["stdout"] = json!(failure(vec![
            format!("production-observation: {}", observed()),
            format!("executor-observation: {}", joined()),
            extra
        ]));
        assert!(rows(&encoded(&bad), &cases, &binding).is_err());
    }
    let mut bad = outputs.clone();
    bad[1]["stdout"] = json!(failure(vec![
        format!("production-observation: {no_work}"),
        format!("executor-observation: {}", joined())
    ]));
    assert!(rows(&encoded(&bad), &cases, &binding).is_err());
}

#[test]
fn refusal_cannot_publish_execution_and_success_cannot_invent_inline_results() {
    let (cases, outputs, binding) = fixture();
    for stdout in [
        outputs[1]["stdout"].as_str().unwrap().to_owned() + "execution value=3\n",
        outputs[1]["stdout"]
            .as_str()
            .unwrap()
            .replace("command=\"run\"", "command=\"other\""),
    ] {
        let mut bad = outputs.clone();
        bad[1]["stdout"] = json!(stdout);
        assert!(rows(&encoded(&bad), &cases, &binding).is_err());
    }
    for field in ["value=3", "result-file=foreign.json"] {
        let mut bad = outputs.clone();
        bad[0]["stdout"] = json!(
            outputs[0]["stdout"].as_str().unwrap().to_owned() + &format!("execution {field}\n")
        );
        assert!(rows(&encoded(&bad), &cases, &binding).is_err());
    }
}
