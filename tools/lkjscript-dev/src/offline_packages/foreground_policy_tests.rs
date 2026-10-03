#![allow(clippy::expect_used, clippy::unwrap_used)]
use super::*;
use serde_json::json;
use std::collections::BTreeMap;

fn cells(base: u64, stride: u64) -> Vec<LoopCell> {
    [
        ("calibration-zero", 0, false),
        ("calibration-thousand", 1000, false),
        ("past-ten-million", LONG_LOOP, false),
        ("bounded-equivalent", 1000, true),
    ]
    .into_iter()
    .enumerate()
    .map(|(command, (label, n, bounded))| {
        let output = json!({"position":n,"total":n * (n - 1) / 2});
        let observation = json!({
            "instructions":base + stride * n as u64,
            "capability_calls":0,"maximum_call_depth":3,
            "maximum_control_frames":3,"maximum_live_allowances":3,
            "maximum_live_locals":9,"maximum_live_type_bindings":2,
            "maximum_value_stack":5,"maximum_live_transactions":0,
            "live_call_frames_after":0,"live_handles_after":0,"live_locals_after":0,
            "live_operands_after":0,"live_transactions_after":0,"live_type_bindings_after":0,
            "parallel_scopes":0,"parallel_worker_dispatches":0,"parallel_inline_fallbacks":0,"tail_transfers":n + 1
        });
        let mut execution = BTreeMap::new();
        for (field, value) in [
            ("execution-mode", "production"),
            ("verification", "not-performed"),
            (
                "execution-profile",
                if bounded {
                    "bounded"
                } else {
                    "trusted-foreground"
                },
            ),
            (
                "instruction-limit",
                if bounded { "10000000" } else { "absent" },
            ),
            (
                "allocation-limit",
                if bounded { "268435456" } else { "absent" },
            ),
            (
                "collection-limit",
                if bounded { "1000000" } else { "absent" },
            ),
            (
                "capability-call-limit",
                if bounded { "100000" } else { "absent" },
            ),
        ] {
            execution.insert(field.into(), value.into());
        }
        execution.insert("production-observation".into(), observation.to_string());
        execution.insert("value".into(), output.to_string());
        execution.insert(
            "cleanup".into(),
            json!({"admission_stopped":true,"remaining_tasks":0,"cleanup_failures":[]}).to_string(),
        );
        LoopCell {
            label: label.into(),
            command,
            n,
            bounded,
            output,
            execution,
        }
    })
    .collect()
}

fn replace(cells: &mut [LoopCell], index: usize, field: &str, value: Value) {
    let mut observation: Value =
        serde_json::from_str(&cells[index].execution["production-observation"]).unwrap();
    observation[field] = value;
    cells[index]
        .execution
        .insert("production-observation".into(), observation.to_string());
}

#[test]
fn all_four_nominal_field_order_costs_are_valid_policy_witnesses() {
    // Two two-field constructors can each require two StoreLocal/LoadLocal pairs.
    // State construction also occurs once per iteration; output construction does not.
    for (base, stride) in [(44, 32), (48, 32), (48, 36), (52, 36)] {
        validate_series(&cells(base, stride)).unwrap();
    }
}

#[test]
fn a_long_run_below_the_former_quota_is_not_a_policy_witness() {
    assert!(validate_series(&cells(44, 1)).is_err());
}

#[test]
fn calibration_cannot_consume_the_long_run_oracle() {
    assert!(validate_series(&cells(44, 20_000)).is_err());
    assert!(validate_series(&cells(44, 0)).is_err());
}

#[test]
fn nonlinear_or_fractional_instruction_work_is_rejected() {
    let mut nonlinear = cells(52, 36);
    replace(&mut nonlinear, 2, "instructions", json!(11_250_017));
    assert!(validate_series(&nonlinear).is_err());
    let mut fractional = cells(52, 36);
    replace(&mut fractional, 1, "instructions", json!(36_053));
    assert!(validate_series(&fractional).is_err());
}

#[test]
fn bounded_and_unbounded_equivalent_work_must_agree() {
    let mut changed = cells(52, 36);
    replace(&mut changed, 3, "instructions", json!(36_056));
    assert!(validate_series(&changed).is_err());
}

#[test]
fn live_state_cannot_grow_with_iteration_count() {
    for field in [
        "maximum_live_locals",
        "maximum_control_frames",
        "maximum_value_stack",
    ] {
        let mut changed = cells(52, 36);
        replace(&mut changed, 2, field, json!(1001));
        assert!(validate_series(&changed).is_err(), "{field}");
    }
}

#[test]
fn completed_loops_must_release_every_tracked_state_kind() {
    for field in [
        "live_call_frames_after",
        "live_handles_after",
        "live_locals_after",
        "live_operands_after",
        "live_transactions_after",
        "live_type_bindings_after",
        "maximum_live_transactions",
        "parallel_scopes",
        "parallel_worker_dispatches",
        "parallel_inline_fallbacks",
    ] {
        let mut changed = cells(52, 36);
        replace(&mut changed, 2, field, json!(1));
        assert!(validate_series(&changed).is_err(), "{field}");
    }
}

#[test]
fn counters_must_be_present_unsigned_integers() {
    for value in [
        Value::Null,
        json!(-1),
        json!(1.5),
        json!("11250016"),
        json!(u64::MAX),
    ] {
        let mut changed = cells(52, 36);
        replace(&mut changed, 2, "instructions", value);
        assert!(validate_series(&changed).is_err());
    }
}

#[test]
fn missing_duplicate_and_substituted_workloads_are_rejected() {
    let mut missing = cells(52, 36);
    missing.pop();
    assert!(validate_series(&missing).is_err());
    let mut reordered = cells(52, 36);
    reordered.swap(1, 2);
    assert!(validate_series(&reordered).is_err());
    let mut changed = cells(52, 36);
    changed[2].n = i64::MAX;
    assert!(validate_series(&changed).is_err());
    let mut mode = cells(52, 36);
    mode[3].bounded = false;
    assert!(validate_series(&mode).is_err());
}

#[test]
fn outputs_and_policy_are_independent_of_the_calibration() {
    let mut output = cells(52, 36);
    output[2]
        .execution
        .insert("value".into(), json!({"position":0,"total":0}).to_string());
    assert!(validate_series(&output).is_err());
    let mut policy = cells(52, 36);
    policy[2]
        .execution
        .insert("instruction-limit".into(), "10000000".into());
    assert!(validate_series(&policy).is_err());
    let mut tail = cells(52, 36);
    replace(&mut tail, 2, "tail_transfers", json!(LONG_LOOP));
    assert!(validate_series(&tail).is_err());
}

#[test]
fn absent_or_failed_cleanup_is_a_diagnostic_not_a_panic() {
    let mut absent = cells(52, 36);
    absent[2].execution.remove("cleanup");
    assert!(validate_series(&absent).is_err());
    let mut failed = cells(52, 36);
    failed[2].execution.insert(
        "cleanup".into(),
        json!({"admission_stopped":true,"remaining_tasks":1,"cleanup_failures":[]}).to_string(),
    );
    assert!(validate_series(&failed).is_err());
}
