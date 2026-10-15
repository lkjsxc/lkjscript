//! Proposed public witness; unexecuted until complete associated admission is enabled.
//! Intended parent: tests/public_cli/native_declarations.rs.
use super::native_byte_buffer::{Export, dependency, export, stage};
use super::native_owned_worklists::{declaration, deployment, run_expected, standard, unchanged};
use super::*;

const CONSUMER: &str = include_str!("../../examples/associated-blockread/01-consumer.lkjc");
const FLAT: &str = include_str!("../../examples/associated-blockread/02-flat.lkjc");
const CHUNKED: &str = include_str!("../../examples/associated-blockread/03-chunked.lkjc");
const APPLICATION: &str = include_str!("../../examples/associated-blockread/04-application.lkjc");
const PROTECTED_CONSUME: &str =
    include_str!("../../examples/associated-blockread/negative-protected-consume.lkjc");
const ESCAPE: &str = include_str!("../../examples/associated-blockread/negative-escape.lkjc");
const PRIVATE_FIELD: &str =
    include_str!("../../examples/associated-blockread/negative-private-field.lkjc");
const MEMBER: &str = "assoc_type_c7000000000000000000000000000001";
const METHODS: [&str; 4] = [
    "method_c7000000000000000000000000000001",
    "method_c7000000000000000000000000000002",
    "method_c7000000000000000000000000000003",
    "method_c7000000000000000000000000000004",
];
const READERS: [(&str, &str); 4] = [
    (
        "forward-block",
        "implparam_c7000000000000000000000000000001",
    ),
    (
        "collect-block",
        "implparam_c7000000000000000000000000000002",
    ),
    ("scan", "implparam_c7000000000000000000000000000003"),
    (
        "scan-and-finish",
        "implparam_c7000000000000000000000000000004",
    ),
];
const TARGETS: [&str; 3] = ["blockread", "blockread-flat", "blockread-chunked"];
const DEPLOYMENT: &str =
    include_str!("../../examples/associated-blockread/blockread.deployment.json");
const CASES: [(&str, &str, &str); 4] = [
    (
        "empty",
        include_str!("../../examples/associated-blockread/empty.arguments.json"),
        include_str!("../../examples/associated-blockread/empty.expected.json"),
    ),
    (
        "one",
        include_str!("../../examples/associated-blockread/one.arguments.json"),
        include_str!("../../examples/associated-blockread/one.expected.json"),
    ),
    (
        "three",
        include_str!("../../examples/associated-blockread/three.arguments.json"),
        include_str!("../../examples/associated-blockread/three.expected.json"),
    ),
    (
        "thirty-three",
        include_str!("../../examples/associated-blockread/thirty-three.arguments.json"),
        include_str!("../../examples/associated-blockread/thirty-three.expected.json"),
    ),
];

fn native(template: &str) -> Native {
    let mut public = Native::template(template);
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        public.root.disable_cleanup(true);
        println!(
            "retained BlockRead evidence: {}",
            public.root.path().display()
        );
    }
    // Candidate and explicitly retained host runs preserve every command response,
    // including commands run internally by the reused helpers.
    if public.retains_evidence() {
        assert!(public.root.path().join("cli-evidence").is_dir());
    }
    public
}

fn archive(public: &Native, name: &str, bytes: &[u8]) {
    let evidence = public.root.path().join("literal-evidence");
    std::fs::create_dir_all(&evidence).unwrap();
    std::fs::write(evidence.join(name), bytes).unwrap();
}

fn accept(public: &Native, label: &str, body: &str) {
    let before = public.revision();
    let request = format!("request base={before}\n{body}");
    archive(public, &format!("{label}-request.lkjc"), request.as_bytes());
    let input = public.input(&format!("{label}.lkjc"), &request);
    let plan = public.plan(&input, true);
    public.apply(&input, &plan, true);
    assert_ne!(public.revision(), before);
    let records = public.cli(&["check"], true);
    let tests = compact_record(&records, "tests");
    assert_eq!(compact_field(tests, "failed"), "0");
    assert_eq!(compact_field(tests, "differential"), "equal");
}

fn draft(public: &Native, module: &str) -> String {
    let before = public.revision();
    let text = unchanged(public, module);
    assert_eq!(public.revision(), before);
    archive(public, &format!("{module}-draft.lkjc"), text.as_bytes());
    text
}

