use crate::components::{Dependency, Native, field, path};
use serde_json::{Value, json};

pub mod cases;
pub mod literal_sources;
pub mod oracle;

pub const SOURCES: [(&str, &str); 10] = [
    (
        "component-order",
        include_str!("../../examples/dependency-components/order.lkjc"),
    ),
    (
        "component-groups",
        include_str!("../../examples/dependency-components/groups.lkjc"),
    ),
    (
        "component-data",
        include_str!("../../examples/dependency-components/index.lkjc"),
    ),
    (
        "dependency-components",
        include_str!("../../examples/dependency-components/application.lkjc"),
    ),
    (
        "component-batch",
        include_str!("../../examples/dependency-components/batch.lkjc"),
    ),
    (
        "plan-edges",
        include_str!("../../examples/dependency-plan/edges.lkjc"),
    ),
    (
        "plan-stages",
        include_str!("../../examples/dependency-plan/stages.lkjc"),
    ),
    (
        "plan-presentation",
        include_str!("../../examples/dependency-plan/presentation.lkjc"),
    ),
    (
        "dependency-plan",
        include_str!("../../examples/dependency-plan/application.lkjc"),
    ),
    (
        "plan-batch",
        include_str!("../../examples/dependency-plan/batch.lkjc"),
    ),
];
pub const DESCRIPTORS: [(&str, &str); 4] = [
    (
        "flat",
        include_str!("../../examples/dependency-plan/flat.deployment.json"),
    ),
    (
        "chunked",
        include_str!("../../examples/dependency-plan/chunked.deployment.json"),
    ),
    (
        "batch-flat",
        include_str!("../../examples/dependency-plan/batch-flat.deployment.json"),
    ),
    (
        "batch-chunked",
        include_str!("../../examples/dependency-plan/batch-chunked.deployment.json"),
    ),
];

pub const STORAGE_MODES: [&str; 4] = [
    "storage-flat-flat",
    "storage-flat-chunked",
    "storage-chunked-flat",
    "storage-chunked-chunked",
];
pub const STORAGE_DESCRIPTORS: [(&str, &str); 8] = [
    (
        "storage-flat-flat",
        include_str!("../../examples/dependency-plan/storage-flat-flat.deployment.json"),
    ),
    (
        "storage-flat-chunked",
        include_str!("../../examples/dependency-plan/storage-flat-chunked.deployment.json"),
    ),
    (
        "storage-chunked-flat",
        include_str!("../../examples/dependency-plan/storage-chunked-flat.deployment.json"),
    ),
    (
        "storage-chunked-chunked",
        include_str!("../../examples/dependency-plan/storage-chunked-chunked.deployment.json"),
    ),
    (
        "batch-storage-flat-flat",
        include_str!("../../examples/dependency-plan/batch-storage-flat-flat.deployment.json"),
    ),
    (
        "batch-storage-flat-chunked",
        include_str!("../../examples/dependency-plan/batch-storage-flat-chunked.deployment.json"),
    ),
    (
        "batch-storage-chunked-flat",
        include_str!("../../examples/dependency-plan/batch-storage-chunked-flat.deployment.json"),
    ),
    (
        "batch-storage-chunked-chunked",
        include_str!(
            "../../examples/dependency-plan/batch-storage-chunked-chunked.deployment.json"
        ),
    ),
];

pub fn author_storage(public: &Native, standard: &Dependency) -> [Dependency; 3] {
    let library = public.project("storage-library");
    public.stage(&library, standard);
    public.author(
        &library,
        include_str!("../../examples/indexed-data-storage/library.lkjc"),
        &[("builtin", standard)],
        true,
    );
    public.check(&library);
    public.unchanged(&library, "indexed-data");
    let library_export = public.export(Some(&library), "storage-library");
    let mut exports = Vec::new();
    for (name, module, source) in [
        (
            "storage-flat",
            "indexed-flat",
            include_str!("../../examples/indexed-data-storage/flat.lkjc"),
        ),
        (
            "storage-chunked",
            "indexed-chunked",
            include_str!("../../examples/indexed-data-storage/chunked.lkjc"),
        ),
    ] {
        let project = public.project(name);
        let dependencies = [("builtin", standard), ("indexed-data", &library_export)];
        for (_, dependency) in &dependencies {
            public.stage(&project, dependency);
        }
        public.author(&project, source, &dependencies, true);
        public.check(&project);
        public.unchanged(&project, module);
        exports.push(public.export(Some(&project), name));
    }
    [library_export, exports.remove(0), exports.remove(0)]
}

