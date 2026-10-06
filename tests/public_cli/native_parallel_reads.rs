//! Public recursive read tasks, exact borrowed task methods and joined owner reuse.
use super::native_byte_buffer::{Export, author, dependency, export, stage};
use super::native_owned_worklists::{
    declaration, deployment, rejected, run_expected, standard, unchanged,
};
use super::*;
use serde_json::json;

const WORKLISTS: &str = include_str!("../../examples/owned-worklists/library.lkjc");
const READERS: &str = include_str!("../../examples/owned-read-results/library.lkjc");
const STORAGE: &str = include_str!("../../examples/generic-owned-implementations/storage.lkjc");
const ELEMENTS: &str = include_str!("../../examples/generic-owned-implementations/elements.lkjc");
const LIBRARY: &str = include_str!("../../examples/scoped-parallel-reads/library.lkjc");
const APPLICATION: &str = include_str!("../../examples/scoped-parallel-reads/application.lkjc");

fn evidence_native(template: &str) -> Native {
    let mut public = Native::template(template);
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        public.root.disable_cleanup(true);
        println!(
            "scoped parallel read evidence root: {}",
            public.root.path().display()
        );
    }
    public
}

fn author_retained(public: &Native, source: &str, name: &str) {
    author(public, source);
    std::fs::copy(
        public.root.path().join("native.lkjc"),
        public.root.path().join(format!("{name}.lkjc")),
    )
    .unwrap();
}

fn unchanged_as(public: &Native, module: &str, label: &str) -> String {
    let draft = public
        .root
        .path()
        .join(format!("{module}-{label}-draft.lkjc"));
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

struct Packages {
    library: Native,
    elements: Native,
    consumer: Native,
    generic: Export,
    concrete: Export,
    standards: [PathBuf; 2],
}

impl Packages {
    fn stage() -> Self {
        let library = evidence_native("minimal");
        let supplier = standard(&library);
        author_retained(
            &library,
            &format!("{}{WORKLISTS}", dependency(&supplier)),
            "worklists",
        );
        author_retained(&library, READERS, "readers");
        author_retained(&library, STORAGE, "storage");
        let before = library.revision();
        for (name, from, to) in [
            (
                "move-does-not-prove-sharing",
                "W (constraint owned shareable)",
                "W (constraint owned transferable)",
            ),
            (
                "missing-sharing",
                "W (constraint owned shareable)",
                "W (constraint owned)",
            ),
            (
                "borrowed-method-mode-mismatch",
                "(parameters (Self borrow))",
                "(parameters (Self consume))",
            ),
            (
                "borrowed-task-result",
                "(returns I64) (effect (task))",
                "(returns Self (borrow-from 0)) (effect (task))",
            ),
        ] {
            assert!(LIBRARY.contains(from), "negative library fixture: {name}");
            let mut invalid = LIBRARY.to_owned();
            if from == "W (constraint owned shareable)" {
                let span = form_span(&invalid, "(function create range-sum");
                let replacement = invalid[span.clone()].replacen(from, to, 1);
                invalid.replace_range(span, &replacement);
            } else {
                invalid = invalid.replacen(from, to, 1);
            }
            rejected(&library, &invalid, &before, name);
        }
        author_retained(&library, LIBRARY, "recursive-read-library");
        let draft = unchanged(&library, "scoped-parallel-reads");
        for absent in ["OwnedI64Cell", "ByteBuffer", "product-create"] {
            assert!(
                !draft.contains(absent),
                "library exists before independent items: {absent}"
            );
        }
        for present in [
            "(constraint owned shareable)",
            "(use borrow)",
            "(parallel ",
            "(owned-implementation ",
        ] {
            assert!(
                draft.contains(present),
                "recursive library draft: {present}"
            );
        }
        let sum = declaration(&library, "scoped-parallel-reads", "sum");
        let inspection = library.cli(
            &[
                "inspect",
                "owner",
                "task_function",
                &sum,
                "--detail",
                "definition",
                "--limit",
                "1000",
            ],
            true,
        );
        assert!(
            inspection
                .iter()
                .any(|record| record.operation == "definition.parameter"
                    && compact_field(record, "use") == "borrow")
        );
        // Only the completed generic library is exported before this project exists.
        let generic = export(&library);
        let elements = evidence_native("minimal");
        stage(&elements, &generic);
        let element_supplier = standard(&elements);
        author_retained(
            &elements,
            &format!(
                "{}{}declarations.begin\n(units (use owned-worklists {} {}))\ndeclarations.end\n{ELEMENTS}",
                dependency(&generic),
                dependency(&element_supplier),
                generic.package,
                generic.revision,
            ),
            "independent-elements",
        );
        unchanged(&elements, "generic-elements");
        let concrete = export(&elements);
        let consumer = evidence_native("command");
        stage(&consumer, &generic);
        stage(&consumer, &concrete);
        Self {
            library,
            elements,
            consumer,
            generic,
            concrete,
            standards: [supplier.path, element_supplier.path],
        }
    }

    fn source(&self, body: &str) -> String {
        format!(
            "{}{}declarations.begin\n(units (use owned-worklists {} {}) (use owned-read-results {} {}) (use generic-storage {} {}) (use scoped-parallel-reads {} {}) (use generic-elements {} {}))\ndeclarations.end\n{body}",
            dependency(&self.generic),
            dependency(&self.concrete),
            self.generic.package,
            self.generic.revision,
            self.generic.package,
            self.generic.revision,
            self.generic.package,
            self.generic.revision,
            self.generic.package,
            self.generic.revision,
            self.concrete.package,
            self.concrete.revision,
        )
    }

    fn detach(&self) {
        for project in [
            &self.library.project,
            &self.elements.project,
            &self.consumer.project,
        ] {
            std::fs::remove_dir_all(project).unwrap();
        }
        for transport in [
            &self.generic.path,
            &self.concrete.path,
            &self.standards[0],
            &self.standards[1],
        ] {
            std::fs::remove_file(transport).unwrap();
        }
    }

    fn retain(self) {
        if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
            for public in [self.library, self.elements, self.consumer] {
                println!(
                    "retained scoped parallel read evidence: {}",
                    public.root.keep().display()
                );
            }
        }
    }
}

