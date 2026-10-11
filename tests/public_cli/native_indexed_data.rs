//! Mixed ordinary/owned contracts, independently authored storage and retained values.
use super::native_byte_buffer::{Export, dependency, export, stage};
use super::native_owned_worklists::{deployment, rejected, standard, unchanged};
use super::*;
use serde_json::json;
use std::cell::Cell;

const LIBRARY: &str = include_str!("../../examples/indexed-data-storage/library.lkjc");
const FLAT: &str = include_str!("../../examples/indexed-data-storage/flat.lkjc");
const CHUNKED: &str = include_str!("../../examples/indexed-data-storage/chunked.lkjc");
const FORWARD: &str = include_str!("../../examples/indexed-data-storage/forward.lkjc");
const APPLICATION: &str = include_str!("../../examples/indexed-data-storage/application.lkjc");
const COSTS: &str = include_str!("../../examples/indexed-data-storage/costs.lkjc");
const TARGETS: [&str; 10] = [
    "indexed-integers",
    "indexed-scalar-flat",
    "indexed-scalar-chunked",
    "indexed-booleans",
    "indexed-payloads",
    "indexed-packets",
    "indexed-captured",
    "indexed-cost-direct",
    "indexed-cost-flat",
    "indexed-cost-chunked",
];

fn native() -> Native {
    native_template("command")
}

fn native_template(template: &str) -> Native {
    let mut public = Native::template(template);
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        public.root.disable_cleanup(true);
        println!(
            "retained indexed-data evidence: {}",
            public.root.path().display()
        );
    }
    public
}

fn accept(public: &Native, label: &str, literal: &str) {
    let input = public.input(
        &format!("{label}.lkjc"),
        &format!("request base={}\n{literal}", public.revision()),
    );
    public.input(
        &format!("{label}-literal.json"),
        &json!({"request":std::fs::read_to_string(&input).unwrap()}).to_string(),
    );
    let plan = public.plan(&input, true);
    let applied = public.apply(&input, &plan, true);
    let checked = public.cli(&["check"], true);
    let tests = compact_record(&checked, "tests");
    assert_eq!(compact_field(tests, "failed"), "0");
    assert_eq!(compact_field(tests, "differential"), "equal");
    std::fs::write(
        public.root.path().join(format!("{label}-operations.log")),
        format!("plan={plan:#?}\napply={applied:#?}\ncheck={checked:#?}\n"),
    )
    .unwrap();
}

fn imports(exports: &[(&str, &Export)]) -> String {
    let mut source = String::new();
    for (module, package) in exports {
        source.push_str(&format!(
            "declarations.begin\n(units (use {module} {} {}))\ndeclarations.end\n",
            package.package, package.revision
        ));
    }
    source
}

struct Packages {
    authors: Vec<Native>,
    consumer: Native,
    exports: Vec<Export>,
    standards: Vec<PathBuf>,
    identities: Vec<(PathBuf, blake3::Hash)>,
    serial: Cell<usize>,
}

