//! Native dependency-first planning with independent expected results. Host matrices share
//! admitted code; exact candidates and detached smoke runs retain the executable boundary.
#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "native_components_support/mod.rs"]
mod components;
#[allow(
    dead_code,
    reason = "shared semantic CLI support spans independent integration crates"
)]
#[path = "support/semantic_cli.rs"]
mod semantic_cli;
use components::copy_executable;
#[path = "native_plan_support/mod.rs"]
mod plan;
use components::matrix::Case;
use components::{Native, digest, quoted};
use serde_json::{Value, json};

#[test]
fn native_plan_condenses_complete_graphs_and_emits_dependency_first_stages() {
    let public = Native::new();
    let executable_hash = digest(&public.root.join("lkjscript"));
    plan::author(&public);
    let artifact = public.root.join("dependency-plan.lkja");
    let artifact_hash = digest(&artifact);
    let descriptors = plan::DESCRIPTORS
        .map(|(name, text)| public.input(&format!("{name}.deployment.json"), text));
    let storage_descriptors = plan::STORAGE_DESCRIPTORS
        .map(|(name, text)| public.input(&format!("{name}.deployment.json"), text));
    let single_modes: Vec<_> = ["flat", "chunked"]
        .into_iter()
        .zip(&descriptors[..2])
        .chain(
            plan::STORAGE_MODES
                .into_iter()
                .zip(&storage_descriptors[..4]),
        )
        .collect();
    let batch_modes: Vec<_> = ["flat", "chunked"]
        .into_iter()
        .zip(&descriptors[2..])
        .chain(
            plan::STORAGE_MODES
                .into_iter()
                .zip(&storage_descriptors[4..]),
        )
        .collect();
    let proposals = plan::cases::suite();
    let expected: Vec<Value> = proposals.iter().map(plan::oracle::expected).collect();
    public.input("expected-suite.json", &json!(expected).to_string());
    // The ordinary runner admits at most 1 MiB of argument-file bytes. Divide
    // independent test inputs instead of weakening that public resource boundary.
    let mut maximum_argument_bytes = 0;
    let batches: Vec<_> = proposals
        .chunks(256)
        .zip(expected.chunks(256))
        .enumerate()
        .map(|(index, (inputs, outputs))| {
            let text = json!([inputs]).to_string();
            assert!(
                text.len() <= 1_048_576,
                "test batch exceeds ordinary runner input bound"
            );
            maximum_argument_bytes = maximum_argument_bytes.max(text.len());
            (
                public.input(&format!("suite-{index:02}.json"), &text),
                json!(outputs),
            )
        })
        .collect();
    assert_eq!(batches.len(), 19);
    let example = components::cases::example();
    let sample_expected = plan::oracle::expected(&example);
    let sample_arguments = public.input(
        "attached-batch-sample.json",
        &json!([[example]]).to_string(),
    );
    let batch_sample_expected = json!([sample_expected]);
    let mut attached_observations = Vec::new();
    for &(mode, descriptor) in &batch_modes {
        let output = public.compare(
            &format!("attached-sample-{mode}"),
            descriptor,
            &sample_arguments,
            &batch_sample_expected,
        );
        attached_observations.push(json!({"phase":"attached-sample","mode":mode,"graphs":1,
            "observation":quoted(&output,"production-observation")}));
    }
    plan::detach(&public);
    public.matrix_boundary(
        "detached-ordinary-boundary",
        &Case {
            name: "detached-ordinary-boundary".to_owned(),
            descriptor: batch_modes[0].1.clone(),
            arguments: sample_arguments,
            expected: batch_sample_expected,
        },
    );
    let mut matrix = Vec::new();
    let mut observations = Vec::new();
    for &(mode, descriptor) in &batch_modes {
        for (index, (arguments, expected)) in batches.iter().enumerate() {
            matrix.push(Case {
                name: format!("source-deleted-{mode}-{index:02}"),
                descriptor: descriptor.clone(),
                arguments: arguments.clone(),
                expected: expected.clone(),
            });
            observations.push(json!({"phase":"source-deleted","mode":mode,"batch":index,
                "graphs":expected.as_array().unwrap().len()}));
        }
    }
    assert_eq!(matrix.len(), 114);
    assert_eq!(
        observations
            .iter()
            .map(|row| row["graphs"].as_u64().unwrap())
            .sum::<u64>(),
        27_810
    );
    for (observation, output) in observations
        .iter_mut()
        .zip(public.matrix("dependency-plan", &matrix))
    {
        observation["observation"] = quoted(&output, "production-observation");
    }
    let example_arguments = public.input("example.json", &json!([example]).to_string());
    let empty_arguments = public.input("empty-batch.json", "[[]]");
    for &(mode, descriptor) in &single_modes {
        public.compare(
            &format!("single-{mode}"),
            descriptor,
            &example_arguments,
            &plan::oracle::expected(&example),
        );
    }
    for &(mode, descriptor) in &batch_modes {
        public.compare(
            &format!("empty-{mode}"),
            descriptor,
            &empty_arguments,
            &json!([]),
        );
    }
    plan::refusals(&public, &single_modes);
    assert_eq!(artifact_hash, digest(&artifact));
    assert_eq!(executable_hash, digest(&public.root.join("lkjscript")));
    public.input(
        "witness-summary.json",
        &json!({
            "purpose":"Native dependency-plan evidence, not source or release acceptance",
            "graphs":proposals.len(), "exhaustive_three_vertex_graphs":512,
            "exhaustive_loop_free_four_vertex_graphs":4096,
            "matched_batch_executions":batches.len()*batch_modes.len(),
            "proposal_comparisons":proposals.len()*batch_modes.len(),
            "full_matrix_phase":"source-deleted", "attached_sample_executions":batch_modes.len(),
            "detached_matrix_boundary_executions":1,
            "attached_observations":attached_observations,
            "storage_combinations":plan::STORAGE_MODES,
            "maximum_argument_bytes":maximum_argument_bytes,
            "single_executions":single_modes.len(),"empty_batch_executions":batch_modes.len(),
            "refusal_executions":single_modes.len()*2,"post_refusal_recovery_executions":single_modes.len()*2,
            "binary_sha256":executable_hash,"artifact_sha256":artifact_hash,
            "execution":public.execution_evidence(),
            "source_projects_and_transports_deleted":true,"status":"passed",
            "observations":observations,
        })
        .to_string(),
    );
}
