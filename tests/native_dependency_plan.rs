//! Native dependency-first planning, independently checked outside the source tree.
#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "native_components_support/mod.rs"]
mod components;
#[path = "native_plan_support/mod.rs"]
mod plan;
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
    let mut observations = Vec::new();
    for phase in ["attached", "source-deleted"] {
        if phase == "source-deleted" {
            public.detach();
        }
        for (mode, descriptor) in ["flat", "chunked"].into_iter().zip(&descriptors[2..]) {
            for (index, (arguments, expected)) in batches.iter().enumerate() {
                let output = public.compare(
                    &format!("{phase}-{mode}-{index:02}"),
                    descriptor,
                    arguments,
                    expected,
                );
                observations.push(json!({"phase":phase,"mode":mode,"batch":index,
                    "graphs":expected.as_array().unwrap().len(),
                    "observation":quoted(&output,"production-observation")}));
            }
        }
    }
    let example = components::cases::example();
    let example_arguments = public.input("example.json", &json!([example]).to_string());
    let empty_arguments = public.input("empty-batch.json", "[[]]");
    for (mode, descriptor) in ["flat", "chunked"].into_iter().zip(&descriptors[..2]) {
        public.compare(
            &format!("single-{mode}"),
            descriptor,
            &example_arguments,
            &plan::oracle::expected(&example),
        );
    }
    for (mode, descriptor) in ["flat", "chunked"].into_iter().zip(&descriptors[2..]) {
        public.compare(
            &format!("empty-{mode}"),
            descriptor,
            &empty_arguments,
            &json!([]),
        );
    }
    plan::refusals(&public, &descriptors);
    assert_eq!(artifact_hash, digest(&artifact));
    assert_eq!(executable_hash, digest(&public.root.join("lkjscript")));
    public.input(
        "witness-summary.json",
        &json!({
            "purpose":"Native dependency-plan evidence, not source or release acceptance",
            "graphs":proposals.len(), "exhaustive_three_vertex_graphs":512,
            "exhaustive_loop_free_four_vertex_graphs":4096,
            "matched_batch_executions":batches.len()*4, "proposal_comparisons":proposals.len()*4,
            "maximum_argument_bytes":maximum_argument_bytes,
            "single_executions":2,"empty_batch_executions":2,
            "refusal_executions":4,"post_refusal_recovery_executions":4,
            "binary_sha256":executable_hash,"artifact_sha256":artifact_hash,
            "source_projects_and_transports_deleted":true,"status":"passed",
            "observations":observations,
        })
        .to_string(),
    );
}
