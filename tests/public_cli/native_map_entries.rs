//! Complete public authoring, detached results and a separately measured scan law.
#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "../map_entry_support/mod.rs"]
mod cases;
#[allow(dead_code)]
#[path = "../native_components_support/mod.rs"]
pub(super) mod support;
use serde_json::{Value, json};
use std::fs;
use support::{Native, digest, path, quoted};

#[test]
fn native_map_entries_preserve_results_and_linear_projection_work_after_detachment() {
    let candidate = super::binary();
    let candidate_hash = digest(&candidate);
    let public = Native::from_candidate(&candidate);
    assert_eq!(candidate_hash, digest(&public.root.join("lkjscript")));
    let executable = digest(&public.root.join("lkjscript"));
    let standard = public.export(None, "standard");
    let project = public.project("application");
    public.stage(&project, &standard);
    public.author(
        &project,
        include_str!("../../examples/map-entry-projection/workload.lkjc"),
        &[("builtin", &standard)],
        true,
    );
    public.check(&project);
    public.unchanged(&project, "map-entry-projection");
    let artifact = public.root.join("map-entry-projection.lkja");
    public.cli(&project, &["build", "--output", path(&artifact)], true);
    let artifact_hash = digest(&artifact);
    let deployment: Value = serde_json::from_str(include_str!(
        "../../examples/map-entry-projection/command.deployment.json"
    ))
    .unwrap();
    let descriptor = public.input("command.deployment.json", &deployment.to_string());
    let smoke = cases::input(8, 3);
    let arguments = public.input("attached.json", &json!([true, smoke, 3]).to_string());
    public.compare(
        "attached",
        &descriptor,
        &arguments,
        &cases::expected(&smoke, 3),
    );
    fs::remove_dir_all(project).unwrap();
    fs::remove_file(public.root.join("standard.lkjp")).unwrap();
    let mut rows = Vec::new();
    for n in [0, 1, 31, 32, 33, 256, 1024, 4096] {
        for shape in 0..4 {
            let input = cases::input(n, shape);
            for k in [0, 1, 8] {
                let expected = cases::expected(&input, k);
                let mut pair = Vec::new();
                for enumerate in [false, true] {
                    let name = format!("n{n}-s{shape}-k{k}-e{enumerate}");
                    let arguments = public.input(
                        &format!("{name}.json"),
                        &json!([enumerate, input, k]).to_string(),
                    );
                    let output = public.compare(&name, &descriptor, &arguments, &expected);
                    pair.push(quoted(&output, "production-observation"));
                }
                assert!(
                    cases::valid_cost(
                        &pair[0]["value_work"],
                        &pair[1]["value_work"],
                        cases::input_nodes(&input),
                        n as u64,
                        k as u64
                    ),
                    "n={n}/shape={shape}/k={k}: {pair:?}"
                );
                rows.push(json!({"n":n,"shape":shape,"repetitions":k,"observations":pair}));
            }
        }
    }
    for enumerate in [false, true] {
        for k in [0, 1] {
            let mut input = cases::input(33, 0);
            input[32]["value"]["items"][2] = json!(false);
            let name = format!("wrong-{enumerate}-{k}");
            let arguments = public.input(
                &format!("{name}.json"),
                &json!([enumerate, input, k]).to_string(),
            );
            cases::refusal(
                &public,
                &descriptor,
                &name,
                &arguments,
                "source",
                "normalized_json_type",
            );
            let good = public.input(
                &format!("recovery-{name}.json"),
                &json!([enumerate, smoke, 3]).to_string(),
            );
            public.compare(
                &format!("recovery-{name}"),
                &descriptor,
                &good,
                &cases::expected(&smoke, 3),
            );
        }
    }
    for (field, code) in [
        ("instruction_fuel", "normalized_instruction_steps"),
        ("maximum_allocated_bytes", "normalized_allocation"),
        ("maximum_collection_items", "normalized_collection_items"),
    ] {
        let mut limited = deployment.clone();
        limited["execution"] = json!({"instruction_fuel":null,"maximum_call_depth":4096,"maximum_value_stack":1000000});
        limited["execution"][field] = json!(1);
        let limit = public.input(&format!("limit-{field}.json"), &limited.to_string());
        cases::refusal(&public, &limit, field, &arguments, "resource", code);
        public.compare(
            &format!("after-{field}"),
            &descriptor,
            &arguments,
            &cases::expected(&smoke, 3),
        );
    }
    assert_eq!(candidate_hash, digest(&candidate));
    assert_eq!(executable, digest(&public.root.join("lkjscript")));
    assert_eq!(artifact_hash, digest(&artifact));
    public.input("map-entry-matrix.json",&json!({"purpose":"Complete public result and modeled-work evidence, not a timing or publication receipt","status":"passed","binary_sha256":executable,"artifact_sha256":artifact_hash,"sources_deleted":true,"pairs":rows}).to_string());
}
