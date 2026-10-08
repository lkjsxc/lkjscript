use super::{CONSUMER, DESCRIPTORS, VERIFIER, cases};
use crate::components::{Native, digest, path};
use serde_json::json;

pub fn author(public: &Native) {
    let standard = public.export(None, "standard");
    let library = public.project("library");
    public.stage(&library, &standard);
    public.author(
        &library,
        include_str!("../../examples/owned-worklists/library.lkjc"),
        &[("builtin", &standard)],
        true,
    );
    public.check(&library);
    let library_export = public.export(Some(&library), "library");
    let carriers = public.project("carriers");
    let dependencies = [("builtin", &standard), ("owned-worklists", &library_export)];
    for (_, dependency) in &dependencies {
        public.stage(&carriers, dependency);
    }
    public.author(
        &carriers,
        include_str!("../../examples/owned-worklists/carriers.lkjc"),
        &dependencies,
        true,
    );
    public.check(&carriers);
    let carriers_export = public.export(Some(&carriers), "carriers");
    let verifier = public.project("verifier");
    let dependencies = [
        ("builtin", &standard),
        ("owned-worklists", &library_export),
        ("worklist-carriers", &carriers_export),
    ];
    for (_, dependency) in &dependencies {
        public.stage(&verifier, dependency);
    }
    public.author(
        &verifier,
        include_str!("../../examples/owned-worklists/reachability.lkjc"),
        &dependencies,
        true,
    );
    for (_, source) in VERIFIER {
        public.author(&verifier, source, &dependencies, false);
    }
    public.check(&verifier);
    for (module, _) in VERIFIER {
        public.unchanged(&verifier, module);
    }
    let artifact = public.root.join("dependency-plan-verifier.lkja");
    public.cli(&verifier, &["build", "--output", path(&artifact)], true);
    // The public checker is executable before any SCC/planner source or artifact exists.
    assert!(!public.root.join("application").exists());
    assert!(!public.root.join("verified-dependency-plan.lkja").exists());
    let descriptor = public.input("standalone-first.deployment.json", DESCRIPTORS[4].1);
    let arguments = public.input(
        "standalone-first.json",
        &json!([
            {"roots":[],"nodes":[]}, cases::empty_plan()
        ])
        .to_string(),
    );
    public.compare(
        "standalone-before-producer",
        &descriptor,
        &arguments,
        &cases::verified(),
    );
    public.input("standalone-before-producer-evidence.json", &json!({
        "verifier_artifact_sha256":digest(&artifact),
        "producer_project_absent":true,"producer_artifact_absent":true,
        "authored_verifier_modules":VERIFIER.iter().map(|(name,_)| name).collect::<Vec<_>>(),
        "shared_source_admission":"provisional-graph::flat",
        "independent_of":"component-order, component-groups, component-data, dependency-components, plan-edges, plan-stages, plan-presentation, dependency-plan",
    }).to_string());
    let verifier_export = public.export(Some(&verifier), "verifier");
    let application = public.project("application");
    let dependencies = [
        ("builtin", &standard),
        ("owned-worklists", &library_export),
        ("worklist-carriers", &carriers_export),
        ("dependency-verifier", &verifier_export),
    ];
    for (_, dependency) in &dependencies {
        public.stage(&application, dependency);
    }
    public.author(
        &application,
        include_str!("../../examples/owned-worklists/reachability.lkjc"),
        &dependencies,
        true,
    );
    for (_, source) in crate::plan::SOURCES {
        public.author(&application, source, &dependencies, false);
    }
    for (_, source) in CONSUMER {
        public.author(&application, source, &dependencies, false);
    }
    public.check(&application);
    for (module, _) in CONSUMER {
        public.unchanged(&application, module);
    }
    public.cli(
        &application,
        &[
            "build",
            "--output",
            path(&public.root.join("verified-dependency-plan.lkja")),
        ],
        true,
    );
}