fn scenario_expected(inputs: &[i64], offset: i64, reuse: i64) -> Value {
    let values: Vec<_> = inputs.iter().map(|n| n + offset).collect();
    let sum: i64 = values.iter().sum();
    let keys: i64 = values.iter().map(|n| n % 4).sum();
    let drained: Vec<_> = values.iter().rev().copied().collect();
    json!({"length":inputs.len(), "sum-left":sum, "sum-right":keys, "method-sum":sum,
        "drained":drained, "empty-length":0, "reused":[reuse + offset]})
}

fn all_expected(inputs: &[i64], reuse: i64) -> Value {
    let ordinary = scenario_expected(inputs, 0, reuse);
    let product = scenario_expected(inputs, 12, reuse);
    json!({"cell-flat":ordinary, "cell-chunked":ordinary,
        "buffer-flat":ordinary, "buffer-chunked":ordinary,
        "product-flat":product, "product-chunked":product})
}

fn run_matched(public: &Native, descriptor: &Path, label: &str, inputs: &[i64], serial: bool) {
    let input = public.input(
        &format!("matched-{label}-arguments.json"),
        &json!([inputs]).to_string(),
    );
    let result = public
        .root
        .path()
        .join(format!("matched-{label}-result.json"));
    let records = public.cli(
        &[
            "run",
            "--deployment",
            path(descriptor),
            "--arguments-file",
            path(&input),
            "--result-file",
            path(&result),
        ],
        true,
    );
    assert_eq!(
        std::fs::read(&result).unwrap(),
        serde_json::to_vec(&scenario_expected(inputs, 0, 17)).unwrap()
    );
    let execution = compact_record(&records, "execution");
    let observation: Value =
        serde_json::from_str(compact_field(execution, "production-observation")).unwrap();
    let scopes = observation["parallel_scopes"].as_u64().unwrap();
    if serial {
        assert_eq!(scopes, 0);
    } else if inputs.len() <= 32 {
        assert_eq!(scopes, 1);
    } else {
        assert!(scopes >= 4);
    }
    assert_eq!(observation["live_handles_after"], json!(0));
    let cleanup: Value = serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
    assert_eq!(cleanup["remaining_tasks"], json!(0));
    assert_eq!(cleanup["cleanup_failures"], json!([]));
    let measurement = json!({"serial":serial, "inputs":inputs.len(),
        "preparation-nanoseconds":compact_field(execution,"preparation-nanoseconds"),
        "invocation-nanoseconds":compact_field(execution,"invocation-nanoseconds"),
        "result-encoding-nanoseconds":compact_field(execution,"result-encoding-nanoseconds"),
        "observation":observation});
    public.input(
        &format!("matched-{label}-measurement.json"),
        &measurement.to_string(),
    );
}