pub fn detach(public: &Native) {
    for name in ["storage-library", "storage-flat", "storage-chunked"] {
        std::fs::remove_dir_all(public.root.join(name)).unwrap();
        std::fs::remove_file(public.root.join(format!("{name}.lkjp"))).unwrap();
    }
    public.detach();
}

pub fn author(public: &Native) {
    let standard = public.export(None, "standard");
    let storage = author_storage(public, &standard);
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
    let application = public.project("application");
    let dependencies = [
        ("builtin", &standard),
        ("owned-worklists", &library_export),
        ("worklist-carriers", &carriers_export),
        ("indexed-data", &storage[0]),
        ("indexed-flat", &storage[1]),
        ("indexed-chunked", &storage[2]),
    ];
    for (_, dependency) in &dependencies {
        public.stage(&application, dependency);
    }
    public.author(
        &application,
        &literal_sources::grouped(
            std::iter::once(include_str!(
                "../../examples/owned-worklists/reachability.lkjc"
            ))
            .chain(SOURCES[..5].iter().map(|(_, source)| *source)),
        ),
        &dependencies,
        true,
    );
    // Keep both source families inside the ordinary per-request validation-work
    // admission. Their combined request exceeds the unchanged affine-work limit.
    public.author(
        &application,
        &literal_sources::grouped(SOURCES[5..].iter().map(|(_, source)| *source)),
        &dependencies,
        false,
    );
    public.check(&application);
    for (module, _) in &SOURCES[5..] {
        public.unchanged(&application, module);
    }
    public.cli(
        &application,
        &[
            "build",
            "--output",
            path(&public.root.join("dependency-plan.lkja")),
        ],
        true,
    );
}

pub fn refusals(public: &Native, descriptors: &[(&str, &std::path::PathBuf)]) {
    let wrong = public.input("wrong-type.json",
        "[{\"roots\":[],\"nodes\":[{\"id\":1,\"successors\":[]},{\"id\":2,\"successors\":[false]}]}]");
    let proposal = crate::components::cases::example();
    let valid = public.input("recovery.json", &json!([proposal]).to_string());
    let expected = oracle::expected(&proposal);
    for &(mode, descriptor) in descriptors {
        let absent = public.root.join(format!("wrong-{mode}-result.json"));
        public.cli(
            &public.root,
            &[
                "run",
                "--deployment",
                path(descriptor),
                "--arguments-file",
                path(&wrong),
                "--result-file",
                path(&absent),
            ],
            false,
        );
        assert!(!absent.exists());
        public.compare(&format!("after-type-{mode}"), descriptor, &valid, &expected);
        let mut restricted: Value = serde_json::from_str(DESCRIPTORS[0].1).unwrap();
        restricted["target"] = json!(format!("dependency-plan-{mode}"));
        restricted["execution"] =
            json!({"instruction_fuel":1,"maximum_call_depth":4096,"maximum_value_stack":1000000});
        let restricted = public.input(
            &format!("refusal-{mode}.deployment.json"),
            &restricted.to_string(),
        );
        let absent = public.root.join(format!("refusal-{mode}-result.json"));
        let failure = public.cli(
            &public.root,
            &[
                "run",
                "--deployment",
                path(&restricted),
                "--arguments-file",
                path(&valid),
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
        public.compare(
            &format!("after-refusal-{mode}"),
            descriptor,
            &valid,
            &expected,
        );
    }
}
