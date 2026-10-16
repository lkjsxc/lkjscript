//! Explicit implementation schemes exported before independently authored items.
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
const APPLICATION: &str =
    include_str!("../../examples/generic-owned-implementations/application.lkjc");

pub(super) enum PureLane<'a> {
    Source(&'a str),
    Detached(&'a Path),
    Process(&'a Path),
}

pub(super) fn pure_row(
    id: impl Into<String>,
    target: &str,
    arguments: Value,
    expected: Value,
) -> (BatchCase, Value) {
    let mut case = BatchCase::success(id, &arguments, expected.clone());
    case.target = Some(target.to_owned());
    (case, expected)
}

pub(super) fn pure_batch(
    public: &Native,
    label: &str,
    lane: PureLane<'_>,
    rows: Vec<(BatchCase, Value)>,
) {
    use lkjscript::platform::json::{
        JsonIntegerPolicy, JsonLimits, decode_strict_with_integer_policy,
    };
    let source = matches!(&lane, PureLane::Source(_));
    let (cases, expected): (Vec<_>, Vec<_>) = rows.into_iter().unzip();
    let observed = match lane {
        PureLane::Source(target) => public.source_batch_outputs(label, target, &cases),
        PureLane::Detached(descriptor) => batch::execute_in(
            public.mode,
            public.root.path(),
            &public.executable,
            label,
            descriptor,
            &cases,
        ),
        // This one original nonempty detached row retains an actual process,
        // including when the surrounding finite matrix uses host adapters.
        PureLane::Process(descriptor) => batch::execute(
            public.root.path(),
            &public.executable,
            label,
            descriptor,
            &cases,
        ),
    };
    let prefix = if source { "source-batch" } else { "batch" };
    let outputs = std::fs::read_to_string(
        public
            .root
            .path()
            .join(format!("{prefix}-{label}.result.jsonl")),
    )
    .unwrap();
    assert_eq!(outputs.lines().count(), cases.len());
    for (index, line) in outputs.lines().enumerate() {
        // Keep run_expected's canonical result-file byte oracle, in addition
        // to the shared reader's complete rows, bindings and cleanup checks.
        let row = decode_strict_with_integer_policy(
            line.as_bytes(),
            JsonLimits {
                maximum_bytes: 4 * 1024 * 1024,
                maximum_string_bytes: 4 * 1024 * 1024,
                ..JsonLimits::default()
            },
            JsonIntegerPolicy::SignedOrUnsigned64,
        )
        .unwrap();
        assert_eq!(row["id"].as_str(), Some(cases[index].id.as_str()));
        assert_eq!(row["exit_code"], 0);
        assert_eq!(
            row["result_json"].as_str().unwrap().as_bytes(),
            serde_json::to_vec(&expected[index]).unwrap(),
            "{}",
            cases[index].id,
        );
        let records = parse_records("implementation pure row", observed[index].as_bytes()).unwrap();
        let execution = compact_record(&records, "execution");
        if source {
            assert_eq!(compact_field(execution, "differential"), "equal");
            for field in ["production-instructions", "reference-expressions"] {
                assert!(compact_field(execution, field).parse::<u64>().unwrap() > 0);
            }
        } else {
            let production: Value =
                serde_json::from_str(compact_field(execution, "production-observation")).unwrap();
            assert_eq!(production["capability_calls"], 0);
            assert_eq!(production["live_handles_after"], 0);
            let cleanup: Value = serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
            assert_eq!(cleanup["remaining_tasks"], 0);
            assert_eq!(cleanup["cleanup_failures"], json!([]));
        }
    }
}

fn evidence_native(template: &str) -> Native {
    let mut public = Native::memory_project(template);
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        public.root.disable_cleanup(true);
        println!(
            "generic implementation evidence root: {}",
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

struct Packages {
    library: Native,
    elements: Native,
    consumer: Native,
    generic: Export,
    concrete: Export,
    standards: [PathBuf; 2],
}

impl Packages {
    fn library() -> (Native, Export) {
        let library = evidence_native("minimal");
        let supplier = standard(&library);
        author_retained(
            &library,
            &format!("{}{WORKLISTS}", dependency(&supplier)),
            "worklists",
        );
        author_retained(&library, READERS, "readers");
        (library, supplier)
    }

    fn stage() -> Self {
        let (library, supplier) = Self::library();
        author_retained(&library, STORAGE, "storage-schemes");
        let draft = unchanged_labeled(&library, "generic-storage", "original");
        for absent in ["OwnedI64Cell", "ByteBuffer", "product-create"] {
            assert!(
                !draft.contains(absent),
                "scheme library must precede downstream items: {absent}"
            );
        }
        for present in [
            "(owned-implementation ",
            "(type-parameter ",
            "(types ",
            "(implementation ",
            "(read-from ",
        ] {
            assert!(draft.contains(present), "scheme draft: {present}");
        }
        let scheme = declaration(&library, "generic-storage", "ReverseReader");
        let function = declaration(&library, "generic-storage", "reverse-at");
        library.cli(&["inspect", "owner", "owned_implementation", &scheme], true);
        // A canonical child replacement retains both implementation and function identities.
        assert!(draft.contains("(i64 1)"));
        let edit = library.input(
            "scheme-helper-edit.lkjc",
            &draft.replacen("(i64 1)", "(if (bool true) (i64 1) (i64 0))", 1),
        );
        let plan = library.plan(&edit, true);
        library.apply(&edit, &plan, true);
        library.cli(&["check"], true);
        assert_eq!(
            declaration(&library, "generic-storage", "ReverseReader"),
            scheme
        );
        assert_eq!(
            declaration(&library, "generic-storage", "reverse-at"),
            function
        );
        unchanged_labeled(&library, "generic-storage", "edited");
        // No downstream item package exists when the complete scheme library is exported.
        Self::downstream(library, supplier)
    }

    fn downstream(library: Native, supplier: Export) -> Self {
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
        let draft = unchanged(&elements, "generic-elements");
        assert!(draft.contains("(owned-product "));
        for absent in ["(owned-sequence ", "FlatReader", "ChunkedReader"] {
            assert!(
                !draft.contains(absent),
                "elements cannot introduce storage specialization: {absent}"
            );
        }
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
            "{}{}declarations.begin\n(units (use owned-worklists {} {}) (use owned-read-results {} {}) (use generic-storage {} {}) (use generic-elements {} {}))\ndeclarations.end\n{body}",
            dependency(&self.generic),
            dependency(&self.concrete),
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
        for public in [&self.library, &self.elements, &self.consumer] {
            public.remove_project();
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
                    "retained generic implementation evidence: {}",
                    public.root.keep().display()
                );
            }
        }
    }
}

fn scenario_expected(inputs: &[i64], offset: i64, reverse: bool, reuse: i64) -> Value {
    let values: Vec<_> = inputs.iter().map(|n| n + offset).collect();
    let mut observed = values.clone();
    if reverse {
        observed.reverse();
    }
    let selected: Vec<_> = observed.iter().max().copied().into_iter().collect();
    // Ties are observed through the identity observer, rather than comparing keys alone.
    let tied_selected: Vec<_> = observed
        .iter()
        .enumerate()
        .max_by_key(|(index, n)| (**n % 4, std::cmp::Reverse(*index)))
        .map(|(_, n)| *n)
        .into_iter()
        .collect();
    let drained: Vec<_> = values.iter().rev().copied().collect();
    json!({"length":inputs.len(), "selected":selected, "tied-selected":tied_selected,
        "drained":drained, "empty-length":0, "reused":[reuse + offset], "moved":values})
}

fn all_expected(inputs: &[i64], reuse: i64) -> Value {
    let ordinary = scenario_expected(inputs, 0, false, reuse);
    let product = scenario_expected(inputs, 12, false, reuse);
    json!({"cell-flat":ordinary, "cell-chunked":ordinary,
        "buffer-flat":ordinary, "buffer-chunked":ordinary,
        "product-flat":product, "product-chunked":product,
        "cell-reverse":scenario_expected(inputs, 0, true, reuse)})
}

#[test]
fn native_owned_generic_implementation_schemes_three_packages_nested_products_read_transfer_reuse_parallel_and_detach()
 {
    let packages = Packages::stage();
    let public = &packages.consumer;
    author_retained(public, &packages.source(APPLICATION), "consumer");
    let main = declaration(public, "generic-owned-app", "main");
    let scenario = declaration(public, "generic-owned-app", "scenario");
    let draft = unchanged_labeled(public, "generic-owned-app", "original");
    assert!(draft.contains("(implementation "));
    assert!(
        draft
            .split_whitespace()
            .any(|token| token == "(borrow-call")
    );
    let artifact = public
        .root
        .path()
        .join("generic-owned-implementations.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = deployment(
        public,
        "generic-owned-implementations.lkja",
        "generic-owned-implementations",
    );
    let direct = deployment(
        public,
        "generic-owned-implementations.lkja",
        "generic-direct-length",
    );
    let parallel = deployment(
        public,
        "generic-owned-implementations.lkja",
        "generic-parallel-lengths",
    );
    let cases = [
        ("generic-cell-flat", 0, false),
        ("generic-cell-chunked", 0, false),
        ("generic-buffer-flat", 0, false),
        ("generic-buffer-chunked", 0, false),
        ("generic-product-flat", 12, false),
        ("generic-product-chunked", 12, false),
        ("generic-cell-reverse", 0, true),
    ];
    let descriptors: Vec<_> = cases
        .iter()
        .map(|(target, _, _)| deployment(public, "generic-owned-implementations.lkja", target))
        .collect();
    let mut workloads = vec![vec![], vec![2], vec![2, 7, 11, 3], vec![0, 255, 128]];
    for length in [31, 32, 33] {
        workloads.push((0..length).map(|n| n % 17).collect());
    }
    // Each representation receives the full large workload independently under
    // unchanged allocation limits; the seven-scenario combined invocation is larger.
    let boundary_inputs: Vec<_> = (0..513).map(|n| n % 17).collect();
    assert_eq!(
        all_expected(&[2, 7, 11, 3], 17)["cell-flat"]["tied-selected"],
        json!([7])
    );
    assert_eq!(
        all_expected(&[2, 7, 11, 3], 17)["cell-reverse"]["tied-selected"],
        json!([3])
    );
    assert_eq!(
        all_expected(&[2, 7, 11, 3], 17)["product-flat"]["selected"],
        json!([23])
    );
    pure_batch(
        public,
        "generic-original",
        PureLane::Source("generic-owned-implementations"),
        workloads
            .iter()
            .enumerate()
            .map(|(case, inputs)| {
                pure_row(
                    format!("workload-{case}"),
                    "generic-owned-implementations",
                    json!([inputs]),
                    all_expected(inputs, 17),
                )
            })
            .collect(),
    );
    // Demonstrate a meaningful accepted edit before restoring the independently tested source.
    assert!(draft.contains("(i64 17)"));
    let edit = public.input(
        "consumer-edit.lkjc",
        &draft.replacen("(i64 17)", "(i64 19)", 1),
    );
    let plan = public.plan(&edit, true);
    public.apply(&edit, &plan, true);
    assert_eq!(declaration(public, "generic-owned-app", "main"), main);
    assert_eq!(
        declaration(public, "generic-owned-app", "scenario"),
        scenario
    );
    run_expected(
        public,
        "generic-owned-implementations",
        &descriptor,
        false,
        "edited",
        json!([[2, 7, 11, 3]]),
        &all_expected(&[2, 7, 11, 3], 19),
    );
    let edited = unchanged_labeled(public, "generic-owned-app", "edited");
    let restore = public.input(
        "consumer-restore.lkjc",
        &edited.replacen("(i64 19)", "(i64 17)", 1),
    );
    let plan = public.plan(&restore, true);
    public.apply(&restore, &plan, true);
    public.cli(&["check"], true);
    unchanged_labeled(public, "generic-owned-app", "restored");
    assert_eq!(
        declaration(public, "generic-owned-app", "scenario"),
        scenario
    );
    for detached in [false, true] {
        if detached {
            packages.detach();
        }
        run_expected(
            public,
            "generic-direct-length",
            &direct,
            detached,
            "direct",
            json!([]),
            &json!(1),
        );
        for length in [0, 1, 33] {
            let inputs: Vec<_> = (0..length).collect();
            run_expected(
                public,
                "generic-parallel-lengths",
                &parallel,
                true,
                &format!("sources-removed-{detached}-{length}"),
                json!([inputs]),
                &json!({"left":length,"right":length}),
            );
        }
        // These pure rows stay after the three ordinary task invocations;
        // source preparation never crosses the accepted edit/restore boundary.
        let mut rows: Vec<_> = cases
            .iter()
            .map(|(target, offset, reverse)| {
                pure_row(
                    format!("{target}-513"),
                    target,
                    json!([boundary_inputs]),
                    scenario_expected(&boundary_inputs, *offset, *reverse, 17),
                )
            })
            .collect();
        if detached {
            // The first full513 row is the original first representation, now
            // an explicit actual-process canary after every source is removed.
            pure_batch(
                public,
                "generic-detached-process",
                PureLane::Process(&descriptors[0]),
                vec![rows.remove(0)],
            );
            for (case, inputs) in workloads.iter().enumerate() {
                rows.push(pure_row(
                    format!("workload-{case}"),
                    "generic-owned-implementations",
                    json!([inputs]),
                    all_expected(inputs, 17),
                ));
            }
            assert_eq!(rows.len(), 13);
            pure_batch(
                public,
                "generic-detached",
                PureLane::Detached(&descriptor),
                rows,
            );
        } else {
            assert_eq!(rows.len(), 7);
            pure_batch(
                public,
                "generic-restored",
                PureLane::Source("generic-owned-implementations"),
                rows,
            );
        }
    }
    packages.retain();
}

#[test]
fn native_owned_generic_implementation_schemes_reject_invalid_unused_mappings_and_applications_without_publication()
 {
    let (library, supplier) = Packages::library();
    let before = library.revision();
    for (name, from, to) in [
        (
            "missing-owned-bound",
            "(type-parameter create T (constraint owned))",
            "(type-parameter create T)",
        ),
        (
            "wrong-mapped-type",
            "std::sequence-empty (types T)",
            "std::sequence-empty (types ByteBuffer)",
        ),
        (
            "missing-mapped-argument",
            "std::sequence-empty (types T)",
            "std::sequence-empty",
        ),
        (
            "extra-mapped-argument",
            "std::sequence-empty (types T)",
            "std::sequence-empty (types T T)",
        ),
        (
            "ordinary-contract-argument",
            "(self (owned-sequence T)) (types T)",
            "(self (owned-sequence T)) (types I64)",
        ),
        (
            "wrong-unused-reader-map",
            "reverse-at (types T)",
            "std::sequence-empty (types T)",
        ),
        (
            "wrong-task-map",
            "task-length (types T)",
            "std::sequence-length (types T)",
        ),
    ] {
        assert!(STORAGE.contains(from), "negative scheme fixture: {name}");
        rejected(&library, &STORAGE.replacen(from, to, 1), &before, name);
    }
    author_retained(&library, STORAGE, "healthy-storage-after-rejections");
    unchanged(&library, "generic-storage");
    let packages = Packages::downstream(library, supplier);
    let public = &packages.consumer;
    let before = public.revision();
    let original = "(implementation generic-storage::Flat (types OwnedI64Cell))";
    for (name, replacement) in [
        ("missing-scheme-argument", "generic-storage::Flat"),
        (
            "extra-scheme-argument",
            "(implementation generic-storage::Flat (types OwnedI64Cell ByteBuffer))",
        ),
        (
            "wrong-scheme-argument",
            "(implementation generic-storage::Flat (types ByteBuffer))",
        ),
        (
            "ordinary-scheme-argument",
            "(implementation generic-storage::Flat (types I64))",
        ),
        (
            "wrong-contract-scheme",
            "(implementation generic-storage::FlatReader (types OwnedI64Cell))",
        ),
    ] {
        assert!(APPLICATION.contains(original));
        rejected(
            public,
            &packages.source(&APPLICATION.replacen(original, replacement, 1)),
            &before,
            name,
        );
    }
    let original = "(implementation generic-storage::Flat (types Product))";
    rejected(
        public,
        &packages.source(&APPLICATION.replacen(
            original,
            "(implementation generic-storage::Flat (types (list OwnedI64Cell)))",
            1,
        )),
        &before,
        "hidden-owned-in-ordinary-argument",
    );
    rejected(
        public,
        &packages.source(&APPLICATION.replacen(
            "(type-parameter create U (constraint owned transferable))",
            "(type-parameter create U (constraint owned))",
            1,
        )),
        &before,
        "untransferable-symbolic-parallel-item",
    );
    author_retained(
        public,
        &packages.source(APPLICATION),
        "healthy-consumer-after-rejections",
    );
    unchanged(public, "generic-owned-app");
    let artifact = public.root.path().join("healthy-generic.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = deployment(
        public,
        "healthy-generic.lkja",
        "generic-owned-implementations",
    );
    run_expected(
        public,
        "generic-owned-implementations",
        &descriptor,
        false,
        "healthy",
        json!([[2, 7, 11, 3]]),
        &all_expected(&[2, 7, 11, 3], 17),
    );
    packages.retain();
}

const FLOW_LIBRARY: &str = r#"declarations.begin
(units (module create flow
  (owned-contract create Bounce (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Other (constraint owned))
    (method method_8d000000000000000000000000000001 bounce
      (parameters) (returns I64) (effect pure)))
  (function create dispatch (visibility public) (effect pure)
    (type-parameter create S (constraint owned))
    (type-parameter create P (constraint owned))
    (implementation-parameter implparam_8d000000000000000000000000000001 witness Bounce S (types P))
    (returns I64)
    (body (method-call parameter@dispatch@implparam_8d000000000000000000000000000001
      Bounce method_8d000000000000000000000000000001)))))
declarations.end
"#;

const FLOW_CONSUMER: &str = r#"declarations.begin
(units (module create scheme-flow
  (function create step (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (type-parameter create U (constraint owned))
    (returns I64)
    (body (if (bool true) (i64 0)
      (implementation-call upstream::dispatch (types T U)
        (implementations (implementation Scheme (types T U)))))))
  (owned-implementation create Scheme (visibility public)
    (type-parameter create T (constraint owned))
    (type-parameter create U (constraint owned))
    (contract upstream::Bounce) (self T) (types U)
    (method method_8d000000000000000000000000000001 step (types T U)))
  (function create main (visibility public) (effect pure) (returns I64)
    (body (call step (types OwnedI64Cell ByteBuffer))))
  (component create command (visibility private)
    (port create main (type (function () I64)) (function main))))
  (target create finite-scheme-flow (component scheme-flow::command)
    (runner command) (port scheme-flow::command::main)))
declarations.end
"#;

#[test]
fn native_owned_generic_implementation_schemes_cross_package_expansion_rejects_plain_and_permutation_admit()
 {
    let supplier = evidence_native("minimal");
    author_retained(&supplier, FLOW_LIBRARY, "flow-supplier");
    unchanged(&supplier, "flow");
    let exported = export(&supplier);
    let mut consumers = Vec::new();
    for (label, body) in [
        ("plain", FLOW_CONSUMER.to_owned()),
        ("permutation", FLOW_CONSUMER.replacen(
            "upstream::dispatch (types T U)\n        (implementations (implementation Scheme (types T U)))",
            "upstream::dispatch (types U T)\n        (implementations (implementation Scheme (types U T)))", 1)),
    ] {
        let public = evidence_native("command");
        stage(&public, &exported);
        let prefix = format!(
            "{}declarations.begin\n(units (use upstream {} {}))\ndeclarations.end\n",
            dependency(&exported), exported.package, exported.revision,
        );
        let before = public.revision();
        let expanding = FLOW_CONSUMER.replacen(
            "upstream::dispatch (types T U)\n        (implementations (implementation Scheme (types T U)))",
            "upstream::dispatch (types (owned-sequence T) U)\n        (implementations (implementation Scheme (types (owned-sequence T) U)))", 1,
        );
        assert_ne!(expanding, FLOW_CONSUMER);
        let input = public.input(&format!("invalid-expanding-{label}.lkjc"),
            &format!("request base={before}\n{prefix}{expanding}"));
        let records = public.plan(&input, false);
        assert!(records.iter().any(|record| record.operation == "diagnostic"
            && compact_field(record, "code") == "kernel_callable_expansion"),
            "mapped cross-package growth must reject semantically: {records:?}");
        assert_eq!(public.revision(), before);
        if label == "permutation" {
            assert_ne!(body, FLOW_CONSUMER);
        }
        author_retained(&public, &format!("{prefix}{body}"), label);
        unchanged(&public, "scheme-flow");
        let artifact = public.root.path().join("finite-scheme-flow.lkja");
        public.cli(&["build", "--output", path(&artifact)], true);
        let descriptor = deployment(&public, "finite-scheme-flow.lkja", "finite-scheme-flow");
        run_expected(&public, "finite-scheme-flow", &descriptor, false, label, json!([]), &json!(0));
        consumers.push((public, descriptor, label));
    }
    supplier.remove_project();
    std::fs::remove_file(&exported.path).unwrap();
    for (public, descriptor, label) in consumers {
        public.remove_project();
        run_expected(
            &public,
            "finite-scheme-flow",
            &descriptor,
            true,
            label,
            json!([]),
            &json!(0),
        );
        if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
            println!(
                "retained finite scheme-flow evidence: {}",
                public.root.keep().display()
            );
        }
    }
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        println!(
            "retained finite scheme-flow supplier: {}",
            supplier.root.keep().display()
        );
    }
}
