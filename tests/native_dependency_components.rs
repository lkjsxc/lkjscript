//! Native compiler-tooling witness; the host supplies only inputs and an independent oracle.
#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "native_components_support/mod.rs"]
mod support;
use serde_json::{Value, json};
use std::fs;
use support::{Native, cases, digest, field, oracle, path, quoted};

const LIBRARY: &str = include_str!("../examples/owned-worklists/library.lkjc");
const CARRIERS: &str = include_str!("../examples/owned-worklists/carriers.lkjc");
const REACH: &str = include_str!("../examples/owned-worklists/reachability.lkjc");
const SOURCES: [(&str, &str); 5] = [
    (
        "component-order",
        include_str!("../examples/dependency-components/order.lkjc"),
    ),
    (
        "component-groups",
        include_str!("../examples/dependency-components/groups.lkjc"),
    ),
    (
        "component-data",
        include_str!("../examples/dependency-components/index.lkjc"),
    ),
    (
        "dependency-components",
        include_str!("../examples/dependency-components/application.lkjc"),
    ),
    (
        "component-batch",
        include_str!("../examples/dependency-components/batch.lkjc"),
    ),
];
const DESCRIPTORS: [(&str, &str); 4] = [
    (
        "flat",
        include_str!("../examples/dependency-components/flat.deployment.json"),
    ),
    (
        "chunked",
        include_str!("../examples/dependency-components/chunked.deployment.json"),
    ),
    (
        "batch-flat",
        include_str!("../examples/dependency-components/batch-flat.deployment.json"),
    ),
    (
        "batch-chunked",
        include_str!("../examples/dependency-components/batch-chunked.deployment.json"),
    ),
];

#[test]
fn native_components_partition_complete_candidates_across_storage_and_detachment() {
    let public = Native::new();
    let executable_hash = digest(&public.root.join("lkjscript"));
    let standard = public.export(None, "standard");
    let library = public.project("library");
    public.stage(&library, &standard);
    public.author(&library, LIBRARY, &[("builtin", &standard)], true);
    public.check(&library);
    let library_export = public.export(Some(&library), "library");
    let carriers = public.project("carriers");
    let carrier_dependencies = [("builtin", &standard), ("owned-worklists", &library_export)];
    for (_, dependency) in &carrier_dependencies {
        public.stage(&carriers, dependency);
    }
    public.author(&carriers, CARRIERS, &carrier_dependencies, true);
    public.check(&carriers);
    let carriers_export = public.export(Some(&carriers), "carriers");
    let application = public.project("application");
    let dependencies = [
        ("builtin", &standard),
        ("owned-worklists", &library_export),
        ("worklist-carriers", &carriers_export),
    ];
    for (_, dependency) in &dependencies {
        public.stage(&application, dependency);
    }
    public.author(&application, REACH, &dependencies, true);
    for (_, source) in SOURCES {
        public.author(&application, source, &dependencies, false);
    }
    public.check(&application);
    for (module, _) in SOURCES {
        public.unchanged(&application, module);
    }
    let artifact = public.root.join("dependency-components.lkja");
    public.cli(&application, &["build", "--output", path(&artifact)], true);
    let artifact_hash = digest(&artifact);
    let descriptors =
        DESCRIPTORS.map(|(name, text)| public.input(&format!("{name}.deployment.json"), text));

    let proposals = cases::suite();
    assert_eq!(proposals.len(), 532);
    let expected = Value::Array(proposals.iter().map(oracle::expected).collect());
    let arguments = public.input("suite.json", &json!([proposals]).to_string());
    public.input("expected-suite.json", &expected.to_string());
    let mut observations = Vec::new();
    for (mode, descriptor) in ["flat", "chunked"].into_iter().zip(&descriptors[2..]) {
        let output = public.compare(
            &format!("attached-{mode}"),
            descriptor,
            &arguments,
            &expected,
        );
        observations.push(json!({"phase":"attached", "mode":mode,
            "observation":quoted(&output, "production-observation")}));
    }
    public.detach();
    let empty_arguments = public.input("empty-batch.json", "[[]]");
    for (mode, descriptor) in ["flat", "chunked"].into_iter().zip(&descriptors[2..]) {
        let output = public.compare(
            &format!("detached-{mode}"),
            descriptor,
            &arguments,
            &expected,
        );
        observations.push(json!({"phase":"source-deleted", "mode":mode,
            "observation":quoted(&output, "production-observation")}));
        public.compare(
            &format!("empty-{mode}"),
            descriptor,
            &empty_arguments,
            &json!([]),
        );
    }
    let example = cases::example();
    let example_arguments = public.input("example.json", &json!([example]).to_string());
    for (mode, descriptor) in ["flat", "chunked"].into_iter().zip(&descriptors[..2]) {
        public.compare(
            &format!("single-{mode}"),
            descriptor,
            &example_arguments,
            &oracle::expected(&example),
        );
    }
    let wrong = public.input(
        "wrong-type.json",
        "[{\"roots\":[],\"nodes\":[{\"id\":true,\"successors\":[]}]}]",
    );
    let absent = public.root.join("wrong-type-result.json");
    public.cli(
        &public.root,
        &[
            "run",
            "--deployment",
            path(&descriptors[0]),
            "--arguments-file",
            path(&wrong),
            "--result-file",
            path(&absent),
        ],
        false,
    );
    assert!(!absent.exists());
    // Explicit execution refusal is not a successful typed graph-capacity outcome.
    let mut restricted: Value = serde_json::from_str(DESCRIPTORS[0].1).unwrap();
    restricted["execution"] =
        json!({"instruction_fuel":1,"maximum_call_depth":4096,"maximum_value_stack":1000000});
    let descriptor = public.input("fuel-refusal.deployment.json", &restricted.to_string());
    let absent = public.root.join("fuel-refusal-result.json");
    let failure = public.cli(
        &public.root,
        &[
            "run",
            "--deployment",
            path(&descriptor),
            "--arguments-file",
            path(&example_arguments),
            "--result-file",
            path(&absent),
        ],
        false,
    );
    assert_eq!(
        field(&failure, "diagnostic", "class"),
        "resource",
        "{failure}"
    );
    assert_eq!(
        field(&failure, "diagnostic", "code"),
        "normalized_instruction_steps",
        "{failure}"
    );
    assert!(!absent.exists());
    assert_eq!(artifact_hash, digest(&artifact));
    assert_eq!(executable_hash, digest(&public.root.join("lkjscript")));
    public.input("witness-summary.json", &json!({
        "purpose":"Native witness evidence, not a source-verification or release-acceptance receipt",
        "graphs":proposals.len(), "exhaustive_three_vertex_graphs":512,
        "matched_batch_executions":4, "single_executions":2, "empty_batch_executions":2,
        "binary_sha256":executable_hash, "artifact_sha256":artifact_hash,
        "source_projects_and_transports_deleted":true, "status":"passed",
        "observations":observations,
        "source_bytes":SOURCES.iter().map(|(name, source)| json!({"module":name,"bytes":source.len()})).collect::<Vec<_>>()
    }).to_string());
    assert!(fs::metadata(artifact).unwrap().len() > 0);
}