#[test]
fn native_parallel_reads_three_packages_recursive_borrowed_task_methods_shareable_constraints_reuse_and_detach()
 {
    let packages = Packages::stage();
    let public = &packages.consumer;
    author_retained(public, &packages.source(APPLICATION), "consumer");
    let scenario = declaration(public, "scoped-read-app", "scenario");
    let draft = unchanged_as(public, "scoped-read-app", "accepted");
    assert!(draft.contains("(constraint owned transferable shareable)"));
    let artifact = public.root.path().join("scoped-parallel-reads.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let combined = public.input(
        "scoped-parallel-reads.deployment.json",
        include_str!("../../examples/scoped-parallel-reads/scoped-parallel-reads.deployment.json"),
    );
    let individual: Vec<_> = [
        (
            "scoped-read-cell-flat",
            0,
            include_str!(
                "../../examples/scoped-parallel-reads/scoped-read-cell-flat.deployment.json"
            ),
        ),
        (
            "scoped-read-cell-chunked",
            0,
            include_str!(
                "../../examples/scoped-parallel-reads/scoped-read-cell-chunked.deployment.json"
            ),
        ),
        (
            "scoped-read-buffer-flat",
            0,
            include_str!(
                "../../examples/scoped-parallel-reads/scoped-read-buffer-flat.deployment.json"
            ),
        ),
        (
            "scoped-read-buffer-chunked",
            0,
            include_str!(
                "../../examples/scoped-parallel-reads/scoped-read-buffer-chunked.deployment.json"
            ),
        ),
        (
            "scoped-read-product-flat",
            12,
            include_str!(
                "../../examples/scoped-parallel-reads/scoped-read-product-flat.deployment.json"
            ),
        ),
        (
            "scoped-read-product-chunked",
            12,
            include_str!(
                "../../examples/scoped-parallel-reads/scoped-read-product-chunked.deployment.json"
            ),
        ),
    ]
    .into_iter()
    .map(|(target, offset, descriptor)| {
        (
            target,
            offset,
            public.input(&format!("{target}.deployment.json"), descriptor),
        )
    })
    .collect();
    let serial = public.input(
        "scoped-read-cell-flat-serial.deployment.json",
        include_str!(
            "../../examples/scoped-parallel-reads/scoped-read-cell-flat-serial.deployment.json"
        ),
    );
    let mut workloads = vec![vec![], vec![2], vec![2, 7, 11, 3], vec![0, 255, 128]];
    for length in [31, 32, 33] {
        workloads.push((0..length).map(|n| n % 17).collect());
    }
    assert_eq!(
        all_expected(&[2, 7, 11, 3], 17)["cell-flat"]["sum-left"],
        json!(23)
    );
    assert_eq!(
        all_expected(&[2, 7, 11, 3], 17)["cell-flat"]["sum-right"],
        json!(11)
    );
    for (case, inputs) in workloads.iter().enumerate() {
        // A Task uses the deployment route even while authoring sources exist.
        run_expected(
            public,
            "scoped-parallel-reads",
            &combined,
            true,
            &format!("attached-{case}"),
            json!([inputs]),
            &all_expected(inputs, 17),
        );
    }
    for length in [31, 32, 33] {
        let inputs: Vec<_> = (0..length).map(|n| n % 17).collect();
        run_matched(
            public,
            &serial,
            &format!("attached-serial-{length}"),
            &inputs,
            true,
        );
        run_matched(
            public,
            &individual[0].2,
            &format!("attached-parallel-{length}"),
            &inputs,
            false,
        );
    }
    assert!(draft.contains("(i64 17)"));
    let edit = public.input(
        "reuse-edit.lkjc",
        &draft.replacen("(i64 17)", "(i64 19)", 1),
    );
    public.apply(&edit, &public.plan(&edit, true), true);
    assert_eq!(declaration(public, "scoped-read-app", "scenario"), scenario);
    public.cli(
        &[
            "build",
            "--output",
            path(&public.root.path().join("edited.lkja")),
        ],
        true,
    );
    let edited_descriptor = public.input(
        "edited-scoped-parallel-reads.deployment.json",
        &include_str!("../../examples/scoped-parallel-reads/scoped-parallel-reads.deployment.json")
            .replace("scoped-parallel-reads.lkja", "edited.lkja"),
    );
    run_expected(
        public,
        "scoped-parallel-reads",
        &edited_descriptor,
        true,
        "edited",
        json!([[2, 7, 11, 3]]),
        &all_expected(&[2, 7, 11, 3], 19),
    );
    let edited = unchanged_as(public, "scoped-read-app", "edited");
    let restore = public.input(
        "reuse-restore.lkjc",
        &edited.replacen("(i64 19)", "(i64 17)", 1),
    );
    public.apply(&restore, &public.plan(&restore, true), true);
    public.cli(&["check"], true);
    assert_eq!(declaration(public, "scoped-read-app", "scenario"), scenario);
    unchanged_as(public, "scoped-read-app", "restored");
    let large: Vec<_> = (0..513).map(|n| n % 17).collect();
    for removed in [false, true] {
        if removed {
            packages.detach();
        }
        for (target, offset, descriptor) in &individual {
            if *target == "scoped-read-cell-flat" {
                for sequential in if removed {
                    [false, true]
                } else {
                    [true, false]
                } {
                    run_matched(
                        public,
                        if sequential { &serial } else { descriptor },
                        &format!("removed-{removed}-513-serial-{sequential}"),
                        &large,
                        sequential,
                    );
                }
                continue;
            }
            run_expected(
                public,
                target,
                descriptor,
                true,
                &format!("removed-{removed}-513"),
                json!([large]),
                &scenario_expected(&large, *offset, 17),
            );
        }
        if removed {
            for (case, inputs) in workloads.iter().enumerate() {
                run_expected(
                    public,
                    "scoped-parallel-reads",
                    &combined,
                    true,
                    &format!("detached-{case}"),
                    json!([inputs]),
                    &all_expected(inputs, 17),
                );
            }
        }
    }
    packages.retain();
}

