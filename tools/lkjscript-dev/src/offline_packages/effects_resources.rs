//! Transferred effect measurements stay in the offline-package receipt owner.
use super::*;
use serde_json::{Value, json};

pub(super) fn validate(parent: &Receipt, root: &Path) -> Result<(), DevError> {
    let actual: Value = serde_json::from_slice(&process::read_bounded(
        &root.join("effect-resources.stdout"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    let effects = &parent.effects;
    let resources = &effects.resources;
    require(
        actual == *resources
            && resources["source_bound"] == true
            && resources["package"] == effects.consumer_package
            && resources["revision"] == effects.consumer_revision
            && resources["artifact"] == effects.artifact_bindings[0]["bundle"]
            && resources["stack_bytes"] == 2_097_152
            && resources["maximum_call_depth_policy"] == 64
            && resources["other_policies"] == "unchanged defaults"
            && resources["live_effects_replayed"] == false
            && resources["cleanup_complete"] == true,
        "effect observations have foreign source, policy, or cleanup bindings",
    )?;
    for number in [
        &resources["preparation_nanoseconds"],
        &resources["preparation"]["type_derivation_steps"],
        &resources["preparation"]["type_metadata_bytes"],
    ] {
        require(
            number.as_u64().is_some_and(|n| n > 0),
            "effect preparation measurements are absent",
        )?;
    }
    let rows = resources["observations"]
        .as_array()
        .ok_or_else(|| DevError::corrupt("effect measurements absent"))?;
    require(
        rows.len() == 56,
        "effect success, boundary, comparison, or stopping observations are missing",
    )?;
    let mut expected_cases = std::collections::BTreeSet::new();
    for tier in ["production", "reference"] {
        expected_cases.insert((tier, "results-jobs", 31, "none"));
        for count in [0_i64, 1, 31, 32, 33, 4097, 8192] {
            for target in ["mapped-jobs", "folded-jobs", "concrete-jobs"] {
                expected_cases.insert((tier, target, count, "none"));
            }
        }
        for case in [
            "cancelled",
            "callback-failure",
            "quota",
            "malformed-result",
            "fuel",
            "allocation",
        ] {
            expected_cases.insert((tier, "mapped-jobs", 31, case));
        }
    }
    for row in rows {
        let string = |field| {
            row[field]
                .as_str()
                .ok_or_else(|| DevError::corrupt("effect observation identity absent"))
        };
        let (tier, target, case) = (string("tier")?, string("target")?, string("case")?);
        let count = row["count"]
            .as_i64()
            .ok_or_else(|| DevError::corrupt("effect workload count absent"))?;
        require(
            expected_cases.remove(&(tier, target, count, case)),
            "effect observation is duplicate or foreign",
        )?;
        let work = &row["observation"];
        require(
            row["cleanup_complete"] == true
                && row["execution_nanoseconds"].as_u64().is_some_and(|n| n > 0),
            "effect runtime timing or cleanup absent",
        )?;
        let counters = work
            .as_object()
            .ok_or_else(|| DevError::corrupt("effect work observation absent"))?;
        let cleanup = counters
            .iter()
            .filter(|(key, _)| key.starts_with("live_") && key.ends_with("_after"))
            .collect::<Vec<_>>();
        require(
            cleanup.len() == if tier == "production" { 6 } else { 8 }
                && cleanup.iter().all(|(_, value)| **value == 0),
            "effect runtime retained state after termination",
        )?;
        require(
            row["instruction_steps_policy"] == if case == "fuel" { 1_000 } else { 10_000_000 }
                && row["allocated_bytes_policy"]
                    == if case == "allocation" {
                        if tier == "reference" {
                            280_000
                        } else {
                            500_000
                        }
                    } else {
                        268_435_456
                    },
            "effect probe changed an unselected execution policy",
        )?;
        let calls = if matches!(case, "fuel" | "allocation") {
            let count_events = row["events"]
                .as_array()
                .ok_or_else(|| DevError::corrupt("budget failure events absent"))?
                .len() as i64;
            require(
                count_events > 0 && count_events < count * 2,
                "budget exhaustion did not stop an active traversal",
            )?;
            count_events
        } else if case == "none" {
            count * 2
        } else if case == "quota" {
            9
        } else {
            3
        };
        let events = (0..calls)
            .map(|index| if index % 2 == 0 { "scale" } else { "bias" })
            .collect::<Vec<_>>();
        require(
            row["events"] == json!(events),
            "effect grant trace or stopping point differs",
        )?;
        if case == "none" {
            let expected = (1..=count).map(|n| n * 3 + 12).collect::<Vec<_>>();
            let output = match target {
                "folded-jobs" => json!(expected.iter().sum::<i64>()),
                "results-jobs" => json!(
                    expected
                        .iter()
                        .map(|n| json!({"case":if *n == 15 {"error"} else {"ok"},"value":n}))
                        .collect::<Vec<_>>()
                ),
                _ => json!(expected),
            };
            let range_frames = if count == 0 {
                1
            } else {
                (count as u64).next_power_of_two().ilog2() as u64 + 1
            };
            require(
                row["output"]["value"] == output
                    && row["output"]["expected_range_frames"] == range_frames
                    && work["capability_calls"] == calls
                    && work["maximum_call_depth"]
                        .as_u64()
                        .is_some_and(|depth| depth <= range_frames + 12),
                "effect complete output, canonical grant count, or logarithmic traversal control bound differs",
            )?;
            if count == 8192 {
                require(
                    row["output"]["observation_sink_disabled_nanoseconds"]
                        .as_u64()
                        .is_some_and(|n| n > 0),
                    "disabled observation cost was not measured",
                )?;
            }
        } else {
            let error = &row["output"]["failure"];
            let accepted = match case {
                "cancelled" => error["code"] == "execution_cancelled",
                "callback-failure" => error["code"] == "effect_probe_callback",
                "quota" => error["class"] == "resource",
                "fuel" => {
                    error["class"] == "resource"
                        && error["code"].as_str().is_some_and(|code| {
                            code.contains("steps") || code.contains("expressions")
                        })
                }
                "allocation" => {
                    error["class"] == "resource"
                        && error["code"]
                            .as_str()
                            .is_some_and(|code| code.contains("allocation"))
                }
                "malformed-result" => error["code"]
                    .as_str()
                    .is_some_and(|code| code.contains("value_admission")),
                _ => false,
            };
            require(
                accepted
                    && row["output"]["reused_alias_complete"] == true
                    && row["output"]["value"].is_null(),
                "effect failure classification or preserved alias recovery differs",
            )?;
        }
    }
    require(
        expected_cases.is_empty(),
        "effect observations are incomplete",
    )?;
    let cancellation: Value = serde_json::from_slice(&process::read_bounded(
        &root.join("effect-transaction.stdout"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    require(
        cancellation == effects.cancellation
            && cancellation["source_bound"] == true
            && cancellation["package"] == effects.consumer_package
            && cancellation["revision"] == effects.consumer_revision
            && cancellation["function"]
                .as_str()
                .is_some_and(|s| s.starts_with("decl_"))
            && cancellation["effects_replayed"] == false
            && cancellation["cleanup_complete"] == true
            && cancellation["callbacks_completed"] == 1
            && cancellation["failure"]["code"] == "execution_cancelled"
            && cancellation["observation"]["capability_calls"] == 3
            && cancellation["observation"]["maximum_live_transactions"] == 1,
        "effect transaction cancellation is not bound to one staged callback",
    )?;
    for name in [
        "live_transactions_after",
        "live_call_frames_after",
        "live_handles_after",
        "live_locals_after",
        "live_operands_after",
        "live_type_bindings_after",
    ] {
        require(
            cancellation["observation"][name] == 0,
            "cancelled effect transaction retained owned state",
        )?;
    }
    Ok(())
}
