//! Source-tied borrowed results through fresh native packages and independent outputs.
use super::native_byte_buffer::{Export, author, dependency, export, stage};
use super::native_owned_worklists::{
    declaration, deployment, rejected, run_expected, standard, unchanged,
};
use super::*;
use serde_json::json;

const WORKLISTS: &str = include_str!("../../examples/owned-worklists/library.lkjc");
const WORKLIST_CARRIERS: &str = include_str!("../../examples/owned-worklists/carriers.lkjc");
const LIBRARY: &str = include_str!("../../examples/owned-read-results/library.lkjc");
const CARRIERS: &str = include_str!("../../examples/owned-read-results/carriers.lkjc");
const APPLICATION: &str = include_str!("../../examples/owned-read-results/application.lkjc");

fn evidence_native(template: &str) -> Native {
    let mut public = Native::template(template);
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        public.root.disable_cleanup(true);
        println!(
            "read-result evidence root (including failed inputs): {}",
            public.root.path().display()
        );
    }
    public
}

struct Packages {
    library: Native,
    carriers: Native,
    consumer: Native,
    generic: Export,
    concrete: Export,
    standards: [PathBuf; 2],
}

impl Packages {
    fn stage() -> Self {
        let library = evidence_native("minimal");
        let library_standard = standard(&library);
        author_retained(
            &library,
            &format!("{}{WORKLISTS}", dependency(&library_standard)),
            "worklist-library",
        );
        author_retained(&library, LIBRARY, "read-library");
        let draft = unchanged(&library, "owned-read-results");
        assert!(draft.contains("(borrow-from "));
        assert!(draft.contains("(borrow-call "));
        for absent in ["OwnedI64Cell", "ByteBuffer", "owned-implementation"] {
            assert!(!draft.contains(absent), "generic library: {absent}");
        }
        // Generic admission and export precede every concrete reader witness.
        let generic = export(&library);
        let carriers = evidence_native("minimal");
        stage(&carriers, &generic);
        let carrier_standard = standard(&carriers);
        author_retained(
            &carriers,
            &format!(
                "{}{}declarations.begin\n(units (use owned-worklists {} {}))\ndeclarations.end\n{WORKLIST_CARRIERS}",
                dependency(&generic),
                dependency(&carrier_standard),
                generic.package,
                generic.revision,
            ),
            "worklist-carriers",
        );
        author_retained(
            &carriers,
            &format!(
                "declarations.begin\n(units (use owned-worklists {} {}) (use owned-read-results {} {}))\ndeclarations.end\n{CARRIERS}",
                generic.package, generic.revision, generic.package, generic.revision,
            ),
            "read-carriers",
        );
        unchanged(&carriers, "worklist-carriers");
        let draft = unchanged(&carriers, "read-carriers");
        assert!(draft.contains("(borrow-from "));
        let concrete = export(&carriers);
        let consumer = evidence_native("command");
        stage(&consumer, &generic);
        stage(&consumer, &concrete);
        Self {
            library,
            carriers,
            consumer,
            generic,
            concrete,
            standards: [library_standard.path, carrier_standard.path],
        }
    }

    fn source(&self, body: &str) -> String {
        format!(
            "{}{}declarations.begin\n(units (use owned-worklists {} {}) (use owned-read-results {} {}) (use worklist-carriers {} {}) (use read-carriers {} {}))\ndeclarations.end\n{body}",
            dependency(&self.generic),
            dependency(&self.concrete),
            self.generic.package,
            self.generic.revision,
            self.generic.package,
            self.generic.revision,
            self.concrete.package,
            self.concrete.revision,
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
            for public in [self.library, self.carriers, self.consumer] {
                println!(
                    "retained owned-read-results public evidence: {}",
                    public.root.keep().display()
                );
            }
        }
    }
}

fn author_retained(public: &Native, source: &str, name: &str) {
    author(public, source);
    std::fs::copy(
        public.root.path().join("native.lkjc"),
        public.root.path().join(format!("{name}.lkjc")),
    )
    .unwrap();
}