const READ_LOANS: &str = r#"declarations.begin
(units (module create scoped-read-validation
  (external create make (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create observe (visibility private) (implementation core.cell.read)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64))
  (external create extract (visibility private) (implementation core.cell.extract)
    (parameter create value (type OwnedI64Cell) (use consume)) (returns I64))
  (function create read (visibility public) (effect (task))
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64)
    (body (call observe (local value))))
  (function create take (visibility public) (effect (task))
    (parameter create value (type OwnedI64Cell) (use consume)) (returns I64)
    (body (call extract (local value))))
  (function create ordinary (visibility public) (effect (task))
    (parameter create n (type I64)) (returns I64) (body (local n)))
  (function create pair (visibility public) (effect (task))
    (returns (record (left I64) (right I64)))
    (body (let (binding value (type OwnedI64Cell) (call make (i64 23)))
      (in (parallel (call read (local value)) (call read (local value)))))))
  (function create main (visibility public) (effect (task)) (returns I64)
    (body (let (binding value (type OwnedI64Cell) (call make (i64 23)))
      (binding observed (type I64) (call read (local value)))
      (in (call extract (local value))))))
  (component create command (visibility private)
    (port create main (type (task-function () I64 (row))) (function main))
    (port create pair (type (task-function () (record (left I64) (right I64)) (row))) (function pair))))
  (target create scoped-read-synchronous (component scoped-read-validation::command)
    (runner command) (port scoped-read-validation::command::main))
  (target create scoped-read-pair (component scoped-read-validation::command)
    (runner command) (port scoped-read-validation::command::pair)))
declarations.end
"#;

#[test]
fn native_parallel_reads_reject_consuming_aliases_borrowed_task_results_and_read_mutation_without_publication()
 {
    let public = evidence_native("command");
    let before = public.revision();
    let pair = "(parallel (call read (local value)) (call read (local value)))";
    for (name, from, to) in [
        (
            "read-then-consume",
            pair,
            "(parallel (call read (local value)) (call take (local value)))",
        ),
        (
            "consume-then-read",
            pair,
            "(parallel (call take (local value)) (call read (local value)))",
        ),
        (
            "pending-right-argument-consumes-left-read",
            pair,
            "(parallel (call read (local value)) (call ordinary (call extract (local value))))",
        ),
        (
            "borrowed-task-cannot-consume",
            "(body (call observe (local value)))",
            "(body (call extract (local value)))",
        ),
        (
            "borrowed-task-cannot-return-owner",
            "(returns I64)\n    (body (call observe (local value)))",
            "(returns OwnedI64Cell)\n    (body (local value))",
        ),
        (
            "task-read-result-still-unsupported",
            "(returns I64)\n    (body (call observe (local value)))",
            "(returns OwnedI64Cell (borrow-from value))\n    (body (local value))",
        ),
    ] {
        assert!(READ_LOANS.contains(from), "negative read fixture: {name}");
        rejected(&public, &READ_LOANS.replacen(from, to, 1), &before, name);
    }
    author_retained(&public, READ_LOANS, "synchronous-and-shared-reads");
    unchanged(&public, "scoped-read-validation");
    let artifact = public.root.path().join("read-validation.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let synchronous = deployment(&public, "read-validation.lkja", "scoped-read-synchronous");
    let pair = deployment(&public, "read-validation.lkja", "scoped-read-pair");
    for removed in [false, true] {
        if removed {
            std::fs::remove_dir_all(&public.project).unwrap();
        }
        run_expected(
            &public,
            "scoped-read-synchronous",
            &synchronous,
            true,
            &format!("removed-{removed}"),
            json!([]),
            &json!(23),
        );
        run_expected(
            &public,
            "scoped-read-pair",
            &pair,
            true,
            &format!("removed-{removed}"),
            json!([]),
            &json!({"left":23,"right":23}),
        );
    }
}