impl Packages {
    fn stage() -> Self {
        let library = native_template("minimal");
        let supplier = standard(&library);
        accept(
            &library,
            "library",
            &format!("{}{LIBRARY}", dependency(&supplier)),
        );
        let draft = unchanged(&library, "indexed-data");
        assert!(draft.contains("(constraint transferable)"));
        assert!(
            draft.contains("unrestricted") && draft.contains("consume") && draft.contains("borrow")
        );
        for forbidden in [
            "Chunk32",
            "ByteBuffer",
            "OwnedI64Cell",
            "Packet",
            "owned-implementation",
        ] {
            assert!(
                !draft.contains(forbidden),
                "generic library authored before consumers: {forbidden}"
            );
        }
        let generic = export(&library);
        let mut authors = vec![library];
        let mut exports = vec![generic];
        let mut standards = vec![supplier.path];
        for (module, literal) in [
            ("indexed-flat", FLAT),
            ("indexed-chunked", CHUNKED),
            ("indexed-forward", FORWARD),
        ] {
            let package = native_template("minimal");
            let supplier = standard(&package);
            stage(&package, &exports[0]);
            accept(
                &package,
                module,
                &format!(
                    "{}{}{}{literal}",
                    dependency(&supplier),
                    dependency(&exports[0]),
                    imports(&[("indexed-data", &exports[0])])
                ),
            );
            let draft = unchanged(&package, module);
            assert!(draft.contains("(constraint transferable)"));
            assert!(!draft.contains("OwnedI64Cell") && !draft.contains("ByteBuffer"));
            exports.push(export(&package));
            authors.push(package);
            standards.push(supplier.path);
        }
        let consumer = native();
        for package in &exports {
            stage(&consumer, package);
        }
        let mut this = Self {
            authors,
            consumer,
            exports,
            standards,
            identities: vec![],
            serial: Cell::new(0),
        };
        accept(
            &this.consumer,
            "application",
            &format!(
                "{}{}{APPLICATION}",
                this.exports.iter().map(dependency).collect::<String>(),
                this.imports()
            ),
        );
        let draft = unchanged(&this.consumer, "indexed-app");
        assert!(draft.contains("(implementations "));
        assert!(draft.contains("Forward") && draft.contains("Reverse"));
        assert!(!draft.contains("(field chunks ") && !draft.contains("(field size "));
        accept(&this.consumer, "costs", &this.source(COSTS));
        unchanged(&this.consumer, "indexed-cost");
        let artifact = this.consumer.root.path().join("indexed-data.lkja");
        this.consumer
            .cli(&["build", "--output", path(&artifact)], true);
        let second = this.consumer.root.path().join("indexed-data-second.lkja");
        this.consumer
            .cli(&["build", "--output", path(&second)], true);
        assert_eq!(
            std::fs::read(&artifact).unwrap(),
            std::fs::read(second).unwrap()
        );
        let mut paths = vec![this.consumer.executable.clone(), artifact];
        paths.extend(this.authors.iter().map(|author| author.executable.clone()));
        paths.extend(
            TARGETS
                .into_iter()
                .map(|target| deployment(&this.consumer, "indexed-data.lkja", target)),
        );
        this.identities = paths
            .into_iter()
            .map(|file| {
                let digest = blake3::hash(&std::fs::read(&file).unwrap());
                (file, digest)
            })
            .collect();
        this
    }

    fn imports(&self) -> String {
        imports(&[
            ("indexed-data", &self.exports[0]),
            ("indexed-flat", &self.exports[1]),
            ("indexed-chunked", &self.exports[2]),
            ("indexed-forward", &self.exports[3]),
        ])
    }

    fn source(&self, literal: &str) -> String {
        format!("{}{literal}", self.imports())
    }

    fn verify_identity(&self) {
        for (file, expected) in &self.identities {
            assert_eq!(&blake3::hash(&std::fs::read(file).unwrap()), expected);
        }
    }

