//! Scoped native reads, exact generic witnesses, unchanged owners and detached artifacts.
use super::native_byte_buffer::{Export, author, dependency, export, stage};
use super::*;
use serde_json::json;

const LIBRARY: &str = include_str!("../../examples/owned-borrows/library.lkjc");
const CARRIERS: &str = include_str!("../../examples/owned-borrows/carriers.lkjc");
const APPLICATION: &str = include_str!("../../examples/owned-borrows/application.lkjc");
const TASK: &str = include_str!("../fixtures/owned-borrows-task.lkjc");

struct Packages {
    library: Native,
    carriers: Native,
    consumer: Native,
    generic: Export,
    concrete: Export,
}

impl Packages {
    fn stage() -> Self {
        let library = Native::new();
        author(&library, LIBRARY);
        let original = unchanged(&library, "owned-borrows");
        for absent in ["OwnedI64Cell", "ByteBuffer", "owned-implementation"] {
            assert!(!original.contains(absent), "generic library: {absent}");
        }
        for form in ["(borrow-owned-field", "(match-borrowed-owned"] {
            assert!(original.contains(form), "generic draft: {form}");
        }
        assert_eq!(
            compact_field(
                compact_record(&library.cli(&["status"], true), "summary"),
                "dependencies",
            ),
            "0",
        );
        // Admission precedes every concrete carrier and implementation declaration.
        let generic = export(&library);
        let carriers = Native::new();
        stage(&carriers, &generic);
        author(
            &carriers,
            &format!(
                "{}declarations.begin\n(units (use owned-borrows {} {}))\ndeclarations.end\n{CARRIERS}",
                dependency(&generic),
                generic.package,
                generic.revision,
            ),
        );
        unchanged(&carriers, "owned-borrow-carriers");
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
        }
    }

    fn source(&self, body: &str) -> String {
        format!(
            "{}{}declarations.begin\n(units (use owned-borrows {} {}) (use owned-borrow-carriers {} {}))\ndeclarations.end\n{body}",
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
        for package in [&self.generic.path, &self.concrete.path] {
            std::fs::remove_file(package).unwrap();
        }
    }

    fn retain(self, label: &str) {
        if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
            for public in [self.library, self.carriers, self.consumer] {
                println!(
                    "retained owned-borrow {label} public evidence: {}",
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

fn expected(n: i64, accepted: bool, marker: i64) -> Value {
    json!({
        "scalar":{"first":n,"second":n,"tag":7},
        "alternate":{"first":99,"second":99,"tag":7},"consumed":n,
        "bytes":{"first":1,"second":1,"tag":8},"frozen":{"$bytes":"SQ=="},
        "nested":{"child":n,"parent":{"first":n,"second":n,"tag":9},"marker":17},
        "nested-consumed":n,"choice":{"first":n,"second":if accepted {n} else {99},"consumed":n},
        "siblings":{"left":n,"right":n},"sibling-consumed":{"left":n,"right":n},"marker":marker,
    })
}

fn clean_execution(records: &[CompactRecord], calls: u64) {
    let execution = compact_record(records, "execution");
    let observation: Value =
        serde_json::from_str(compact_field(execution, "production-observation")).unwrap();
    assert_eq!(observation["capability_calls"], json!(calls));
    assert_eq!(observation["live_handles_after"], json!(0));
    let cleanup: Value = serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
    assert_eq!(cleanup["remaining_tasks"], json!(0));
    assert_eq!(cleanup["cleanup_failures"], json!([]));
}

#[test]
fn native_owned_borrows_three_packages_exact_reads_then_consume_and_detach() {
    let packages = Packages::stage();
    author(&packages.consumer, &packages.source(APPLICATION));
    let public = &packages.consumer;
    let advertised = public.cli(&["capabilities", "--section", "change"], true);
    for syntax in ["borrow-owned-field", "match-borrowed-owned"] {
        assert!(
            advertised.iter().any(|record| record
                .fields
                .iter()
                .any(|field| field.value.contains(syntax))),
            "discovery: {syntax}"
        );
    }
    let original = unchanged(public, "owned-borrows-app");
    assert!(original.contains("(borrow-owned-field"));
    assert!(original.contains("(match-borrowed-owned"));
    let artifact = public.root.path().join("owned-borrows.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let descriptor: Value = serde_json::from_str(include_str!(
        "../../examples/owned-borrows/owned-borrows.deployment.json"
    ))
    .unwrap();
    let deployment = public.input("owned-borrows.deployment.json", &descriptor.to_string());
    for detached in [false, true] {
        if detached {
            packages.detach();
        }
        for n in [i64::MIN, -257, i64::MAX] {
            for accepted in [false, true] {
                let arguments = public.input(
                    &format!("arguments-{detached}-{n}-{accepted}.json"),
                    &json!([n, accepted]).to_string(),
                );
                let result = public
                    .root
                    .path()
                    .join(format!("result-{detached}-{n}-{accepted}.json"));
                let mut args = if detached {
                    vec!["run", "--deployment", path(&deployment)]
                } else {
                    vec!["run", "owned-borrows"]
                };
                args.extend([
                    "--arguments-file",
                    path(&arguments),
                    "--result-file",
                    path(&result),
                ]);
                let records = public.cli(&args, true);
                // The expected JSON is independent of both the authored generic helpers and compiler.
                assert_eq!(
                    serde_json::from_slice::<Value>(&std::fs::read(result).unwrap()).unwrap(),
                    expected(n, accepted, 41)
                );
                if detached {
                    clean_execution(&records, 0);
                }
            }
        }
    }
    packages.retain("detached");
}

#[test]
fn native_owned_borrows_reject_escaping_or_consuming_views_and_preserve_draft_identities() {
    let packages = Packages::stage();
    let public = &packages.consumer;
    let before = public.revision();
    let first_read = "(call owned-borrow-carriers::cell-read (local view))";
    let choice_span = form_span(APPLICATION, "(match-borrowed-owned");
    let choice = &APPLICATION[choice_span];
    let rejected_arm = &choice[form_span(choice, "(case rejected (binding")];
    let choice_edit =
        |from: &str, to: &str| APPLICATION.replacen(choice, &choice.replacen(from, to, 1), 1);
    let mut cases = vec![
        ("view-consume", APPLICATION.replacen(first_read, "(call owned-borrow-carriers::cell-finish (local view))", 1)),
        ("root-consume", APPLICATION.replacen(first_read,
            "(call owned-borrow-carriers::cell-finish (call owned-borrows::take-packet (types OwnedI64Cell) (local packet)))", 1)),
        ("wrong-field-type", APPLICATION.replacen("(field payload (binding view (type OwnedI64Cell)))", "(field payload (binding view (type ByteBuffer)))", 1)),
        ("missing-field", APPLICATION.replacen("(field payload (binding view (type OwnedI64Cell)))", "(field absent (binding view (type OwnedI64Cell)))", 1)),
        ("ordinary-field", APPLICATION.replacen("(field payload (binding view (type OwnedI64Cell)))", "(field tag (binding view (type I64)))", 1)),
        ("choice-wrong-type", choice_edit("(binding value (type I64))", "(binding value (type Bool))")),
        ("choice-incomplete", APPLICATION.replacen(choice, &choice.replacen(rejected_arm, "", 1), 1)),
        ("choice-duplicate", choice_edit("(case rejected (binding", "(case accepted (binding")),
        ("choice-view-consume", APPLICATION.replacen(choice, &choice.replacen(first_read, "(call owned-borrow-carriers::cell-finish (local view))", 1), 1)),
        ("untaken-view-consume", APPLICATION.replacen(first_read,
            "(if (bool false) (call owned-borrow-carriers::cell-finish (local view)) (call owned-borrow-carriers::cell-read (local view)))", 1)),
        ("wrong-self-witness", APPLICATION.replacen("concrete@owned-borrow-carriers::Scalar", "concrete@owned-borrow-carriers::Octets", 1)),
    ];
    cases.extend([
        (
            "view-escape",
            include_str!("../fixtures/owned-borrows-escape.lkjc").to_owned(),
        ),
        (
            "view-storage",
            include_str!("../fixtures/owned-borrows-store.lkjc").to_owned(),
        ),
        (
            "view-unrestricted-argument",
            include_str!("../fixtures/owned-borrows-unrestricted.lkjc").to_owned(),
        ),
    ]);
    for (name, invalid) in cases {
        let input = public.input(
            &format!("invalid-{name}.lkjc"),
            &format!("request base={before}\n{}", packages.source(&invalid)),
        );
        let failed = public.plan(&input, false);
        assert!(
            failed.iter().any(|record| record.operation == "diagnostic"
                && compact_field(record, "code").starts_with("kernel_")),
            "{name} must fail semantic admission: {failed:?}"
        );
        assert_eq!(public.revision(), before, "{name}");
    }
    let borrowed_task = include_str!("../fixtures/owned-borrows-task-parameter.lkjc");
    author(
        public,
        &packages.source(&format!("{borrowed_task}\n{APPLICATION}")),
    );
    unchanged(public, "borrowed-task-parameter");
    let accepted = public.revision();
    let original = unchanged(public, "owned-borrows-app");
    let modules = public.cli(&["query", "find", "module", "owned-borrows-app"], true);
    let module = compact_field(compact_record(&modules, "owner"), "id");
    let owners = public.cli(
        &["query", "find", "declaration", "main", "--parent", module],
        true,
    );
    let main = compact_field(compact_record(&owners, "owner"), "id");
    let definition = public.cli(
        &[
            "inspect",
            "owner",
            "pure_function",
            main,
            "--detail",
            "definition",
            "--limit",
            "1000",
        ],
        true,
    );
    for syntax in ["borrow_owned_field", "match_borrowed_owned"] {
        let scope = definition
            .iter()
            .find(|record| {
                record.operation == "definition.expression"
                    && compact_field(record, "form") == syntax
            })
            .unwrap();
        let input = public.input(&format!("unsupported-extract-{syntax}.lkjc"),
            &format!("request base={accepted}\nextract.function as=$helper function={main} expression={} name=borrowed-helper\n", compact_field(scope, "id")));
        let failed = public.plan(&input, false);
        assert!(
            failed.iter().any(|record| record.operation == "diagnostic"
                && compact_field(record, "code") == "change_extract_owned_borrow"),
            "{failed:?}"
        );
        assert_eq!(public.revision(), accepted);
    }
    let edited = public.input(
        "reviewed-borrow-literal-edit.lkjc",
        &original.replacen("(i64 41)", "(i64 42)", 1),
    );
    let plan = public.plan(&edited, true);
    public.apply(&edited, &plan, true);
    public.cli(&["check"], true);
    assert_eq!(
        unchanged(public, "owned-borrows-app"),
        original
            .replacen(&accepted, &public.revision(), 1)
            .replacen("(i64 41)", "(i64 42)", 1),
        "literal edit retains declaration, witness, parent-source, arm and view identities"
    );
    let arguments = public.input("edited-arguments.json", "[42,false]");
    let result = public.root.path().join("edited-result.json");
    public.cli(
        &[
            "run",
            "owned-borrows",
            "--arguments-file",
            path(&arguments),
            "--result-file",
            path(&result),
        ],
        true,
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(result).unwrap()).unwrap(),
        expected(42, false, 42)
    );
    packages.retain("rejections-and-edit");
}

#[test]
fn native_owned_borrows_task_body_keeps_effect_authority_and_returns_unrelated_owner() {
    let packages = Packages::stage();
    let public = &packages.consumer;
    let before = public.revision();
    let missing_effect = TASK.replacen(
        "(effect (task (requirement command::clock)))",
        "(effect (task))",
        1,
    );
    let input = public.input(
        "borrow-missing-effect.lkjc",
        &format!(
            "request base={before}\n{}",
            packages.source(&missing_effect)
        ),
    );
    let rejected = public.plan(&input, false);
    assert!(
        rejected
            .iter()
            .any(|record| record.operation == "diagnostic"
                && compact_field(record, "code").starts_with("kernel_")),
        "{rejected:?}"
    );
    assert_eq!(public.revision(), before);
    author(public, &packages.source(TASK));
    unchanged(public, "owned-borrow-task");
    let artifact = public.root.path().join("task-borrows.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = json!({
        "artifact":"task-borrows.lkja", "target":"owned-borrow-task", "listen":null,
        "http":null,"session":null,"worker":null,
        "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":1048576,"maximum_live_streams":1024},
        "grants":[{"requirement":"clock","sharing_domain":"owned-borrow-task","authority_revision":"86".repeat(32),"adapter":{"kind":"wall_clock"}}],
        "secrets":[],"configuration":{},
    });
    let deployment = public.input("task-borrows.deployment.json", &descriptor.to_string());
    packages.detach();
    let mut missing = descriptor;
    missing["grants"] = json!([]);
    let denied = public.input("task-borrows-denied.json", &missing.to_string());
    let result = public.root.path().join("task-borrows-denied-result.json");
    let arguments = public.input("task-borrows-arguments.json", "[-257]");
    let rejected = public.cli(
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
    assert!(
        rejected
            .iter()
            .any(|record| record.operation == "diagnostic"
                && compact_field(record, "code") == "deployment_grant_missing")
    );
    assert!(!result.exists());
    for n in [i64::MIN, -257, i64::MAX] {
        let arguments = public.input(&format!("task-borrows-{n}.json"), &json!([n]).to_string());
        let result = public
            .root
            .path()
            .join(format!("task-borrows-result-{n}.json"));
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
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(result).unwrap()).unwrap(),
            json!({"read":n,"unrelated":77,"original":n})
        );
        clean_execution(&records, 1);
    }
    packages.retain("task");
}