fn exported(public: &Native, label: &str) -> Export {
    let package = export(public);
    let bytes = std::fs::read(&package.path).unwrap();
    archive(
        public,
        &format!("{label}-export.json"),
        &serde_json::to_vec_pretty(&serde_json::json!({
            "package": package.package,
            "semantic_revision": package.semantic,
            "package_revision": package.revision,
            "transport": package.transport,
            "transport_file": package.path,
            "transport_file_blake3": blake3::hash(&bytes).to_hex().to_string(),
        }))
        .unwrap(),
    );
    package
}

fn imports(packages: &[(&str, &Export)]) -> String {
    let mut source = String::new();
    for (name, package) in packages {
        source.push_str(&format!(
            "declarations.begin\n(units (use {name} {} {}))\ndeclarations.end\n",
            package.package, package.revision,
        ));
    }
    source
}

fn inspect_consumer(public: &Native, text: &str) -> String {
    assert!(text.contains(&format!("(associated-type {MEMBER} Block (bound owned))")));
    assert!(text.contains("(associated-contract-member "));
    assert!(text.contains("(associated-witness-projection "));
    assert!(text.contains("(borrow-call ") && text.contains("(read-from 1)"));
    assert!(text.contains("(constraint transferable)") && text.contains("(constraint owned)"));
    for forbidden in ["ChunkBlock", "ChunkStorage", "owned-implementation"] {
        assert!(
            !text.contains(forbidden),
            "consumer before suppliers: {forbidden}"
        );
    }
    let contract = declaration(public, "blockread-consumer", "BlockRead");
    let records = public.cli(&["inspect", "owner", "owned_contract", &contract], true);
    let summary = compact_record(&records, "owned.contract");
    assert_eq!(compact_field(summary, "type-parameters"), "1");
    let methods: Vec<_> = records
        .iter()
        .filter(|record| record.operation == "owned.method")
        .collect();
    assert_eq!(methods.len(), METHODS.len());
    for (record, expected) in methods.iter().zip(METHODS) {
        assert_eq!(compact_field(record, "id"), expected);
        assert_eq!(compact_field(record, "kind"), "pure");
        assert!(text.contains(expected));
    }
    assert_eq!(compact_field(methods[1], "result-kind"), "read-from");
    assert_eq!(compact_field(methods[1], "result-source"), "1");
    assert_eq!(compact_field(methods[1], "result-scope"), "-");
    for (function, reader) in READERS {
        let identity = declaration(public, "blockread-consumer", function);
        let records = public.cli(&["inspect", "owner", "pure_function", &identity], true);
        let witnesses: Vec<_> = records
            .iter()
            .filter(|record| record.operation == "owned.implementation-parameter")
            .collect();
        assert_eq!(witnesses.len(), 1, "{function}");
        assert_eq!(compact_field(witnesses[0], "id"), reader);
        assert_eq!(compact_field(witnesses[0], "type-arguments"), "1");
        assert!(text.contains(reader) && text.contains(&identity));
    }
    contract
}

fn inspect_supplier(public: &Native, module: &str, name: &str, generic: &Export, contract: &str) {
    let text = draft(public, module);
    assert_eq!(
        text.matches(&format!("(associated-type {MEMBER} ")).count(),
        1
    );
    for method in METHODS {
        assert!(text.contains(method));
    }
    let identity = declaration(public, module, name);
    let records = public.cli(
        &["inspect", "owner", "owned_implementation", &identity],
        true,
    );
    assert_eq!(
        compact_field(compact_record(&records, "owned.implementation"), "contract"),
        format!("{}/{contract}", generic.package),
    );
    let mappings: Vec<_> = records
        .iter()
        .filter(|record| record.operation == "owned.method-implementation")
        .collect();
    assert_eq!(mappings.len(), METHODS.len());
    for (mapping, expected) in mappings.iter().zip(METHODS) {
        assert_eq!(compact_field(mapping, "method"), expected);
    }
    if module == "blockread-chunked" {
        assert!(text.contains("ChunkBlock") && text.contains("ChunkStorage"));
        assert!(text.contains("(field ") && text.contains("items") && text.contains("chunks"));
    }
}