fn unchanged_labeled(public: &Native, module: &str, label: &str) -> String {
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

fn scenario_expected(inputs: &[i64]) -> Value {
    let selected: Vec<i64> = inputs.iter().max().copied().into_iter().collect();
    // Reverse comparison uses the original index as the stable tie breaker.
    // This independent host oracle observes the selected value, not just its key.
    let tied_selected: Vec<i64> = inputs
        .iter()
        .enumerate()
        .max_by_key(|(index, n)| (**n % 4, std::cmp::Reverse(*index)))
        .map(|(_, n)| *n)
        .into_iter()
        .collect();
    let drained: Vec<i64> = inputs.iter().rev().copied().collect();
    json!({ "length": inputs.len(), "selected": selected, "tied-selected": tied_selected,
        "drained": drained, "empty-length": 0, "reused": [17] })
}

fn all_expected(inputs: &[i64]) -> Value {
    let scenario = scenario_expected(inputs);
    json!({ "cell-flat": scenario, "cell-chunked": scenario,
        "buffer-flat": scenario, "buffer-chunked": scenario })
}

#[test]
fn native_owned_read_results_three_packages_select_first_max_drain_reuse_and_detach() {
    let packages = Packages::stage();
    let public = &packages.consumer;
    author(
        public,
        &packages.source(&format!("{APPLICATION}{UNCHECKED_CONSUMER}")),
    );
    let main = declaration(public, "owned-read-app", "main");
    let scenario = declaration(public, "owned-read-app", "scenario");
    let draft = unchanged_labeled(public, "owned-read-app", "original");
    assert!(draft.contains("(borrow-call "));
    // Canonical edits preserve function identities, and subsequent re-entry stays exact.
    assert!(draft.contains("(i64 17)"));
    let edit = public.input(
        "identity-edit.lkjc",
        &draft.replacen("(i64 17)", "(i64 19)", 1),
    );
    let plan = public.plan(&edit, true);
    public.apply(&edit, &plan, true);
    assert_eq!(declaration(public, "owned-read-app", "main"), main);
    assert_eq!(declaration(public, "owned-read-app", "scenario"), scenario);
    let edited = unchanged_labeled(public, "owned-read-app", "edited");
    let restore = public.input(
        "identity-restore.lkjc",
        &edited.replacen("(i64 19)", "(i64 17)", 1),
    );
    let plan = public.plan(&restore, true);
    public.apply(&restore, &plan, true);
    public.cli(&["check"], true);
    assert_eq!(declaration(public, "owned-read-app", "scenario"), scenario);
    unchanged_labeled(public, "owned-read-app", "restored");

    let artifact = public.root.path().join("owned-read-results.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = deployment(public, "owned-read-results.lkja", "owned-read-results");
    let unchecked_descriptor = deployment(public, "owned-read-results.lkja", "read-unchecked");
    let targets = [
        "read-cell-flat",
        "read-cell-chunked",
        "read-buffer-flat",
        "read-buffer-chunked",
    ];
    let descriptors: Vec<_> = targets
        .iter()
        .map(|target| deployment(public, "owned-read-results.lkja", target))
        .collect();
    let mut workloads = vec![vec![], vec![2], vec![2, 7, 11, 3], vec![0, 255, 128]];
    for length in [31, 32, 33, 513] {
        workloads.push((0..length).map(|n| n % 17).collect());
    }
    assert_eq!(scenario_expected(&[2, 7, 11, 3])["selected"], json!([11]));
    assert_eq!(
        scenario_expected(&[2, 7, 11, 3])["tied-selected"],
        json!([7])
    );
    for detached in [false, true] {
        if detached {
            packages.detach();
        }
        let input = public.input(&format!("empty-selector-{detached}.json"), "[[]]");
        let result = public
            .root
            .path()
            .join(format!("empty-selector-{detached}-result.json"));
        let mut args = if detached {
            vec!["run", "--deployment", path(&unchecked_descriptor)]
        } else {
            vec!["run", "read-unchecked"]
        };
        args.extend([
            "--arguments-file",
            path(&input),
            "--result-file",
            path(&result),
        ]);
        let rejected = public.cli(&args, false);
        assert!(
            rejected
                .iter()
                .any(|record| record.operation == "diagnostic")
        );
        assert!(!result.exists());
        run_expected(
            public,
            "read-unchecked",
            &unchecked_descriptor,
            detached,
            "healthy-after-empty",
            json!([[2, 7, 11, 3]]),
            &json!(11),
        );
        for (case, inputs) in workloads.iter().enumerate() {
            run_expected(
                public,
                "owned-read-results",
                &descriptor,
                detached,
                &case.to_string(),
                json!([inputs]),
                &all_expected(inputs),
            );
        }
        let inputs = workloads.last().unwrap();
        for (target, descriptor) in targets.iter().zip(&descriptors) {
            run_expected(
                public,
                target,
                descriptor,
                detached,
                "513",
                json!([inputs]),
                &scenario_expected(inputs),
            );
        }
    }
    packages.retain();
}

const ROOT_VIEWS: &str = r#"declarations.begin
(units (module create source-results
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create read-cell (visibility private) (implementation core.cell.read)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64))
  (external create finish-cell (visibility private) (implementation core.cell.extract)
    (parameter create value (type OwnedI64Cell) (use consume)) (returns I64))
  (function create direct (visibility public) (effect pure)
    (parameter create source (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from source)) (body (local source)))
  (function create forward (visibility public) (effect pure)
    (parameter create source (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from source))
    (body (borrow-call (call direct (local source))
      (binding view (type OwnedI64Cell)) (in (local view)))))
  (function create choose (visibility public) (effect pure)
    (parameter create first (type OwnedI64Cell) (use borrow))
    (parameter create second (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from first)) (body (local first)))
  (function create main (visibility public) (effect pure) (returns I64)
    (body (let
      (binding source (type OwnedI64Cell) (call new-cell (i64 42)))
      (binding observed (type I64)
        (borrow-call (call forward (local source)) (binding outer (type OwnedI64Cell))
          (in (borrow-call (call choose (local outer) (local source))
            (binding inner (type OwnedI64Cell))
            (in (sequence (call read-cell (local outer)) (call read-cell (local inner))))))))
      (in (sequence (call finish-cell (local source)) (local observed))))))
  (component create command (visibility private)
    (port create main (type (function () I64)) (function main))))
  (target create read-root (component source-results::command)
    (runner command) (port source-results::command::main)))
