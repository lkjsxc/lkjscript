//! Ordinary native edits retain exact incremental caches through a copied executable.

use super::native_byte_buffer::{Export, author, dependency, export, stage};
use super::native_owned_worklists::{deployment, rejected, run_expected};
use super::*;
use serde_json::json;
use sha2::{Digest, Sha256};

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn assert_native_diamond_plan(
    records: &[CompactRecord],
    base: &Export,
    left: &Export,
    right: &Export,
    root: &Export,
) {
    let plans = records
        .iter()
        .filter(|record| record.operation == "native-package-plan")
        .collect::<Vec<_>>();
    assert_eq!(
        plans.len(),
        1,
        "one actually consumed native plan: {records:?}"
    );
    let plan = plans[0];
    assert_eq!(compact_field(plan, "packages"), "4");
    assert_eq!(compact_field(plan, "edges"), "4");
    let artifact = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tools/native-package-plan/generated/package-plan.lkja");
    assert_eq!(
        compact_field(plan, "artifact-sha256"),
        sha256(&std::fs::read(artifact).unwrap())
    );
    let mut identities = [
        &base.revision,
        &left.revision,
        &right.revision,
        &root.revision,
    ];
    identities.sort();
    let position = |revision: &str| {
        i64::try_from(
            identities
                .iter()
                .position(|identity| *identity == revision)
                .unwrap(),
        )
        .unwrap()
    };
    // Exact package-revision admission owns canonical PackageId edge order.
    // The workload fixes its four edges; no topology algorithm supplies an oracle.
    let mut root_suppliers = [left, right];
    root_suppliers.sort_by(|left, right| left.package.cmp(&right.package));
    #[derive(serde::Serialize)]
    struct Node {
        id: i64,
        successors: Vec<i64>,
    }
    #[derive(serde::Serialize)]
    struct Proposal {
        roots: [i64; 1],
        nodes: Vec<Node>,
    }
    let nodes = identities
        .iter()
        .enumerate()
        .map(|(id, revision)| Node {
            id: i64::try_from(id).unwrap(),
            successors: if *revision == &root.revision {
                root_suppliers
                    .iter()
                    .map(|package| position(&package.revision))
                    .collect()
            } else if *revision == &base.revision {
                Vec::new()
            } else {
                vec![position(&base.revision)]
            },
        })
        .collect::<Vec<_>>();
    let mut branches = [left, right];
    branches.sort_by(|left, right| left.revision.cmp(&right.revision));
    let stages = json!([
        [position(&base.revision)],
        branches
            .iter()
            .map(|package| position(&package.revision))
            .collect::<Vec<_>>(),
        [position(&root.revision)],
    ]);
    let expected = json!({"case":"valid","value":{
        "components":nodes.iter().map(|node| json!({
            "members":[node.id],"cyclic":false,"dependencies":node.successors,
        })).collect::<Vec<_>>(),
        "stages":stages,"reachable":[0,1,2,3],"unreachable":[],
    }});
    assert_eq!(
        compact_field(plan, "outcome-sha256"),
        sha256(&serde_json::to_vec(&expected).unwrap())
    );
    assert_eq!(
        compact_field(plan, "input-sha256"),
        sha256(
            &serde_json::to_vec(&[Proposal {
                roots: [position(&root.revision)],
                nodes,
            }])
            .unwrap()
        )
    );
    let mut ordered_bytes = Vec::new();
    for package in [base, branches[0], branches[1], root] {
        let hex = package.revision.strip_prefix("package_revision_").unwrap();
        assert_eq!(hex.len(), 64);
        for offset in (0..64).step_by(2) {
            ordered_bytes.push(u8::from_str_radix(&hex[offset..offset + 2], 16).unwrap());
        }
    }
    assert_eq!(compact_field(plan, "order-sha256"), sha256(&ordered_bytes));
}

fn retained(template: Option<&str>) -> Native {
    let mut public = if let Some(template) = template {
        Native::template(template)
    } else {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("lkjscript");
        copy_executable(&binary(), &executable);
        let project = root.path().join("project");
        Native {
            root,
            executable,
            project,
        }
    };
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        public.root.disable_cleanup(true);
        println!(
            "native planner public evidence: {}",
            public.root.path().display()
        );
    }
    public
}

fn imported(imports: &[(&str, &Export)], body: &str) -> String {
    let mut source = String::new();
    for (name, package) in imports {
        source.push_str(&format!(
            "{}declarations.begin\n(units (use {name} {} {}))\ndeclarations.end\n",
            dependency(package),
            package.package,
            package.revision,
        ));
    }
    source.push_str(body);
    source
}

