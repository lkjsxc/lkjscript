//! Runtime-sized owned collections through fresh native packages and copied executables.
use super::native_byte_buffer::{Export, author, dependency, export, stage};
use super::*;
use serde_json::json;

const LIBRARY: &str = include_str!("../../examples/owned-sequences/library.lkjc");
const CARRIERS: &str = include_str!("../../examples/owned-sequences/carriers.lkjc");
const APPLICATION: &str = include_str!("../../examples/owned-sequences/application.lkjc");
const TASKS: &str = include_str!("../../examples/owned-sequences/tasks.lkjc");

struct Packages {
    library: Native,
    carriers: Native,
    consumer: Native,
    generic: Export,
    concrete: Export,
    standard: PathBuf,
}

impl Packages {
    fn stage() -> Self {
        let library = Native::new();
        let standard = library.root.path().join("standard.lkjp");
        let exported = library.cli(
            &[
                "package",
                "builtin",
                "export",
                "--kind",
                "transport",
                "--output",
                path(&standard),
            ],
            true,
        );
        let builtin = compact_record(&exported, "package");
        library.cli(
            &[
                "package",
                "dependency",
                "stage",
                "--transport",
                compact_field(builtin, "transport"),
                "--input-file",
                path(&standard),
            ],
            true,
        );
        author(
            &library,
            &format!(
                "add.dependency package={} semantic-revision={} package-revision={}\n{LIBRARY}",
                compact_field(builtin, "id"),
                compact_field(builtin, "revision"),
                compact_field(builtin, "package-revision"),
            ),
        );
        let original = unchanged(&library, "owned-sequences");
        for absent in ["OwnedI64Cell", "ByteBuffer", "owned-implementation"] {
            assert!(!original.contains(absent), "generic library: {absent}");
        }
        for form in [
            "sequence-empty",
            "sequence-length",
            "sequence-push",
            "sequence-pop",
            "borrow-owned-item",
        ] {
            assert!(original.contains(form), "generic draft: {form}");
        }
        assert_eq!(
            compact_field(
                compact_record(&library.cli(&["status"], true), "summary"),
                "dependencies"
            ),
            "1",
        );
        // The open generic library is admitted and exported before concrete carriers exist.
        let generic = export(&library);
        let carriers = Native::new();
        stage(&carriers, &generic);
        author(
            &carriers,
            &format!(
                "{}declarations.begin\n(units (use owned-sequences {} {}))\ndeclarations.end\n{CARRIERS}",
                dependency(&generic),
                generic.package,
                generic.revision,
            ),
        );
        unchanged(&carriers, "sequence-carriers");
        let concrete = export(&carriers);
        let consumer = Native::template("command");
        stage(&consumer, &generic);
        stage(&consumer, &concrete);
        Self {
            library,
            carriers,
            consumer,
            generic,
            concrete,
            standard,
        }
    }

    fn source(&self, body: &str) -> String {
        format!(
            "{}{}declarations.begin\n(units (use owned-sequences {} {}) (use sequence-carriers {} {}))\ndeclarations.end\n{body}",
            dependency(&self.generic),
            dependency(&self.concrete),
            self.generic.package,
            self.generic.revision,
            self.concrete.package,
            self.concrete.revision,
        )
    }

    fn detach(&self) {
        for project in [
            &self.library.project,
            &self.carriers.project,
            &self.consumer.project,
        ] {
            std::fs::remove_dir_all(project).unwrap();
        }
        for package in [&self.generic.path, &self.concrete.path, &self.standard] {
            std::fs::remove_file(package).unwrap();
        }
    }

    fn retain(self, label: &str) {
        if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
            for public in [self.library, self.carriers, self.consumer] {
                println!(
                    "retained owned-sequence {label} public evidence: {}",
                    public.root.keep().display()
                );
            }
        }
    }
}