declarations.end
"#;

#[test]
fn native_owned_read_results_forward_root_and_sibling_reads_then_consume() {
    let public = evidence_native("command");
    author(&public, ROOT_VIEWS);
    let draft = unchanged(&public, "source-results");
    assert!(draft.contains("(borrow-from "));
    let artifact = public.root.path().join("read-root.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = deployment(&public, "read-root.lkja", "read-root");
    run_expected(
        &public,
        "read-root",
        &descriptor,
        false,
        "root",
        json!([]),
        &json!(42),
    );
    std::fs::remove_dir_all(&public.project).unwrap();
    run_expected(
        &public,
        "read-root",
        &descriptor,
        true,
        "root",
        json!([]),
        &json!(42),
    );
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        println!(
            "retained direct read-result evidence: {}",
            public.root.keep().display()
        );
    }
}

#[test]
fn native_owned_read_results_reject_escapes_wrong_source_and_protected_consumption_without_publication()
 {
    let public = evidence_native("command");
    let before = public.revision();
    let cases = [
        (
            "wrong-source",
            "(body (local first))",
            "(body (local second))",
        ),
        (
            "untaken-wrong-source",
            "(body (local first))",
            "(body (if (bool true) (local first) (local second)))",
        ),
        (
            "local-owner-escape",
            "(body (local source))",
            "(body (let (binding local-owner (type OwnedI64Cell) (call new-cell (i64 7))) (in (local local-owner))))",
        ),
        (
            "owning-result-for-borrow-body",
            "(returns OwnedI64Cell (borrow-from source))",
            "(returns OwnedI64Cell)",
        ),
        (
            "consume-result-source",
            "(parameter create source (type OwnedI64Cell) (use borrow))",
            "(parameter create source (type OwnedI64Cell) (use consume))",
        ),
        (
            "owning-binding",
            "(call new-cell (i64 42))",
            "(call direct (call new-cell (i64 42)))",
        ),
        (
            "protected-root-consumption",
            "(sequence (call read-cell (local outer)) (call read-cell (local inner)))",
            "(sequence (call finish-cell (local source)) (call read-cell (local inner)))",
        ),
        (
            "borrow-consumption",
            "(sequence (call read-cell (local outer)) (call read-cell (local inner)))",
            "(sequence (call finish-cell (local outer)) (call read-cell (local inner)))",
        ),
        (
            "ordinary-call-return-forwarding",
            "(borrow-call (call direct (local source))\n      (binding view (type OwnedI64Cell)) (in (local view)))",
            "(call direct (local source))",
        ),
    ];
    for (name, from, to) in cases {
        assert!(ROOT_VIEWS.contains(from), "negative fixture: {name}");
        rejected(&public, &ROOT_VIEWS.replacen(from, to, 1), &before, name);
    }
    // Admit the unchanged independent source after every failed candidate.
    author(&public, ROOT_VIEWS);
    unchanged(&public, "source-results");
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        println!(
            "retained read-result rejection evidence: {}",
            public.root.keep().display()
        );
    }
}