#[test]
fn native_package_planner_rebuilds_exactly_and_runs_checked_plan_after_source_deletion() {
    let public = retained(None);
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    copy_regular_tree(
        &repository.join("tools/native-package-plan/project"),
        &public.project,
    );
    for cache in ["derived", "catalog"] {
        let cache = public.project.join(cache);
        if cache.exists() {
            std::fs::remove_dir_all(cache).unwrap();
        }
    }
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let checked = public.cli(&["check"], true);
    let tests = compact_record(&checked, "tests");
    assert_eq!(compact_field(tests, "failed"), "0");
    assert_eq!(compact_field(tests, "differential"), "equal");
    assert_eq!(compact_field(tests, "passed"), "176");
    let artifact = public.root.path().join("planner.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    assert_eq!(
        std::fs::read(&artifact).unwrap(),
        std::fs::read(repository.join("tools/native-package-plan/generated/package-plan.lkja"))
            .unwrap()
    );
    assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    let descriptor = deployment(&public, "planner.lkja", "package-plan");
    let graph = json!({"roots":[10],"nodes":[
        {"id":40,"successors":[]},
        {"id":30,"successors":[40]},
        {"id":20,"successors":[40]},
        {"id":10,"successors":[20,30]},
        {"id":50,"successors":[]},
    ]});
    // Literal expected order fixes dependency direction, authored stage order,
    // and the complete unreachable closure independently of the producer.
    let expected = json!({"case":"valid","value":{
        "components":[
            {"members":[40],"cyclic":false,"dependencies":[]},
            {"members":[30],"cyclic":false,"dependencies":[40]},
            {"members":[20],"cyclic":false,"dependencies":[40]},
            {"members":[10],"cyclic":false,"dependencies":[20,30]},
            {"members":[50],"cyclic":false,"dependencies":[]},
        ],"stages":[[40,50],[30,20],[10]],
        "reachable":[40,30,20,10],"unreachable":[50],
    }});
    for detached in [false, true] {
        if detached {
            std::fs::remove_dir_all(&public.project).unwrap();
        }
        run_expected(
            &public,
            "package-plan",
            &descriptor,
            detached,
            "authored-diamond-and-unreachable",
            json!([graph.clone()]),
            &expected,
        );
        run_expected(
            &public,
            "package-plan",
            &descriptor,
            detached,
            "unreachable-malformed-edge",
            json!([{"roots":[1],"nodes":[
                {"id":1,"successors":[]},{"id":2,"successors":[99]},
            ]}]),
            &json!({"case":"invalid","value":{
                "code":"missing-successor","owner":2,"target":99,
            }}),
        );
    }
}

#[test]
fn native_package_planner_compiles_independently_authored_diamond_and_detached_results() {
    let base = retained(Some("minimal"));
    author(&base, include_str!("../fixtures/package-plan-base.lkjc"));
    let base_package = export(&base);
    let left = retained(Some("minimal"));
    stage(&left, &base_package);
    author(
        &left,
        &imported(
            &[("base", &base_package)],
            include_str!("../fixtures/package-plan-left.lkjc"),
        ),
    );
    let left_package = export(&left);
    let right = retained(Some("minimal"));
    stage(&right, &base_package);
    author(
        &right,
        &imported(
            &[("base", &base_package)],
            include_str!("../fixtures/package-plan-right.lkjc"),
        ),
    );
    let right_package = export(&right);
    let consumer = retained(Some("minimal"));
    // Transitive base admission arrives through independently exported suppliers.
    // No direct base stage or dependency can substitute for the actual diamond.
    stage(&consumer, &right_package);
    stage(&consumer, &left_package);
    consumer.cli(&["check"], true);
    let body = imported(
        &[("right", &right_package), ("left", &left_package)],
        include_str!("../fixtures/package-plan-consumer.lkjc"),
    );
    let before = consumer.revision();
    let inventory = content_inventory(&consumer.project);
    rejected(
        &consumer,
        &body.replacen(
            "(call right::transform (local n))",
            "(call right::transform (local n) (i64 7))",
            1,
        ),
        &before,
        "diamond-wrong-supplier-arity",
    );
    assert_eq!(content_inventory(&consumer.project), inventory);
    author(&consumer, &body);
    let root_package = export(&consumer);
    assert_native_diamond_plan(
        &consumer.cli(&["check"], true),
        &base_package,
        &left_package,
        &right_package,
        &root_package,
    );
    let draft = consumer.root.path().join("diamond-draft.lkjc");
    consumer.cli(
        &[
            "change",
            "draft",
            "--module",
            "diamond",
            "--output",
            path(&draft),
        ],
        true,
    );
    assert_eq!(
        compact_field(
            compact_record(&consumer.plan(&draft, true), "result"),
            "outcome"
        ),
        "unchanged"
    );
    let artifact = consumer.root.path().join("diamond.lkja");
    let repeated = consumer.root.path().join("diamond-repeated.lkja");
    for output in [&artifact, &repeated] {
        assert_native_diamond_plan(
            &consumer.cli(&["build", "--output", path(output)], true),
            &base_package,
            &left_package,
            &right_package,
            &root_package,
        );
    }
    assert_eq!(
        std::fs::read(&artifact).unwrap(),
        std::fs::read(repeated).unwrap()
    );
    let descriptor = deployment(&consumer, "diamond.lkja", "diamond");
    for detached in [false, true] {
        if detached {
            for public in [&base, &left, &right, &consumer] {
                std::fs::remove_dir_all(&public.project).unwrap();
                for entry in std::fs::read_dir(public.root.path()).unwrap() {
                    let path = entry.unwrap().path();
                    if path
                        .extension()
                        .is_some_and(|extension| extension == "lkjc")
                    {
                        std::fs::remove_file(path).unwrap();
                    }
                }
            }
            for package in [&base_package, &left_package, &right_package, &root_package] {
                std::fs::remove_file(&package.path).unwrap();
            }
        }
        for n in [-19, 0, 27, 10_000] {
            run_expected(
                &consumer,
                "diamond",
                &descriptor,
                detached,
                &format!("independent-{n}"),
                json!([n]),
                &json!({"left":n+11,"right":n+31,"total":2*n+42}),
            );
        }
    }
}