fn unchanged(public: &Native, module: &str) -> String {
    let directory = tempfile::Builder::new()
        .prefix(&format!("{module}-{}-draft-", public.revision()))
        .tempdir_in(public.root.path())
        .unwrap()
        .keep();
    let draft = directory.join("draft.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--module",
            module,
            "--output",
            path(&draft),
        ],
        true,
    );
    let plan = public.plan(&draft, true);
    assert_eq!(
        compact_field(compact_record(&plan, "result"), "outcome"),
        "unchanged"
    );
    std::fs::read_to_string(draft).unwrap()
}

fn expected(inputs: &[i64], marker: i64) -> Value {
    let sum: i64 = inputs.iter().sum();
    let mut drain = vec![11];
    drain.extend(inputs.iter().rev().skip(1).copied());
    let scenario = |observed| {
        json!({
            "first":observed,"second":observed,"popped":inputs.last().copied().unwrap_or(-1),
            "drain":drain,"empty-length":0,"reused":[17],
        })
    };
    json!({
        "scalar":scenario(sum),"bytes":scenario(sum),
        "alternate":scenario(99 * inputs.len() as i64),"nested":sum,"marker":marker,
    })
}

fn clean_execution(records: &[CompactRecord], calls: u64, parallel: bool) {
    let execution = compact_record(records, "execution");
    let Some(observation) = super::super::compact_field(execution, "production-observation") else {
        // Pure project execution reports differential work; the detached runs below
        // separately report and check full foreground lifecycle observations.
        assert_eq!(calls, 0);
        assert!(!parallel);
        assert_eq!(compact_field(execution, "differential"), "equal");
        assert!(
            compact_field(execution, "production-instructions")
                .parse::<u64>()
                .unwrap()
                > 0
        );
        assert!(
            compact_field(execution, "reference-expressions")
                .parse::<u64>()
                .unwrap()
                > 0
        );
        return;
    };
    let observation: Value = serde_json::from_str(observation).unwrap();
    assert_eq!(observation["capability_calls"], json!(calls));
    assert_eq!(observation["live_handles_after"], json!(0));
    if parallel {
        assert_eq!(observation["parallel_scopes"], json!(1));
        assert!(observation["parallel_worker_dispatches"].as_u64().unwrap() > 0);
    }
    let cleanup: Value = serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
    assert_eq!(cleanup["remaining_tasks"], json!(0));
    assert_eq!(cleanup["cleanup_failures"], json!([]));
}

#[test]
fn native_owned_sequence_three_packages_generic_reads_pop_drain_reuse_and_detach() {
    let packages = Packages::stage();
    author(&packages.consumer, &packages.source(APPLICATION));
    let public = &packages.consumer;
    let discovery = public.cli(&["capabilities", "--section", "change"], true);
    for syntax in [
        "sequence-empty",
        "sequence-length",
        "sequence-push",
        "sequence-pop",
        "borrow-owned-item",
    ] {
        assert!(
            discovery.iter().any(|record| record
                .fields
                .iter()
                .any(|field| field.value.contains(syntax))),
            "discovery: {syntax}"
        );
    }
    for name in [
        "sequence-empty",
        "sequence-length",
        "sequence-push",
        "sequence-pop",
    ] {
        let owners = public.cli(
            &["package", "builtin", "query", "owners", "--name", name],
            true,
        );
        assert_eq!(
            owners
                .iter()
                .filter(|record| record.operation == "owner")
                .count(),
            1,
            "{name}"
        );
    }
    unchanged(public, "owned-sequences-app");
    let artifact = public.root.path().join("owned-sequences.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let deployment = public.input(
        "owned-sequences.deployment.json",
        include_str!("../../examples/owned-sequences/owned-sequences.deployment.json"),
    );
    let workloads = [
        vec![],
        vec![2],
        vec![2, 3, 7],
        vec![0, 255, 128],
        (0..513).map(|n| n % 17).collect(),
    ];
    for detached in [false, true] {
        if detached {
            packages.detach();
        }
        for (case, inputs) in workloads.iter().enumerate() {
            let arguments = public.input(
                &format!("arguments-{detached}-{case}.json"),
                &json!([inputs]).to_string(),
            );
            let result = public
                .root
                .path()
                .join(format!("result-{detached}-{case}.json"));
            let mut args = if detached {
                vec!["run", "--deployment", path(&deployment)]
            } else {
                vec!["run", "owned-sequences"]
            };
            args.extend([
                "--arguments-file",
                path(&arguments),
                "--result-file",
                path(&result),
            ]);
            let records = public.cli(&args, true);
            // This expected result is independent of the generic algorithm, carrier methods and compiler.
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(result).unwrap()).unwrap(),
                expected(inputs, 41)
            );
            clean_execution(&records, 0, false);
        }
    }
    packages.retain("three-packages");
}