const METHOD_RESULTS: &str = r#"declarations.begin
(units (module create result-methods
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_8b000000000000000000000000000001 at
      (parameters (I64 unrestricted) (Self borrow))
      (returns Self (borrow-from 1)) (effect pure)))
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create read-cell (visibility private) (implementation core.cell.read)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64))
  (function create at (visibility public) (effect pure)
    (parameter create index (type I64))
    (parameter create source (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from source)) (body (local source)))
  (owned-implementation create Cells (visibility public)
    (contract Reader) (self OwnedI64Cell)
    (method method_8b000000000000000000000000000001 at))
  (function create main (visibility public) (effect pure) (returns I64)
    (body (let (binding source (type OwnedI64Cell) (call new-cell (i64 42)))
      (in (borrow-call
        (method-call concrete@Cells Reader method_8b000000000000000000000000000001
          (i64 0) (local source))
        (binding view (type OwnedI64Cell)) (in (call read-cell (local view))))))))
  (component create command (visibility private)
    (port create main (type (function () I64)) (function main))))
  (target create read-method (component result-methods::command)
    (runner command) (port result-methods::command::main)))
declarations.end
"#;

#[test]
fn native_owned_read_results_match_exact_method_source_positions_before_publication() {
    let public = evidence_native("command");
    let before = public.revision();
    for (name, from, to) in [
        (
            "ordinary-method-source",
            "(returns Self (borrow-from 1))",
            "(returns Self (borrow-from 0))",
        ),
        (
            "out-of-range-method-source",
            "(returns Self (borrow-from 1))",
            "(returns Self (borrow-from 2))",
        ),
        (
            "missing-method-result-mode",
            "(returns Self (borrow-from 1))",
            "(returns Self)",
        ),
        (
            "consuming-method-source",
            "(parameters (I64 unrestricted) (Self borrow))",
            "(parameters (I64 unrestricted) (Self consume))",
        ),
        (
            "task-method-result",
            "(returns Self (borrow-from 1)) (effect pure)",
            "(returns Self (borrow-from 1)) (effect (task))",
        ),
    ] {
        assert!(METHOD_RESULTS.contains(from), "negative fixture: {name}");
        rejected(
            &public,
            &METHOD_RESULTS.replacen(from, to, 1),
            &before,
            name,
        );
    }
    author(&public, METHOD_RESULTS);
    let draft = unchanged(&public, "result-methods");
    assert!(draft.contains("(borrow-from 1)"));
    let artifact = public.root.path().join("read-method.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = deployment(&public, "read-method.lkja", "read-method");
    run_expected(
        &public,
        "read-method",
        &descriptor,
        false,
        "position",
        json!([]),
        &json!(42),
    );
    std::fs::remove_dir_all(&public.project).unwrap();
    run_expected(
        &public,
        "read-method",
        &descriptor,
        true,
        "position",
        json!([]),
        &json!(42),
    );
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        println!(
            "retained exact method-result evidence: {}",
            public.root.keep().display()
        );
    }
}

// A deliberately unchecked caller gives empty selection its existing indexed-read
// trap while leaving the ordinary maintained application length-checked.
const UNCHECKED_CONSUMER: &str = r#"declarations.begin
(units (module create read-result-failure
  (function create main (visibility public) (effect pure)
    (parameter create inputs (type (list I64))) (returns I64)
    (body (let
      (binding storage (type (owned-sequence OwnedI64Cell))
        (implementation-call owned-worklists::build
          (types OwnedI64Cell (owned-sequence OwnedI64Cell))
          (implementations concrete@worklist-carriers::Scalar concrete@worklist-carriers::FlatCells)
          (local inputs)))
      (in (borrow-call
        (implementation-call owned-read-results::select-max
          (types OwnedI64Cell (owned-sequence OwnedI64Cell))
          (implementations concrete@worklist-carriers::Scalar concrete@read-carriers::FlatCellReader)
          (local storage))
        (binding selected (type OwnedI64Cell))
        (in (call worklist-carriers::cell-read (local selected))))))))
  (component create command (visibility private)
    (port create main (type (function ((list I64)) I64)) (function main))))
  (target create read-unchecked (component read-result-failure::command)
    (runner command) (port read-result-failure::command::main)))
declarations.end
"#;
