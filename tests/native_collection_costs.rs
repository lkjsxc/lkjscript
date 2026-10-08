//! Public cost ceilings permit future proof propagation; they do not require rescanning.
#![allow(clippy::unwrap_used, clippy::panic)]
#[allow(dead_code)]
#[path = "native_components_support/mod.rs"]
mod support;
use serde_json::{Value, json};
use std::fs;
use support::{Native, digest, field, path, quoted};

const SOURCE: &str = include_str!("../examples/owned-metadata-costs/workload.lkjc");

fn within_ceiling(mode: u64, n: u64, k: u64, work: &Value) -> bool {
    let Some(input) = n.checked_add(3) else {
        return false;
    };
    let repetitions = match mode {
        0 | 1 => Some(0),
        2 => k.checked_mul(2),
        3 => Some(k),
        4 => k.checked_add(1),
        _ => None,
    };
    let Some(ceiling) = repetitions.and_then(|k| n.checked_add(2)?.checked_mul(k)) else {
        return false;
    };
    work["input_admission_nodes"].as_u64() == Some(input)
        && work["raw_result_admission_nodes"]
            .as_u64()
            .is_some_and(|n| n <= ceiling)
        && work["internal_guard_descendant_visits"].as_u64() == Some(0)
        && work["capture_admission_nodes"].as_u64() == Some(0)
}

#[test]
fn metadata_cost_oracle_rejects_underchecked_input_and_worsened_work() {
    for (mode, ceiling) in [(0, 0), (1, 0), (2, 262656), (3, 131328), (4, 132354)] {
        let work = json!({"input_admission_nodes":1027, "raw_result_admission_nodes":ceiling,
            "internal_guard_descendant_visits":0,"capture_admission_nodes":0});
        assert!(within_ceiling(mode, 1024, 128, &work));
        let mut better = work.clone();
        better["raw_result_admission_nodes"] = json!(0);
        assert!(within_ceiling(mode, 1024, 128, &better));
        for (field, value) in [
            ("input_admission_nodes", 1026),
            ("raw_result_admission_nodes", ceiling + 1),
            ("internal_guard_descendant_visits", 1),
            ("capture_admission_nodes", 1),
        ] {
            let mut wrong = work.clone();
            wrong[field] = json!(value);
            assert!(!within_ceiling(mode, 1024, 128, &wrong));
        }
    }
    assert!(!within_ceiling(5, 0, 0, &json!({})));
    assert!(!within_ceiling(2, u64::MAX, u64::MAX, &json!({})));
}

#[test]
fn native_collection_selection_cost_matrix_is_observed_without_host_results() {
    let public = Native::new();
    let executable_hash = digest(&public.root.join("lkjscript"));
    let standard = public.export(None, "standard");
    let project = public.project("application");
    public.stage(&project, &standard);
    public.author(&project, SOURCE, &[("builtin", &standard)], true);
    public.check(&project);
    public.unchanged(&project, "collection-cost");
    let artifact = public.root.join("collection-cost.lkja");
    public.cli(&project, &["build", "--output", path(&artifact)], true);
    let artifact_hash = digest(&artifact);
    let mut deployment: Value = serde_json::from_str(include_str!(
        "../examples/owned-metadata-costs/command.deployment.json"
    ))
    .unwrap();
    let descriptor = public.input("collection-cost.deployment.json", &deployment.to_string());
    fs::remove_dir_all(&project).unwrap();
    fs::remove_file(public.root.join("standard.lkjp")).unwrap();
    let mut rows = Vec::new();
    for mode in 0_u64..=4 {
        for n in [0, 8, 64, 256, 1024] {
            for k in [0, 1, 32, 128] {
                let name = format!("mode-{mode}-n-{n}-k-{k}");
                let arguments = public.input(
                    &format!("{name}.json"),
                    &json!([mode, (0..n).collect::<Vec<_>>(), k]).to_string(),
                );
                let output = public.compare(&name, &descriptor, &arguments, &json!(n * k));
                let observation = quoted(&output, "production-observation");
                assert!(
                    within_ceiling(mode, n, k, &observation["value_work"]),
                    "{name}: {observation}"
                );
                assert!(
                    observation["maximum_call_depth"].as_u64().unwrap() <= 4,
                    "{name}"
                );
                rows.push(json!({"mode":mode,"payload_items":n,"repetitions":k,
                    "observation":observation}));
            }
        }
        // Even an unused input must cross complete ordinary ingress admission.
        for k in [0, 1] {
            let mut items: Vec<Value> = (0..32).map(|i| json!(i)).collect();
            items[31] = json!(false);
            let name = format!("wrong-{mode}-{k}");
            let wrong = public.input(
                &format!("{name}.json"),
                &json!([mode, items, k]).to_string(),
            );
            let absent = public.root.join(format!("result-{name}.json"));
            let rejected = public.cli(
                &public.root,
                &[
                    "run",
                    "--deployment",
                    path(&descriptor),
                    "--arguments-file",
                    path(&wrong),
                    "--result-file",
                    path(&absent),
                ],
                false,
            );
            assert_eq!(field(&rejected, "diagnostic", "class"), "source");
            assert_eq!(
                field(&rejected, "diagnostic", "code"),
                "normalized_json_type"
            );
            assert!(!absent.exists());
        }
        let recovery = public.input(
            &format!("recovery-{mode}.json"),
            &json!([mode, [5, 9], 3]).to_string(),
        );
        public.compare(
            &format!("recovery-{mode}"),
            &descriptor,
            &recovery,
            &json!(6),
        );
    }
    deployment["execution"] = json!({"instruction_fuel":1,
        "maximum_call_depth":4096,"maximum_value_stack":1000000});
    let restricted = public.input("restricted.deployment.json", &deployment.to_string());
    let arguments = public.input("restricted-arguments.json", "[2,[5,9],3]");
    let absent = public.root.join("refused-result.json");
    let refused = public.cli(
        &public.root,
        &[
            "run",
            "--deployment",
            path(&restricted),
            "--arguments-file",
            path(&arguments),
            "--result-file",
            path(&absent),
        ],
        false,
    );
    assert_eq!(field(&refused, "diagnostic", "class"), "resource");
    assert_eq!(
        field(&refused, "diagnostic", "code"),
        "normalized_instruction_steps"
    );
    assert!(!absent.exists());
    public.compare("after-refusal", &descriptor, &arguments, &json!(6));
    assert_eq!(artifact_hash, digest(&artifact));
    assert_eq!(executable_hash, digest(&public.root.join("lkjscript")));
    public.input("collection-cost-matrix.json", &json!({
        "purpose":"Public source-authored cost observations; not a speedup or acceptance receipt",
        "modes":{"0":"list-get","1":"map-get-or","2":"pack-unpack",
            "3":"pack-drop","4":"borrowed-field"},"rows":rows,
        "binary_sha256":executable_hash,"artifact_sha256":artifact_hash,
        "source_project_and_transport_deleted":true,"status":"passed",
        "cost_ceilings_allow_lower_future_work":true
    }).to_string());
}