fn rejected(public: &Native, source: &str, before: &str, name: &str) {
    let request = public.input(
        &format!("invalid-{name}.lkjc"),
        &format!("request base={before}\n{source}"),
    );
    let records = public.plan(&request, false);
    assert!(
        records.iter().any(|record| record.operation == "diagnostic"
            && compact_field(record, "code").starts_with("kernel_")),
        "{name}: {records:?}"
    );
    assert_eq!(public.revision(), before, "{name}");
}

const READ_SCOPE: &str = r#"declarations.begin
(units (module create invalid-sequence
  (function create read (visibility public) (effect pure)
    (parameter create values (type (owned-sequence OwnedI64Cell)) (use consume)) (returns I64)
    (body (borrow-owned-item (type (owned-sequence OwnedI64Cell)) (local values)
      (index (i64 0)) (binding view (type OwnedI64Cell))
      (in (call sequence-carriers::cell-read (local view))))))))
declarations.end
"#;

#[test]
fn native_owned_sequence_rejects_invalid_meaning_preserves_draft_ids_and_recovers_after_bounds() {
    let packages = Packages::stage();
    let public = &packages.consumer;
    let before = public.revision();
    let observed = "(call sequence-carriers::cell-read (local view))";
    let cases = [
        ("view-consume", READ_SCOPE.replace(observed, "(call sequence-carriers::cell-finish (local view))")),
        ("untaken-view-consume", READ_SCOPE.replace(observed, "(if (bool false) (call sequence-carriers::cell-finish (local view)) (call sequence-carriers::cell-read (local view)))")),
        ("protected-source", READ_SCOPE.replace(observed, "(sequence (sequence-pop (type (owned-sequence OwnedI64Cell)) (local values)) (i64 0))")),
        ("wrong-index", READ_SCOPE.replace("(index (i64 0))", "(index (bool false))")),
        ("wrong-child", READ_SCOPE.replace("(binding view (type OwnedI64Cell))", "(binding view (type ByteBuffer))")),
        ("wrong-source", READ_SCOPE.replace("(body (borrow-owned-item (type (owned-sequence OwnedI64Cell))", "(body (borrow-owned-item (type (owned-sequence ByteBuffer))")),
        ("view-escape", READ_SCOPE.replace("(returns I64)", "(returns OwnedI64Cell)").replace(observed, "(local view)")),
        ("view-storage", READ_SCOPE.replace(observed, "(let (binding destination (type (owned-sequence OwnedI64Cell)) (sequence-empty (type (owned-sequence OwnedI64Cell)))) (in (sequence (sequence-push (type (owned-sequence OwnedI64Cell)) (local view) (local destination)) (i64 0))))")),
        ("view-capture", READ_SCOPE.replace(observed, "(sequence (bind (function-value sequence-carriers::cell-read) (local view)) (i64 0))")),
        ("borrowed-task-parameter", READ_SCOPE.replace("(effect pure)", "(effect (task))").replace("(use consume)", "(use borrow)")),
        ("wrong-self-witness", APPLICATION.replacen("concrete@sequence-carriers::Scalar", "concrete@sequence-carriers::Octets", 1)),
        ("double-pop", READ_SCOPE.replace("(borrow-owned-item (type (owned-sequence OwnedI64Cell)) (local values)\n      (index (i64 0)) (binding view (type OwnedI64Cell))\n      (in (call sequence-carriers::cell-read (local view))))", "(sequence (sequence-pop (type (owned-sequence OwnedI64Cell)) (local values)) (sequence-pop (type (owned-sequence OwnedI64Cell)) (local values)) (i64 0))")),
        ("ordinary-element", r#"declarations.begin
(units (module create invalid-sequence
  (function create invalid (visibility public) (effect pure)
    (returns (owned-sequence I64)) (body (sequence-empty (type (owned-sequence I64)))))))
declarations.end
"#.into()),
        ("unconstrained-element", r#"declarations.begin
(units (module create invalid-sequence
  (function create invalid (visibility public) (effect pure) (type-parameter create T)
    (returns (owned-sequence T)) (body (sequence-empty (type (owned-sequence T)))))))
declarations.end
"#.into()),
        ("unused-invalid-substitution", r#"declarations.begin
(units (module create invalid-sequence
  (function create unused (visibility private) (effect pure)
    (type-parameter create T (constraint owned)) (returns I64) (body (i64 0)))
  (function create invalid (visibility public) (effect pure) (returns I64)
    (body (call unused (types (owned-sequence I64)))))))
declarations.end
"#.into()),
        ("ordinary-container", r#"declarations.begin
(units (module create invalid-sequence
  (function create invalid (visibility public) (effect pure)
    (returns (list (owned-sequence OwnedI64Cell)))
    (body (list (owned-sequence OwnedI64Cell))))))
declarations.end
"#.into()),
        ("data-codec", r#"declarations.begin
(units (use std builtin) (module create invalid-sequence
  (function create invalid (visibility public) (effect pure)
    (parameter create values (type (owned-sequence OwnedI64Cell)) (use consume))
    (returns Bytes) (body (call std::data-encode (types (owned-sequence OwnedI64Cell)) (local values))))))
declarations.end
"#.into()),
    ];
    for (name, body) in cases {
        rejected(public, &packages.source(&body), &before, name);
    }
    author(public, &packages.source(APPLICATION));
    let accepted = public.revision();
    let original = unchanged(public, "owned-sequences-app");
    let modules = public.cli(&["query", "find", "module", "owned-sequences-app"], true);
    let module = compact_field(compact_record(&modules, "owner"), "id");
    let functions = public.cli(
        &["query", "find", "declaration", "bounds", "--parent", module],
        true,
    );
    let bounds = compact_field(compact_record(&functions, "owner"), "id");
    let definition = public.cli(
        &[
            "inspect",
            "owner",
            "pure_function",
            bounds,
            "--detail",
            "definition",
            "--limit",
            "1000",
        ],
        true,
    );
    let scope = definition
        .iter()
        .find(|record| {
            record.operation == "definition.expression"
                && compact_field(record, "form") == "borrow_owned_item"
        })
        .unwrap();
    let request = public.input("unsupported-sequence-extraction.lkjc", &format!("request base={accepted}\nextract.function as=$helper function={bounds} expression={} name=indexed-helper\n", compact_field(scope, "id")));
    let records = public.plan(&request, false);
    assert!(
        records.iter().any(|record| record.operation == "diagnostic"
            && compact_field(record, "code") == "change_extract_owned_borrow"),
        "{records:?}"
    );
    assert_eq!(public.revision(), accepted);
    let edited = public.input(
        "reviewed-sequence-literal-edit.lkjc",
        &original.replacen("(i64 41)", "(i64 42)", 1),
    );
    public.apply(&edited, &public.plan(&edited, true), true);
    public.cli(&["check"], true);
    assert_eq!(
        unchanged(public, "owned-sequences-app"),
        original
            .replacen(&accepted, &public.revision(), 1)
            .replacen("(i64 41)", "(i64 42)", 1),
        "literal editing preserves every sequence, source, index and view identity"
    );
    for index in [i64::MIN, -1, 1, i64::MAX, 0] {
        let arguments = public.input(&format!("bounds-{index}.json"), &json!([index]).to_string());
        let result = public
            .root
            .path()
            .join(format!("bounds-result-{index}.json"));
        let records = public.cli(
            &[
                "run",
                "owned-sequence-bounds",
                "--arguments-file",
                path(&arguments),
                "--result-file",
                path(&result),
            ],
            index == 0,
        );
        if index == 0 {
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(result).unwrap()).unwrap(),
                json!(2)
            );
            clean_execution(&records, 0, false);
        } else {
            assert!(!result.exists());
            assert!(
                records
                    .iter()
                    .any(|record| record.operation == "diagnostic"),
                "{records:?}"
            );
        }
    }
    let arguments = public.input("edited-arguments.json", "[[2,3,7]]");
    let result = public.root.path().join("edited-result.json");
    public.cli(
        &[
            "run",
            "owned-sequences",
            "--arguments-file",
            path(&arguments),
            "--result-file",
            path(&result),
        ],
        true,
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(result).unwrap()).unwrap(),
        expected(&[2, 3, 7], 42)
    );
    packages.retain("rejections-and-edit");
}

#[test]
fn native_owned_sequence_task_effectful_index_unrelated_owner_and_parallel_transfer() {
    let packages = Packages::stage();
    let public = &packages.consumer;
    let before = public.revision();
    rejected(
        public,
        &packages.source(&TASKS.replacen(
            "(effect (task (requirement command::clock)))",
            "(effect (task))",
            1,
        )),
        &before,
        "missing-effect",
    );
    author(public, &packages.source(TASKS));
    unchanged(public, "owned-sequence-tasks");
    let artifact = public.root.path().join("sequence-tasks.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = json!({
        "artifact":"sequence-tasks.lkja","target":"owned-sequence-tasks","listen":null,
        "http":null,"session":null,"worker":null,
        "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":1048576,"maximum_live_streams":1024},
        "grants":[{"requirement":"clock","sharing_domain":"owned-sequence-tasks","authority_revision":"87".repeat(32),"adapter":{"kind":"wall_clock"}}],
        "secrets":[],"configuration":{},
    });
    let deployment = public.input("sequence-tasks.deployment.json", &descriptor.to_string());
    packages.detach();
    let mut missing = descriptor;
    missing["grants"] = json!([]);
    let denied = public.input("sequence-tasks-denied.json", &missing.to_string());
    let arguments = public.input("sequence-tasks-denied-arguments.json", "[[2,3,7]]");
    let result = public.root.path().join("sequence-tasks-denied-result.json");
    let records = public.cli(
        &[
            "run",
            "--deployment",
            path(&denied),
            "--arguments-file",
            path(&arguments),
            "--result-file",
            path(&result),
        ],
        false,
    );
    assert!(records.iter().any(|record| record.operation == "diagnostic"
        && compact_field(record, "code") == "deployment_grant_missing"));
    assert!(!result.exists());
    for (case, inputs) in [vec![2, 3, 7], vec![31], vec![0, 255, 128]]
        .iter()
        .enumerate()
    {
        let arguments = public.input(
            &format!("sequence-task-arguments-{case}.json"),
            &json!([inputs]).to_string(),
        );
        let result = public
            .root
            .path()
            .join(format!("sequence-task-result-{case}.json"));
        let records = public.cli(
            &[
                "run",
                "--deployment",
                path(&deployment),
                "--arguments-file",
                path(&arguments),
                "--result-file",
                path(&result),
            ],
            true,
        );
        let drained: Vec<i64> = std::iter::once(11)
            .chain(inputs.iter().rev().copied())
            .collect();
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(result).unwrap()).unwrap(),
            json!({"read":inputs[0],"unrelated":77,"scalar":drained,"bytes":drained})
        );
        clean_execution(&records, 1, true);
    }
    packages.retain("task-transfer");
}