fn source(ports: usize, target: bool) -> String {
    let ports = (0..ports)
        .map(|index| format!("(port create p{index} (type (function () I64)) (function one))"))
        .collect::<Vec<_>>()
        .join("\n");
    let target = if target {
        "(target create extra-main (component extra::console) (runner command) \
         (port extra::console::p0))"
    } else {
        ""
    };
    format!(
        "declarations.begin\n(units \
        (module create extra \
          (function create one (visibility public) (returns I64) \
            (effect pure) (body (i64 1))) \
          (component create console (visibility private) {ports})) \
        {target})\ndeclarations.end\n"
    )
}

fn apply_exact(public: &Native, name: &str, source: &str, units: usize) {
    let input = public.input(
        name,
        &format!("request base={}\n{source}", public.revision()),
    );
    let plan = public.plan(&input, true);
    assert_eq!(
        compact_field(compact_record(&plan, "validation"), "compiler-units"),
        units.to_string()
    );
    let applied = public.apply(&input, &plan, true);
    assert_eq!(compact_field(&applied[0], "status"), "accepted");
    assert_eq!(
        compact_field(compact_record(&applied, "derived-cache"), "status"),
        "updated"
    );
    let checked = public.cli(&["check"], true);
    assert_eq!(
        compact_field(compact_record(&checked, "compilation"), "cache"),
        "exact-current"
    );
    assert_eq!(
        compact_field(compact_record(&checked, "tests"), "failed"),
        "0"
    );
}

#[test]
fn new_components_do_not_promote_ports_to_compiler_units() {
    for ports in [1, 2] {
        let public = Native::template("command");
        let baseline = public.cli(&["check"], true);
        apply_exact(&public, "new-component.lkjc", &source(ports, false), 2);
        let checked = public.cli(&["check"], true);
        assert_eq!(
            compact_field(compact_record(&checked, "tests"), "passed"),
            compact_field(compact_record(&baseline, "tests"), "passed")
        );
    }
}

#[test]
fn new_command_targets_keep_exact_incremental_cache_and_detached_behavior() {
    for ports in [1, 2] {
        let public = Native::template("command");
        public.cli(&["check"], true);
        apply_exact(&public, "new-command.lkjc", &source(ports, true), 3);
        let attached = public.cli(&["run", "extra-main"], true);
        assert_eq!(
            compact_field(compact_record(&attached, "execution"), "value"),
            "1"
        );
        assert_eq!(
            compact_field(compact_record(&attached, "execution"), "differential"),
            "equal"
        );
        let bundle = public.root.path().join("bundle");
        std::fs::create_dir(&bundle).unwrap();
        public.cli(
            &["build", "--output", path(&bundle.join("command.lkja"))],
            true,
        );
        let mut descriptor: Value = serde_json::from_slice(
            &std::fs::read(public.project.join("command.deployment.json")).unwrap(),
        )
        .unwrap();
        descriptor["artifact"] = serde_json::json!("command.lkja");
        descriptor["target"] = serde_json::json!("extra-main");
        let deployment = bundle.join("command.deployment.json");
        std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
        std::fs::remove_dir_all(&public.project).unwrap();
        let detached = public.cli(&["run", "--deployment", path(&deployment)], true);
        assert_eq!(
            compact_field(compact_record(&detached, "execution"), "value"),
            "1"
        );
    }
}

#[test]
fn new_port_type_failure_preserves_authority_and_cache_then_recovers() {
    let public = Native::template("command");
    public.cli(&["check"], true);
    let before = content_inventory(&public.project);
    let invalid = public.input(
        "invalid-port.lkjc",
        &format!(
            "request base={}\n{}",
            public.revision(),
            source(1, false).replace("(function () I64)", "(function () Bool)"),
        ),
    );
    let rejected = public.plan(&invalid, false);
    assert!(
        rejected
            .iter()
            .any(|record| record.operation == "diagnostic"
                && super::super::compact_field(record, "code")
                    == Some("kernel_type_port_function")),
        "{rejected:?}"
    );
    assert_eq!(content_inventory(&public.project), before);
    apply_exact(&public, "valid-port.lkjc", &source(1, false), 2);
}

#[cfg(unix)]
#[path = "incremental_cache_files.rs"]
mod cache_files;