    fn detach(&self) {
        for public in self.authors.iter().chain(std::iter::once(&self.consumer)) {
            std::fs::remove_dir_all(&public.project).unwrap();
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

    fn run(&self, target: &str, arguments: &Value, detached: bool, success: bool) -> Option<Value> {
        let serial = self.serial.get();
        self.serial.set(serial + 1);
        let input = self
            .consumer
            .input(&format!("arguments-{serial}.json"), &arguments.to_string());
        let result = self
            .consumer
            .root
            .path()
            .join(format!("result-{serial}.json"));
        let descriptor = self
            .consumer
            .root
            .path()
            .join(format!("{target}.deployment.json"));
        let mut command = if detached {
            vec!["run", "--deployment", path(&descriptor)]
        } else {
            vec!["run", target]
        };
        command.extend([
            "--arguments-file",
            path(&input),
            "--result-file",
            path(&result),
        ]);
        let records = self.consumer.cli(&command, success);
        std::fs::write(
            self.consumer
                .root
                .path()
                .join(format!("execution-{serial}.log")),
            format!("{records:#?}"),
        )
        .unwrap();
        assert_eq!(
            std::fs::read(input).unwrap(),
            arguments.to_string().as_bytes()
        );
        if success {
            let execution = compact_record(&records, "execution");
            if detached {
                joined(execution);
            } else {
                assert_eq!(compact_field(execution, "differential"), "equal");
            }
            Some(serde_json::from_slice(&std::fs::read(result).unwrap()).unwrap())
        } else {
            assert!(!result.exists());
            let diagnostic = compact_record(&records, "diagnostic");
            if detached && compact_field(diagnostic, "class") != "source" {
                failed_join(diagnostic);
            }
            None
        }
    }

    fn expected(&self, target: &str, input: &Value, expected: &Value, detached: bool) {
        self.consumer.input(
            &format!("expected-{}.json", self.serial.get()),
            &expected.to_string(),
        );
        assert_eq!(
            &self.run(target, input, detached, true).unwrap(),
            expected,
            "{target}, detached={detached}"
        );
    }
}

fn joined(execution: &CompactRecord) {
    let observation: Value =
        serde_json::from_str(compact_field(execution, "production-observation")).unwrap();
    for field in [
        "live_call_frames_after",
        "live_handles_after",
        "live_locals_after",
        "live_operands_after",
        "live_transactions_after",
        "live_type_bindings_after",
    ] {
        assert_eq!(observation[field], 0, "unreleased {field}");
    }
    let cleanup: Value = serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
    assert_eq!(cleanup["remaining_tasks"], 0);
    assert_eq!(cleanup["cleanup_failures"], json!([]));
    let executor: Value =
        serde_json::from_str(compact_field(execution, "executor-observation")).unwrap();
    assert_eq!(executor["remaining_workers"], 0);
}

fn failed_join(diagnostic: &CompactRecord) {
    let notes: Vec<String> = serde_json::from_str(compact_field(diagnostic, "notes")).unwrap();
    assert!(notes.iter().any(|note| note
        == "foreground cleanup: admission-stopped=true remaining-owned-tasks=0 failures=0"));
    assert!(notes.iter().any(|note| note == "structured worker cleanup: dispatch-stopped=true active=0 remaining-workers=0 joined-workers=0"));
}

fn exercise_expected(values: &[Value], index: usize, replacement: Value, reverse: bool) -> Value {
    let mut before = values.to_vec();
    let mut after = values.to_vec();
    let displaced = std::mem::replace(&mut after[index], replacement);
    if reverse {
        before.reverse();
        after.reverse();
    }
    json!({"before":before,"displaced":displaced,"after":after,"length":values.len()})
}

fn scalar_expected(values: &[Value], index: usize, replacement: Value) -> Value {
    let normal = exercise_expected(values, index, replacement.clone(), false);
    let reverse = exercise_expected(values, index, replacement, true);
    json!({"flat":normal,"chunked":normal,"forward-flat":normal,"forward-chunked":normal,"nested-forward":normal,"reverse":reverse})
}

fn cost_expected(values: &[i64], count: i64) -> Value {
    let mut values = values.to_vec();
    let mut checksum = 0;
    if !values.is_empty() {
        for offset in 0..count {
            let index = offset as usize % values.len();
            checksum += std::mem::replace(&mut values[index], offset) + offset;
        }
    }
    json!({"checksum":checksum,"length":values.len()})
}

#[test]
fn native_indexed_data_separate_packages_snapshots_forwarded_witnesses_and_detached_execution() {
    let packages = Packages::stage();
    for detached in [false, true] {
        if detached {
            packages.detach();
        }
        for length in [1, 31, 32, 33, 64, 65, 257, 1024] {
            let values: Vec<Value> = (0..length).map(|n| json!(n as i64 - 17)).collect();
            for index in [0, length / 2, length - 1] {
                let input = json!([values, index, i64::MAX]);
                if !detached && length > 65 {
                    // Six forwarded results cumulatively exceed the reference's fixed
                    // allocation budget. Keep complete large storage results per lane;
                    // detached production still checks the full six-result aggregate.
                    let expected = exercise_expected(&values, index, json!(i64::MAX), false);
                    for target in ["indexed-scalar-flat", "indexed-scalar-chunked"] {
                        packages.expected(target, &input, &expected, false);
                    }
                } else {
                    packages.expected(
                        "indexed-integers",
                        &input,
                        &scalar_expected(&values, index, json!(i64::MAX)),
                        detached,
                    );
                }
            }
            let booleans: Vec<_> = (0..length).map(|n| json!(n % 3 == 0)).collect();
            let expected = exercise_expected(&booleans, length - 1, json!(false), false);
            packages.expected(
                "indexed-booleans",
                &json!([booleans, length - 1, false]),
                &json!({"flat":expected,"chunked":expected}),
                detached,
            );
        }
        let payloads = vec![
            json!({"labels":["before","α"],"values":[[i64::MIN,i64::MAX],[]],"flags":[true,false]}),
            json!({"labels":[],"values":[[7]],"flags":[]}),
        ];
        let replacement = json!({"labels":["after"],"values":[[99,98]],"flags":[false]});
        let expected = exercise_expected(&payloads, 0, replacement.clone(), false);
        packages.expected(
            "indexed-payloads",
            &json!([payloads, 0, replacement]),
            &json!({"flat":expected,"chunked":expected}),
            detached,
        );
        let packets: Vec<_> = (0..33)
            .map(|n| json!({"label":format!("packet-{n}"),"values":[n,n+1]}))
            .collect();
        let replacement = json!({"label":"new","values":[-1]});
        let expected = exercise_expected(&packets, 32, replacement.clone(), false);
        packages.expected(
            "indexed-packets",
            &json!([packets, 32, replacement]),
            &json!({"flat":expected,"chunked":expected}),
            detached,
        );
        packages.expected(
            "indexed-captured",
            &json!([[i64::MIN, 0, i64::MAX], 2]),
            &json!(i64::MAX),
            detached,
        );
        for length in [0, 1, 31, 32, 33, 128, 512] {
            let input: Vec<i64> = (0..length).map(|n| n as i64 * 3 - 7).collect();
            for count in [0, length as i64, length as i64 * 4] {
                let expected = cost_expected(&input, count);
                for target in [
                    "indexed-cost-direct",
                    "indexed-cost-flat",
                    "indexed-cost-chunked",
                ] {
                    packages.expected(target, &json!([input, count]), &expected, detached);
                }
            }
        }
        packages.verify_identity();
    }
}

#[test]
fn native_indexed_data_rejects_wrong_categories_modes_scopes_and_phantom_actuals() {
    let packages = Packages::stage();
    let public = &packages.consumer;
    let before = public.revision();
    for (label, source) in [
        (
            "transferable-owner-is-not-ordinary",
            "declarations.begin\n(units (module create bad (function create ignored (visibility private) (effect pure) (returns Unit) (body (sequence (method-call (implementation indexed-flat::Flat (types ByteBuffer)) indexed-data::IndexedData method_a1000000000000000000000000000001) (unit))))))\ndeclarations.end\n",
        ),
        (
            "hidden-secret-actual",
            "declarations.begin\n(units (module create bad (function create ignored (visibility private) (effect pure) (returns Unit) (body (sequence (method-call (implementation indexed-flat::Flat (types (record (hidden (list Secret))))) indexed-data::IndexedData method_a1000000000000000000000000000001) (unit))))))\ndeclarations.end\n",
        ),
        (
            "phantom-ordinary-actual",
            "declarations.begin\n(units (module create bad (record create Phantom (visibility private) (type-parameter create T) (field create tag (type I64))) (function create ignored (visibility private) (effect pure) (returns Unit) (body (sequence (method-call (implementation indexed-flat::Flat (types (Phantom (stream I64)))) indexed-data::IndexedData method_a1000000000000000000000000000001) (unit))))))\ndeclarations.end\n",
        ),
        (
            "ordinary-self",
            "declarations.begin\n(units (module create bad (function create ignored (visibility private) (effect pure) (type-parameter create T (constraint transferable)) (implementation-parameter implparam_a1000000000000000000000000000020 storage indexed-data::IndexedData T (types I64)) (returns Unit) (body (unit)))))\ndeclarations.end\n",
        ),
        (
            "wrong-ordered-forward-arguments",
            "declarations.begin\n(units (module create bad (function create ignored (visibility private) (effect pure) (returns Unit) (body (sequence (method-call (implementation indexed-forward::Forward (types (owned-sequence I64) I64) (implementations (implementation indexed-flat::Flat (types I64)))) indexed-data::IndexedData method_a1000000000000000000000000000001) (unit))))))\ndeclarations.end\n",
        ),
        (
            "missing-forward-prerequisite",
            "declarations.begin\n(units (module create bad (function create ignored (visibility private) (effect pure) (returns Unit) (body (sequence (method-call (implementation indexed-forward::Forward (types I64 (owned-sequence I64))) indexed-data::IndexedData method_a1000000000000000000000000000001) (unit))))))\ndeclarations.end\n",
        ),
    ] {
        rejected(public, &packages.source(source), &before, label);
    }
    let contract = native();
    let before = contract.revision();
    for (label, from, to) in [
        (
            "ordinary-consume-mode",
            "(Item unrestricted) (Self consume)",
            "(Item consume) (Self consume)",
        ),
        (
            "ordinary-borrow-mode",
            "(I64 unrestricted) (Self borrow)",
            "(I64 borrow) (Self borrow)",
        ),
        (
            "ordinary-result-borrow-provenance",
            "(returns Item) (effect pure)",
            "(returns Item (borrow-from 1)) (effect pure)",
        ),
        (
            "ordinary-after-owned-prefix",
            "(Item unrestricted) (Self consume)",
            "(Self consume) (Item unrestricted)",
        ),
    ] {
        assert!(LIBRARY.contains(from));
        rejected(&contract, &LIBRARY.replacen(from, to, 1), &before, label);
    }
    stage(&contract, &packages.exports[0]);
    accept(
        &contract,
        "dependency",
        &format!(
            "{}{}",
            dependency(&packages.exports[0]),
            imports(&[("indexed-data", &packages.exports[0])])
        ),
    );
    let before = contract.revision();
    let foreign_scope = FORWARD.replacen(
        "parameter@empty@implparam_a1000000000000000000000000000011",
        "parameter@get@implparam_a1000000000000000000000000000014",
        1,
    );
    assert_ne!(foreign_scope, FORWARD);
    rejected(
        &contract,
        &format!(
            "{}{foreign_scope}",
            imports(&[("indexed-data", &packages.exports[0])])
        ),
        &before,
        "foreign-method-map-scope",
    );
    let values = vec![json!(7), json!(11), json!(13)];
    packages.expected(
        "indexed-integers",
        &json!([values, 1, 19]),
        &scalar_expected(&values, 1, json!(19)),
        false,
    );
    packages.verify_identity();
}

#[test]
fn native_indexed_data_traps_refusals_cleanup_and_recovery() {
    let packages = Packages::stage();
    let healthy = vec![json!(7), json!(9), json!(11)];
    for detached in [false, true] {
        if detached {
            packages.detach();
        }
        for index in [i64::MIN, -33, -32, -1, 3, 32, 33, i64::MAX] {
            assert!(
                packages
                    .run(
                        "indexed-integers",
                        &json!([healthy, index, 13]),
                        detached,
                        false
                    )
                    .is_none()
            );
            assert!(
                packages
                    .run(
                        "indexed-scalar-chunked",
                        &json!([healthy, index, 13]),
                        detached,
                        false
                    )
                    .is_none()
            );
            packages.expected(
                "indexed-integers",
                &json!([healthy, 1, 17]),
                &scalar_expected(&healthy, 1, json!(17)),
                detached,
            );
        }
        assert!(
            packages
                .run("indexed-integers", &json!([[], 0, 1]), detached, false)
                .is_none()
        );
        assert!(
            packages
                .run(
                    "indexed-scalar-chunked",
                    &json!([[], 0, 1]),
                    detached,
                    false
                )
                .is_none()
        );
        // Complete raw-value admission must inspect the invalid final value before an index trap.
        assert!(
            packages
                .run(
                    "indexed-integers",
                    &json!([[7, "invalid"], -1, 1]),
                    detached,
                    false
                )
                .is_none()
        );
        packages.verify_identity();
    }
    let original = packages
        .consumer
        .root
        .path()
        .join("indexed-integers.deployment.json");
    let mut descriptor: Value = serde_json::from_slice(&std::fs::read(original).unwrap()).unwrap();
    for fuel in [100, 4096] {
        descriptor["execution"] = json!({"instruction_fuel":fuel,"maximum_call_depth":4096,"maximum_value_stack":1000000});
        packages
            .consumer
            .input("limited.deployment.json", &descriptor.to_string());
        assert!(
            packages
                .run(
                    "limited",
                    &json!([(0..257).collect::<Vec<_>>(), 32, 99]),
                    true,
                    false
                )
                .is_none()
        );
        packages.expected(
            "indexed-integers",
            &json!([healthy, 1, 17]),
            &scalar_expected(&healthy, 1, json!(17)),
            true,
        );
    }
    packages.verify_identity();
}
