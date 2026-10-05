//! Independently authored prerequisite witnesses, borrowed selection and joined tasks.
use super::native_byte_buffer::{Export, author, dependency, export, stage};
use super::native_owned_worklists::{declaration, deployment, rejected, run_expected, standard};
use super::*;
use serde_json::json;

const WORKLISTS: &str = include_str!("../../examples/owned-worklists/library.lkjc");
const READERS: &str = include_str!("../../examples/owned-read-results/library.lkjc");
const STORAGE: &str = include_str!("../../examples/generic-owned-implementations/storage.lkjc");
const ELEMENTS: &str = include_str!("../../examples/generic-owned-implementations/elements.lkjc");
const ADAPTERS: &str =
    include_str!("../../examples/composable-owned-implementations/adapters.lkjc");
const APPLICATION: &str =
    include_str!("../../examples/composable-owned-implementations/application.lkjc");

fn evidence_native(template: &str) -> Native {
    let mut public = Native::template(template);
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        public.root.disable_cleanup(true);
        println!(
            "composable implementation evidence root: {}",
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

fn draft_alias(draft: &str, family: &str, exact_tail: &str) -> String {
    let prefix = format!("({family} ");
    draft
        .match_indices(&prefix)
        .find_map(|(offset, _)| {
            let remaining = &draft[offset..];
            let form = &remaining[form_span(remaining, &prefix)];
            form.ends_with(&format!(" {exact_tail})"))
                .then(|| form.split_whitespace().nth(1).unwrap().to_owned())
        })
        .expect("canonical draft must contain an alias for the exact locator")
}

fn assert_nested_draft_identity(packages: &Packages, draft: &str) {
    // Resolve aliases only by their exact public locators, then compare the entire
    // nested literal. Re-entry must retain each ordered type and prerequisite.
    let draft = draft.split_whitespace().collect::<Vec<_>>().join(" ");
    let delegate = draft_alias(
        &draft,
        "reference",
        &format!(
            "declaration {}/{}",
            packages.generic.package,
            declaration(&packages.library, "composable-adapters", "DelegatingReader")
        ),
    );
    let storage = draft_alias(&draft, "type-alias", "(owned-sequence OwnedI64Cell)");
    for (function, leaf, excluded) in [
        ("cell-nested", "FlatReader", "ReverseReader"),
        ("cell-nested-reverse", "ReverseReader", "FlatReader"),
    ] {
        let leaf = draft_alias(
            &draft,
            "reference",
            &format!(
                "declaration {}/{}",
                packages.generic.package,
                declaration(&packages.library, "generic-storage", leaf)
            ),
        );
        let excluded = draft_alias(
            &draft,
            "reference",
            &format!(
                "declaration {}/{}",
                packages.generic.package,
                declaration(&packages.library, "generic-storage", excluded)
            ),
        );
        let prefix = format!(
            "(function edit {} {function}",
            declaration(&packages.consumer, "composable-owned-app", function)
        );
        let function = &draft[form_span(&draft, &prefix)];
        let expected = format!(
            "(implementation {delegate} (types OwnedI64Cell {storage}) (implementations (implementation {delegate} (types OwnedI64Cell {storage}) (implementations (implementation {leaf} (types OwnedI64Cell))))))"
        );
        assert!(
            function.contains(&expected),
            "full nested identity: {function}"
        );
        assert!(
            !function.contains(&excluded),
            "foreign leaf identity: {function}"
        );
    }
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
        author_retained(&library, STORAGE, "storage");
        (library, supplier)
    }

    fn stage() -> Self {
        let (library, supplier) = Self::library();
        author_retained(&library, ADAPTERS, "adapters");
        let draft = unchanged_labeled(&library, "composable-adapters", "original");
        for absent in ["OwnedI64Cell", "ByteBuffer", "product-create"] {
            assert!(
                !draft.contains(absent),
                "adapter export before items: {absent}"
            );
        }
        for present in [
            "(implementation-parameter ",
            "(implementations ",
            "(borrow-from ",
        ] {
            assert!(
                draft.contains(present),
                "adapter canonical draft: {present}"
            );
        }
        let maximum = declaration(&library, "composable-adapters", "Maximum");
        let delegate = declaration(&library, "composable-adapters", "DelegatingReader");
        for implementation in [&maximum, &delegate] {
            library.cli(
                &["inspect", "owner", "owned_implementation", implementation],
                true,
            );
        }
        // Review an actual function-body edit while retaining both scheme identities.
        let original = "(in (local view))";
        assert!(draft.contains(original));
        let edit = library.input(
            "adapter-edit.lkjc",
            &draft.replacen(
                original,
                "(in (if (bool true) (local view) (local view)))",
                1,
            ),
        );
        let plan = library.plan(&edit, true);
        library.apply(&edit, &plan, true);
        library.cli(&["check"], true);
        assert_eq!(
            declaration(&library, "composable-adapters", "Maximum"),
            maximum
        );
        assert_eq!(
            declaration(&library, "composable-adapters", "DelegatingReader"),
            delegate
        );
        unchanged_labeled(&library, "composable-adapters", "edited");
        Self::downstream(library, supplier)
    }

    fn downstream(library: Native, supplier: Export) -> Self {
        // Export before even creating the independent item package.
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
        let draft = unchanged_labeled(&elements, "generic-elements", "original");
        assert!(draft.contains("(owned-product "));
        for absent in ["(owned-sequence ", "Maximum", "DelegatingReader"] {
            assert!(
                !draft.contains(absent),
                "independent element semantics: {absent}"
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
            "{}{}declarations.begin\n(units (use owned-worklists {} {}) (use owned-read-results {} {}) (use generic-storage {} {}) (use composable-adapters {} {}) (use generic-elements {} {}))\ndeclarations.end\n{body}",
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
                    "retained composable implementation evidence: {}",
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
    let tied_selected: Vec<_> = observed
        .iter()
        .enumerate()
        .max_by_key(|(index, n)| (**n % 4, std::cmp::Reverse(*index)))
        .map(|(_, n)| *n)
        .into_iter()
        .collect();
    let drained: Vec<_> = values.iter().rev().copied().collect();
    json!({"length":inputs.len(),"selected":selected,"tied-selected":tied_selected,
        "drained":drained,"empty-length":0,"reused":[reuse + offset]})
}

fn all_expected(inputs: &[i64], reuse: i64) -> Value {
    let ordinary = scenario_expected(inputs, 0, false, reuse);
    let product = scenario_expected(inputs, 12, false, reuse);
    let reverse = scenario_expected(inputs, 0, true, reuse);
    json!({"cell-flat":ordinary,"buffer-chunked":ordinary,
        "product-flat":product,"product-chunked":product,"cell-reverse":reverse,
        "cell-nested":ordinary,"cell-nested-reverse":reverse})
}

#[test]
fn native_owned_composable_implementation_prerequisites_three_packages_nested_identity_borrow_reuse_parallel_and_detach()
 {
    let packages = Packages::stage();
    let public = &packages.consumer;
    author_retained(public, &packages.source(APPLICATION), "consumer");
    let scenario = declaration(public, "composable-owned-app", "scenario");
    let main = declaration(public, "composable-owned-app", "main");
    let draft = unchanged_labeled(public, "composable-owned-app", "original");
    assert!(draft.contains("(implementations "));
    assert!(draft.contains("(borrow-call "));
    assert_nested_draft_identity(&packages, &draft);
    let artifact = public
        .root
        .path()
        .join("composable-owned-implementations.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = deployment(
        public,
        "composable-owned-implementations.lkja",
        "composable-owned-implementations",
    );
    let parallel = deployment(
        public,
        "composable-owned-implementations.lkja",
        "composable-parallel-drain",
    );
    let individual: Vec<_> = [
        ("composable-cell-nested", 0, false),
        ("composable-cell-nested-reverse", 0, true),
        ("composable-product-chunked", 12, false),
    ]
    .into_iter()
    .map(|(target, offset, reverse)| {
        (
            target,
            offset,
            reverse,
            deployment(public, "composable-owned-implementations.lkja", target),
        )
    })
    .collect();
    let mut workloads = vec![vec![], vec![2], vec![2, 7, 11, 3], vec![0, 255, 128]];
    for length in [31, 32, 33] {
        workloads.push((0..length).map(|n| n % 17).collect());
    }
    // These independent tie oracles distinguish prerequisite identity with equal signatures.
    assert_eq!(
        all_expected(&[2, 7, 11, 3], 17)["cell-nested"]["tied-selected"],
        json!([7])
    );
    assert_eq!(
        all_expected(&[2, 7, 11, 3], 17)["cell-nested-reverse"]["tied-selected"],
        json!([3])
    );
    for (case, inputs) in workloads.iter().enumerate() {
        run_expected(
            public,
            "composable-owned-implementations",
            &descriptor,
            false,
            &case.to_string(),
            json!([inputs]),
            &all_expected(inputs, 17),
        );
    }
    assert!(draft.contains("(i64 17)"));
    let edit = public.input(
        "consumer-edit.lkjc",
        &draft.replacen("(i64 17)", "(i64 19)", 1),
    );
    let plan = public.plan(&edit, true);
    public.apply(&edit, &plan, true);
    assert_eq!(declaration(public, "composable-owned-app", "main"), main);
    assert_eq!(
        declaration(public, "composable-owned-app", "scenario"),
        scenario
    );
    run_expected(
        public,
        "composable-owned-implementations",
        &descriptor,
        false,
        "edited",
        json!([[2, 7, 11, 3]]),
        &all_expected(&[2, 7, 11, 3], 19),
    );
    let edited = unchanged_labeled(public, "composable-owned-app", "edited");
    let restore = public.input(
        "consumer-restore.lkjc",
        &edited.replacen("(i64 19)", "(i64 17)", 1),
    );
    let plan = public.plan(&restore, true);
    public.apply(&restore, &plan, true);
    public.cli(&["check"], true);
    unchanged_labeled(public, "composable-owned-app", "restored");
    assert_eq!(
        declaration(public, "composable-owned-app", "scenario"),
        scenario
    );
    for detached in [false, true] {
        if detached {
            packages.detach();
        }
        for inputs in [
            &[][..],
            &[2, 7, 11, 3][..],
            &(0..33).collect::<Vec<_>>()[..],
        ] {
            let drained: Vec<_> = inputs.iter().rev().copied().collect();
            run_expected(
                public,
                "composable-parallel-drain",
                &parallel,
                true,
                &format!("sources-removed-{detached}-{}", inputs.len()),
                json!([inputs]),
                &json!({"left":drained,"right":drained}),
            );
        }
        let large: Vec<_> = (0..513).map(|n| n % 17).collect();
        for (target, offset, reverse, deployment) in &individual {
            run_expected(
                public,
                target,
                deployment,
                detached,
                "513",
                json!([large]),
                &scenario_expected(&large, *offset, *reverse, 17),
            );
        }
        if detached {
            for (case, inputs) in workloads.iter().enumerate() {
                run_expected(
                    public,
                    "composable-owned-implementations",
                    &descriptor,
                    true,
                    &case.to_string(),
                    json!([inputs]),
                    &all_expected(inputs, 17),
                );
            }
        }
    }
    packages.retain();
}

#[test]
fn native_owned_composable_implementation_prerequisites_reject_invalid_unused_maps_and_applications_without_publication()
 {
    let (library, supplier) = Packages::library();
    let before = library.revision();
    let maximum_arguments = "(implementations parameter@Maximum@implparam_8e000000000000000000000000000001\n          parameter@Maximum@implparam_8e000000000000000000000000000002)";
    for (name, from, to) in [
        ("missing-map-prerequisites", maximum_arguments, ""),
        (
            "reordered-map-prerequisites",
            maximum_arguments,
            "(implementations parameter@Maximum@implparam_8e000000000000000000000000000002 parameter@Maximum@implparam_8e000000000000000000000000000001)",
        ),
        (
            "foreign-map-scope",
            "parameter@Maximum@implparam_8e000000000000000000000000000001",
            "parameter@Drain@implparam_8e000000000000000000000000000008",
        ),
        (
            "wrong-unused-reader-type",
            "reader owned-read-results::IndexRead W (types T)",
            "reader owned-read-results::IndexRead T (types W)",
        ),
        (
            "missing-result-provenance",
            "(returns Item (borrow-from 0))",
            "(returns Item)",
        ),
        (
            "wrong-task-kind",
            "drain-values (visibility public) (effect (task))",
            "drain-values (visibility public) (effect pure)",
        ),
    ] {
        assert!(ADAPTERS.contains(from), "negative adapter fixture: {name}");
        rejected(&library, &ADAPTERS.replacen(from, to, 1), &before, name);
    }
    author_retained(&library, ADAPTERS, "healthy-adapters-after-rejections");
    unchanged_labeled(&library, "composable-adapters", "healthy");
    let packages = Packages::downstream(library, supplier);
    let public = &packages.consumer;
    let before = public.revision();
    let maximum_arguments = "(implementations parameter@scenario@implparam_8e000000000000000000000000000011\n                  parameter@scenario@implparam_8e000000000000000000000000000014)";
    for (name, from, to) in [
        ("missing-application-prerequisites", maximum_arguments, ""),
        (
            "extra-application-prerequisite",
            maximum_arguments,
            "(implementations parameter@scenario@implparam_8e000000000000000000000000000011 parameter@scenario@implparam_8e000000000000000000000000000014 parameter@scenario@implparam_8e000000000000000000000000000012)",
        ),
        (
            "reordered-application-prerequisites",
            maximum_arguments,
            "(implementations parameter@scenario@implparam_8e000000000000000000000000000014 parameter@scenario@implparam_8e000000000000000000000000000011)",
        ),
        (
            "foreign-application-scope",
            maximum_arguments,
            "(implementations parameter@composable-adapters::Maximum@implparam_8e000000000000000000000000000001 parameter@scenario@implparam_8e000000000000000000000000000014)",
        ),
        (
            "wrong-application-type",
            "composable-adapters::Maximum (types T W)",
            "composable-adapters::Maximum (types W T)",
        ),
        (
            "wrong-nested-prerequisite-type",
            "(implementations (implementation generic-storage::FlatReader (types OwnedI64Cell)))",
            "(implementations (implementation generic-storage::FlatReader (types ByteBuffer)))",
        ),
    ] {
        assert!(
            APPLICATION.contains(from),
            "negative application fixture: {name}"
        );
        rejected(
            public,
            &packages.source(&APPLICATION.replacen(from, to, 1)),
            &before,
            name,
        );
    }
    author_retained(
        public,
        &packages.source(APPLICATION),
        "healthy-consumer-after-rejections",
    );
    unchanged_labeled(public, "composable-owned-app", "healthy");
    let artifact = public.root.path().join("healthy-composable.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = deployment(
        public,
        "healthy-composable.lkja",
        "composable-owned-implementations",
    );
    run_expected(
        public,
        "composable-owned-implementations",
        &descriptor,
        false,
        "healthy",
        json!([[2, 7, 11, 3]]),
        &all_expected(&[2, 7, 11, 3], 17),
    );
    packages.retain();
}
