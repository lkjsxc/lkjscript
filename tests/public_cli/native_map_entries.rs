//! Complete public authoring, detached results and a separately measured scan law.
#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "../map_entry_support/mod.rs"]
mod cases;
#[allow(dead_code)]
#[path = "../native_components_support/mod.rs"]
pub(super) mod support;
use super::semantic_cli::Mode;
use serde_json::{Value, json};
use std::fs;
use support::batch::BatchCase;
use support::{Native, digest, path, quoted};

#[test]
fn native_map_entries_preserve_results_and_linear_projection_work_after_detachment() {
    let candidate = if std::env::var_os(super::RELEASE_CANDIDATE_ENVIRONMENT).is_some() {
        Some(super::binary())
    } else {
        std::env::var_os("LKJSCRIPT_COMPONENT_CANDIDATE").map(std::path::PathBuf::from)
    };
    let public = candidate
        .as_deref()
        .map_or_else(Native::new, Native::from_candidate);
    let mode = Mode::for_candidate(candidate.is_some());
    assert_eq!(public.execution_evidence()["mode"], mode.label());
    let candidate_hash = candidate.as_ref().map(|candidate| {
        let hash = digest(candidate);
        assert_eq!(hash, digest(&public.root.join("lkjscript")));
        hash
    });
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
    let descriptor = public.root.join("command.deployment.json");
    fs::write(&descriptor, deployment.to_string()).unwrap();
    let smoke = cases::input(8, 3);
    let arguments = public.input("attached.json", &json!([true, smoke, 3]).to_string());
    public.compare(
        "attached",
        &descriptor,
        &arguments,
        &cases::expected(&smoke, 3),
    );
    let mut batch = Vec::new();
    let mut pairs = Vec::new();
    for n in [0, 1, 31, 32, 33, 256, 1024, 4096] {
        for shape in 0..4 {
            let input = cases::input(n, shape);
            for k in [0, 1, 8] {
                let expected = cases::expected(&input, k);
                let first = batch.len();
                for enumerate in [false, true] {
                    let name = format!("n{n}-s{shape}-k{k}-e{enumerate}");
                    batch.push(BatchCase::success(
                        name,
                        &json!([enumerate, input, k]),
                        expected.clone(),
                    ));
                }
                pairs.push((n, shape, k, cases::input_nodes(&input), first));
            }
        }
    }
    for enumerate in [false, true] {
        for k in [0, 1] {
            let mut input = cases::input(33, 0);
            input[32]["value"]["items"][2] = json!(false);
            let name = format!("wrong-{enumerate}-{k}");
            batch.push(BatchCase::failure(
                name.clone(),
                &json!([enumerate, input, k]),
                "source",
                "normalized_json_type",
            ));
            batch.push(BatchCase::success(
                format!("recovery-{name}"),
                &json!([enumerate, smoke, 3]),
                cases::expected(&smoke, 3),
            ));
        }
    }
    for (field, code) in [
        ("instruction_fuel", "normalized_instruction_steps"),
        ("maximum_allocated_bytes", "normalized_allocation"),
        ("maximum_collection_items", "normalized_collection_items"),
    ] {
        let mut policy = json!({"instruction_fuel":null,"maximum_call_depth":4096,"maximum_value_stack":1000000});
        policy[field] = json!(1);
        let mut limited = BatchCase::failure(field, &json!([true, smoke, 3]), "resource", code);
        limited.execution = Some(policy);
        batch.push(limited);
        batch.push(BatchCase::success(
            format!("after-{field}"),
            &json!([true, smoke, 3]),
            cases::expected(&smoke, 3),
        ));
    }
    assert_eq!(
        batch.len(),
        206,
        "every original detached matrix/refusal/recovery row"
    );
    // The same complete inventory exercises the source differential in the selected mode.
    // Expectations and modeled work remain in this independent harness.
    let source_outputs =
        public.source_batch("map-matrix", &project, "map-entry-projection", &batch);
    let mut source_rows = Vec::new();
    for &(n, shape, k, nodes, first) in &pairs {
        let pair = [
            quoted(&source_outputs[first], "production-observation"),
            quoted(&source_outputs[first + 1], "production-observation"),
        ];
        assert!(
            cases::valid_cost(
                &pair[0]["value_work"],
                &pair[1]["value_work"],
                nodes,
                n as u64,
                k as u64
            ),
            "source n={n}/shape={shape}/k={k}: {pair:?}"
        );
        source_rows.push(json!({"n":n,"shape":shape,"repetitions":k,"observations":pair}));
    }
    fs::remove_dir_all(project).unwrap();
    fs::remove_file(public.root.join("standard.lkjp")).unwrap();
    public.sources_detached();
    let outputs = public.batch("map-matrix", &descriptor, &batch);
    let mut rows = Vec::new();
    for (n, shape, k, nodes, first) in pairs {
        let pair = [
            quoted(&outputs[first], "production-observation"),
            quoted(&outputs[first + 1], "production-observation"),
        ];
        assert!(
            cases::valid_cost(
                &pair[0]["value_work"],
                &pair[1]["value_work"],
                nodes,
                n as u64,
                k as u64
            ),
            "n={n}/shape={shape}/k={k}: {pair:?}"
        );
        rows.push(json!({"n":n,"shape":shape,"repetitions":k,"observations":pair}));
    }
    public.compare(
        "detached-smoke",
        &descriptor,
        &arguments,
        &cases::expected(&smoke, 3),
    );
    if mode == Mode::HostSemantic {
        assert_eq!(public.execution_evidence()["detached_binary_smokes"], 1);
    }
    public.batch_controls(
        &descriptor,
        &json!([true, smoke, 3]),
        &cases::expected(&smoke, 3),
    );
    support::source_batch::controls_in(mode, &public.root.join("lkjscript"));
    if let (Some(candidate), Some(expected)) = (&candidate, &candidate_hash) {
        assert_eq!(&digest(candidate), expected);
        assert_eq!(&digest(&public.root.join("lkjscript")), expected);
    }
    assert_eq!(artifact_hash, digest(&artifact));
    let validator = lkjscript::platform::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    public.input(
        "map-entry-matrix.json",
        &json!({
            "purpose":"Complete public result and modeled-work evidence, not a timing or publication receipt",
            "status":"passed",
            "execution":public.execution_evidence(),
            "matrix_adapter":if mode == Mode::HostSemantic { "public-cli" } else { "selected-executable" },
            "source_identity":{
                "canonical_schema":lkjscript::platform::canonical_schema::CANONICAL_SCHEMA_HASH_HEX,
                "validator_implementation":validator,
            },
            "selected_candidate":candidate.as_ref().map(|candidate| json!({
                "path":candidate,"sha256":candidate_hash,
            })),
            "artifact_sha256":artifact_hash,
            "sources_deleted":true,
            "source_batch_rows":source_outputs.len(),
            "source_pairs":source_rows,
            "deployment_batch_rows":outputs.len(),
            "pairs":rows,
        })
        .to_string(),
    );
}