struct Packages {
    authors: [Native; 3],
    application: Native,
    exports: [Export; 3],
    standards: Vec<PathBuf>,
    identities: Vec<(PathBuf, blake3::Hash)>,
}

impl Packages {
    fn stage() -> Self {
        // Export the generic contract/consumer before creating either supplier project.
        let consumer = native("minimal");
        let builtin = standard(&consumer);
        accept(
            &consumer,
            "consumer",
            &format!("{}{CONSUMER}", dependency(&builtin)),
        );
        let text = draft(&consumer, "blockread-consumer");
        let contract = inspect_consumer(&consumer, &text);
        let generic = exported(&consumer, "consumer");
        let mut standards = vec![builtin.path];

        let flat = native("minimal");
        let builtin = standard(&flat);
        stage(&flat, &generic);
        accept(
            &flat,
            "flat",
            &format!(
                "{}{}{}{FLAT}",
                dependency(&builtin),
                dependency(&generic),
                imports(&[("blockread-consumer", &generic)]),
            ),
        );
        inspect_supplier(&flat, "blockread-flat", "Flat", &generic, &contract);
        let flat_export = exported(&flat, "flat");
        standards.push(builtin.path);

        let chunked = native("minimal");
        let builtin = standard(&chunked);
        stage(&chunked, &generic);
        accept(
            &chunked,
            "chunked",
            &format!(
                "{}{}{}{CHUNKED}",
                dependency(&builtin),
                dependency(&generic),
                imports(&[("blockread-consumer", &generic)]),
            ),
        );
        inspect_supplier(
            &chunked,
            "blockread-chunked",
            "Chunked",
            &generic,
            &contract,
        );
        let chunked_export = exported(&chunked, "chunked");
        standards.push(builtin.path);
        assert_ne!(flat_export.package, chunked_export.package);
        assert!(
            !std::fs::read_to_string(flat.root.path().join("flat.lkjc"))
                .unwrap()
                .contains(&chunked_export.package)
        );
        assert!(
            !std::fs::read_to_string(chunked.root.path().join("chunked.lkjc"))
                .unwrap()
                .contains(&flat_export.package)
        );

        // Command template already owns its exact builtin dependency; don't add a duplicate.
        let application = native("command");
        let exports = [generic, flat_export, chunked_export];
        for package in &exports {
            stage(&application, package);
        }
        let this = Self {
            authors: [consumer, flat, chunked],
            application,
            exports,
            standards,
            identities: Vec::new(),
        };
        accept(
            &this.application,
            "application",
            &format!(
                "{}{}{APPLICATION}",
                this.exports.iter().map(dependency).collect::<String>(),
                this.imports(),
            ),
        );
        let text = draft(&this.application, "blockread-app");
        assert!(text.contains("Flat") && text.contains("Chunked"));
        assert!(text.contains("(implementations "));
        assert!(!text.contains("(field items ") && !text.contains("(field chunks "));
        this
    }

    fn imports(&self) -> String {
        imports(&[
            ("blockread-consumer", &self.exports[0]),
            ("blockread-flat", &self.exports[1]),
            ("blockread-chunked", &self.exports[2]),
        ])
    }

    fn refused(&self, label: &str, body: &str) {
        let before = self.application.revision();
        let head = std::fs::read(self.application.project.join("HEAD")).unwrap();
        let request = format!("request base={before}\n{}{body}", self.imports());
        archive(
            &self.application,
            &format!("invalid-{label}.lkjc"),
            request.as_bytes(),
        );
        let input = self
            .application
            .input(&format!("invalid-{label}.lkjc"), &request);
        let records = self.application.plan(&input, false);
        let error = compact_record(&records, "diagnostic");
        assert_ne!(
            compact_field(error, "code"),
            "owned_associated_semantics_unsupported"
        );
        assert!(matches!(
            compact_field(error, "class"),
            "semantic" | "source"
        ));
        if label == "private-field" {
            // Foreign OwnedRecord fields are excluded by reference namespace admission.
            // This pins the statically traced boundary; actual CLI confirmation is pending.
            assert_eq!(compact_field(error, "class"), "semantic");
            assert_eq!(compact_field(error, "code"), "change_reference_not_exposed");
        } else {
            // Existing worklist rejection helper uses this exact semantic-stage condition.
            assert!(compact_field(error, "code").starts_with("kernel_"));
        }
        archive(
            &self.application,
            &format!("invalid-{label}-diagnostics.txt"),
            format!("{records:#?}").as_bytes(),
        );
        assert_eq!(self.application.revision(), before);
        assert_eq!(
            std::fs::read(self.application.project.join("HEAD")).unwrap(),
            head
        );
        // A refused plan has no reviewed apply token; never fabricate one.
    }

    fn verify_identity(&self) {
        for (file, expected) in &self.identities {
            assert_eq!(blake3::hash(&std::fs::read(file).unwrap()), *expected);
        }
    }

    fn detach(&self) {
        for public in self
            .authors
            .iter()
            .chain(std::iter::once(&self.application))
        {
            std::fs::remove_dir_all(&public.project).unwrap();
            assert!(!public.project.exists());
            for entry in std::fs::read_dir(public.root.path()).unwrap() {
                let file = entry.unwrap().path();
                if file
                    .extension()
                    .is_some_and(|suffix| suffix == "lkjc" || suffix == "lkjp")
                {
                    std::fs::remove_file(file).unwrap();
                }
            }
        }
        for file in self
            .exports
            .iter()
            .map(|package| &package.path)
            .chain(self.standards.iter())
        {
            assert!(!file.exists());
        }
        self.verify_identity();
    }
}

#[test]
fn native_associated_blockread_consumer_first_four_literals_three_targets_and_detached_bytes() {
    let mut packages = Packages::stage();
    for (label, body) in [
        ("protected-consume", PROTECTED_CONSUME),
        ("escape", ESCAPE),
        ("private-field", PRIVATE_FIELD),
    ] {
        packages.refused(label, body);
    }
    for (label, arguments, expected) in CASES {
        archive(
            &packages.application,
            &format!("{label}.arguments.json"),
            arguments.as_bytes(),
        );
        archive(
            &packages.application,
            &format!("{label}.expected.json"),
            expected.as_bytes(),
        );
    }
    let artifact = packages.application.root.path().join("blockread.lkja");
    packages
        .application
        .cli(&["build", "--output", path(&artifact)], true);
    let second = packages
        .application
        .root
        .path()
        .join("blockread-second.lkja");
    packages
        .application
        .cli(&["build", "--output", path(&second)], true);
    assert_eq!(
        std::fs::read(&artifact).unwrap(),
        std::fs::read(&second).unwrap()
    );
    archive(
        &packages.application,
        "blockread.literal-deployment.json",
        DEPLOYMENT.as_bytes(),
    );
    let descriptors =
        TARGETS.map(|target| deployment(&packages.application, "blockread.lkja", target));
    let literal_deployment: Value = serde_json::from_str(DEPLOYMENT).unwrap();
    for (target, descriptor) in TARGETS.iter().zip(&descriptors) {
        let mut expected = literal_deployment.clone();
        expected["target"] = Value::String((*target).to_owned());
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(descriptor).unwrap()).unwrap(),
            expected,
        );
    }
    let mut files = vec![packages.application.executable.clone(), artifact, second];
    files.extend(
        packages
            .authors
            .iter()
            .map(|author| author.executable.clone()),
    );
    files.extend(descriptors.iter().cloned());
    packages.identities = files
        .into_iter()
        .map(|file| {
            let hash = blake3::hash(&std::fs::read(&file).unwrap());
            (file, hash)
        })
        .collect();
    for detached in [false, true] {
        if detached {
            packages.detach();
        }
        for (label, arguments, literal_expected) in CASES {
            let arguments: Value = serde_json::from_str(arguments).unwrap();
            let expected: Value = serde_json::from_str(literal_expected).unwrap();
            for (index, target) in TARGETS.iter().enumerate() {
                let result = match *target {
                    "blockread" => &expected,
                    "blockread-flat" => &expected["flat"],
                    "blockread-chunked" => &expected["chunked"],
                    _ => unreachable!("fixed three literal targets"),
                };
                run_expected(
                    &packages.application,
                    target,
                    &descriptors[index],
                    detached,
                    label,
                    arguments.clone(),
                    result,
                );
            }
        }
        packages.verify_identity();
    }
}
