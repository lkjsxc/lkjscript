//! Literal nominal ownership through exact exports, private representations and detached execution.
use super::native_byte_buffer::{Export, dependency, export, stage};
use super::native_owned_worklists::{declaration, deployment};
use super::*;
use serde_json::json;
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

const LIBRARY: &str = include_str!("../../../examples/owned-nominals/library.lkjc");
const APPLICATION: &str = include_str!("../../../examples/owned-nominals/application.lkjc");
const TASKS: &str = include_str!("../../../examples/owned-nominals/tasks.lkjc");
const OPAQUE_LEAF: &str = include_str!("../../../examples/owned-nominals/opaque-leaf.lkjc");
const OPAQUE_WRAPPER: &str = include_str!("../../../examples/owned-nominals/opaque-wrapper.lkjc");
const OPAQUE_CONSUMER: &str = include_str!("../../../examples/owned-nominals/opaque-consumer.lkjc");
const TARGETS: [&str; 5] = [
    "nominal-main",
    "nominal-data",
    "nominal-transfer",
    "nominal-trap",
    "nominal-fuel",
];
static NEXT_DRAFT: AtomicUsize = AtomicUsize::new(0);

fn native() -> Native {
    let mut public = Native::template("command");
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        public.root.disable_cleanup(true);
        println!(
            "retained nominal public evidence: {}",
            public.root.path().display()
        );
    }
    public
}

fn accept(public: &Native, label: &str, source: &str) {
    let input = public.input(
        &format!("{label}.lkjc"),
        &format!("request base={}\n{source}", public.revision()),
    );
    public.apply(&input, &public.plan(&input, true), true);
    let checked = public.cli(&["check"], true);
    let tests = compact_record(&checked, "tests");
    assert_eq!(compact_field(tests, "failed"), "0");
    assert_eq!(compact_field(tests, "differential"), "equal");
}

fn unchanged(public: &Native, module: &str) -> String {
    let serial = NEXT_DRAFT.fetch_add(1, Ordering::Relaxed);
    let draft = public
        .root
        .path()
        .join(format!("{module}-{serial}-draft.lkjc"));
    assert!(!draft.exists());
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

fn reject(public: &Native, label: &str, source: &str) -> Vec<CompactRecord> {
    let before = public.revision();
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let input = public.input(
        &format!("rejected-{label}.lkjc"),
        &format!("request base={before}\n{source}"),
    );
    let records = public.plan(&input, false);
    let diagnostic = compact_record(&records, "diagnostic");
    assert!(
        matches!(compact_field(diagnostic, "class"), "semantic" | "source"),
        "{label}: {records:?}"
    );
    assert_eq!(
        public.revision(),
        before,
        "rejected {label} changed meaning"
    );
    assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    records
}

struct Nominals {
    supplier: Native,
    consumer: Native,
    generic: Export,
    identities: Vec<(PathBuf, blake3::Hash)>,
    serial: Cell<usize>,
}

impl Nominals {
    fn new() -> Self {
        let supplier = native();
        accept(&supplier, "library", LIBRARY);
        let draft = unchanged(&supplier, "owned-nominals");
        for syntax in [
            "owned-record",
            "owned-contract",
            "owned-implementation",
            "(self ",
            "(constraint owned)",
            "(constraint transferable)",
            "read-from",
            "borrow-owned-field",
        ] {
            assert!(draft.contains(syntax), "generic supplier: {syntax}");
        }
        for concrete in [
            "OwnedI64Cell",
            "ByteBuffer",
            "nominal-consumer",
            "nominal-tasks",
        ] {
            assert!(
                !draft.contains(concrete),
                "supplier depends on future consumer: {concrete}"
            );
        }
        // Export the complete abstract supplier before creating either concrete consumer module.
        let mut generic = export(&supplier);
        let retained = supplier.root.path().join("supplier-first.lkjp");
        std::fs::rename(&generic.path, &retained).unwrap();
        generic.path = retained;
        let consumer = native();
        stage(&consumer, &generic);
        let imports = format!(
            "declarations.begin\n(units (use nominal {} {}))\ndeclarations.end\n",
            generic.package, generic.revision
        );
        accept(
            &consumer,
            "application",
            &format!("{}{imports}{APPLICATION}", dependency(&generic)),
        );
        accept(&consumer, "tasks", &format!("{imports}{TASKS}"));
        let concrete = unchanged(&consumer, "nominal-consumer");
        assert!(concrete.contains("(implementation-call "));
        assert!(concrete.contains("(implementation "));
        unchanged(&consumer, "nominal-tasks");
        let artifact = consumer.root.path().join("owned-nominals.lkja");
        consumer.cli(&["build", "--output", path(&artifact)], true);
        let mut files = vec![
            supplier.executable.clone(),
            consumer.executable.clone(),
            artifact,
        ];
        for target in TARGETS {
            files.push(deployment(&consumer, "owned-nominals.lkja", target));
        }
        let identities: Vec<_> = files
            .into_iter()
            .map(|file| {
                let digest = blake3::hash(&std::fs::read(&file).unwrap());
                (file, digest)
            })
            .collect();
        assert_eq!(identities[0].1, identities[1].1, "one exact copied product");
        Self {
            supplier,
            consumer,
            generic,
            identities,
            serial: Cell::new(0),
        }
    }

    fn imports(&self) -> String {
        format!(
            "declarations.begin\n(units (use nominal {} {}))\ndeclarations.end\n",
            self.generic.package, self.generic.revision
        )
    }

    fn verify_identity(&self) {
        for (file, digest) in &self.identities {
            assert_eq!(
                blake3::hash(&std::fs::read(file).unwrap()),
                *digest,
                "{}",
                file.display()
            );
        }
    }

    fn detach(&self) {
        for public in [&self.supplier, &self.consumer] {
            std::fs::remove_dir_all(&public.project).unwrap();
            for entry in std::fs::read_dir(public.root.path()).unwrap() {
                let file = entry.unwrap().path();
                if file
                    .extension()
                    .is_some_and(|suffix| suffix == "lkjc" || suffix == "lkjp")
                {
                    assert!(file.is_file());
                    std::fs::remove_file(file).unwrap();
                }
            }
            assert!(!public.project.exists());
        }
        assert!(!self.generic.path.exists());
        self.verify_identity();
    }

    fn run(
        &self,
        target: &str,
        input: &Value,
        detached: bool,
        success: bool,
    ) -> (Vec<CompactRecord>, Option<Value>) {
        let serial = self.serial.get();
        self.serial.set(serial + 1);
        let arguments = self
            .consumer
            .input(&format!("arguments-{serial}.json"), &input.to_string());
        let result = self
            .consumer
            .root
            .path()
            .join(format!("result-{serial}.json"));
        assert!(!result.exists());
        let descriptor = self
            .consumer
            .root
            .path()
            .join(format!("{target}.deployment.json"));
        let production = detached || target == "nominal-transfer";
        let mut command = if production {
            vec!["run", "--deployment", path(&descriptor)]
        } else {
            vec!["run", target]
        };
        command.extend([
            "--arguments-file",
            path(&arguments),
            "--result-file",
            path(&result),
        ]);
        let records = self.consumer.cli(&command, success);
        std::fs::write(
            self.consumer
                .root
                .path()
                .join(format!("execution-{serial}.log")),
            format!("target={target} detached={detached} success={success}\n{records:#?}\n"),
        )
        .unwrap();
        assert_eq!(
            std::fs::read(&arguments).unwrap(),
            input.to_string().as_bytes()
        );
        let value = if success {
            let execution = compact_record(&records, "execution");
            if production {
                joined(execution);
            } else {
                assert_eq!(compact_field(execution, "differential"), "equal");
            }
            Some(serde_json::from_slice(&std::fs::read(&result).unwrap()).unwrap())
        } else {
            assert!(!result.exists(), "failure published a partial result");
            None
        };
        (records, value)
    }

    fn expected(
        &self,
        target: &str,
        input: &Value,
        expected: &Value,
        detached: bool,
    ) -> Vec<CompactRecord> {
        let (records, actual) = self.run(target, input, detached, true);
        assert_eq!(
            actual.as_ref(),
            Some(expected),
            "{target}, detached={detached}"
        );
        records
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
    assert_eq!(observation["capability_calls"], 0);
    let cleanup: Value = serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
    assert_eq!(cleanup["admission_stopped"], true);
    assert_eq!(cleanup["remaining_tasks"], 0);
    assert_eq!(cleanup["cleanup_failures"], json!([]));
    let executor: Value =
        serde_json::from_str(compact_field(execution, "executor-observation")).unwrap();
    assert_eq!(executor["dispatch_open"], false);
    assert_eq!(executor["active_dispatches"], 0);
    assert_eq!(executor["remaining_workers"], 0);
}

fn failed_join(diagnostic: &CompactRecord) {
    let notes: Vec<String> = serde_json::from_str(compact_field(diagnostic, "notes")).unwrap();
    assert!(notes.iter().any(|note| note
        == "foreground cleanup: admission-stopped=true remaining-owned-tasks=0 failures=0"));
    assert!(notes.iter().any(|note| note == "structured worker cleanup: dispatch-stopped=true active=0 remaining-workers=0 joined-workers=0"));
}

fn snapshot(n: i64) -> Value {
    json!({"stamp":n,"direct":n,"nested":n,"payload":n})
}

fn opaque_three_package_execution() {
    let leaf = native();
    accept(&leaf, "opaque-leaf", OPAQUE_LEAF);
    unchanged(&leaf, "opaque-leaf");
    let c = export(&leaf);
    let leaf_id = declaration(&leaf, "opaque-leaf", "Leaf");
    let wrapper = native();
    stage(&wrapper, &c);
    accept(
        &wrapper,
        "opaque-wrapper",
        &format!(
            "{}declarations.begin\n(units (use leaf {} {}))\ndeclarations.end\n{OPAQUE_WRAPPER}",
            dependency(&c),
            c.package,
            c.revision
        ),
    );
    unchanged(&wrapper, "opaque-wrapper");
    let b = export(&wrapper);
    let consumer = native();
    // B's complete transport carries its admitted C representation. A stages and
    // selects only B; transitive layout evidence does not grant A a C import.
    stage(&consumer, &b);
    let imports = format!(
        "declarations.begin\n(units (use wrapper {} {}))\ndeclarations.end\n",
        b.package, b.revision
    );
    accept(
        &consumer,
        "opaque-consumer",
        &format!("{}{imports}{OPAQUE_CONSUMER}", dependency(&b)),
    );
    let draft = unchanged(&consumer, "opaque-consumer");
    assert!(
        !draft.contains(&c.package),
        "opaque representation granted direct authored C visibility"
    );
    assert!(!draft.contains("Leaf") && !draft.contains("hidden"));
    let rejected = reject(
        &consumer,
        "transitive-package-is-not-direct-authority",
        &format!(
            r#"declarations.begin
(units (reference forbidden declaration {}/{leaf_id})
  (module create forbidden-direct-c
    (function create unused (visibility private) (effect pure)
      (parameter create value (type forbidden) (use consume)) (returns Unit) (body (unit)))))
declarations.end
"#,
            c.package
        ),
    );
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        "change_validate_type_reference"
    );
    let rejected = reject(
        &consumer,
        "transitive-package-is-not-named-import-authority",
        &format!(
            r#"declarations.begin
(units (use forbidden {} {})
  (module create forbidden-direct-c
    (function create unused (visibility private) (effect pure)
      (parameter create value (type forbidden::Leaf) (use consume)) (returns Unit) (body (unit)))))
declarations.end
"#,
            c.package, c.revision
        ),
    );
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        "change_reference_dependency_missing"
    );
    reject(
        &consumer,
        "transitive-provider-fields-stay-private",
        &format!(
            "{imports}{}",
            r#"declarations.begin
(units (module create forbidden-wrapper-field
  (function create read (visibility private) (effect pure)
    (parameter create packet (type wrapper::Envelope) (use borrow)) (returns I64)
    (body (field (local packet) wrapper::Envelope::hidden)))))
declarations.end
"#
        ),
    );
    let artifact = consumer.root.path().join("opaque.lkja");
    consumer.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = deployment(&consumer, "opaque.lkja", "nominal-opaque");
    let identities: Vec<_> = [
        leaf.executable.clone(),
        wrapper.executable.clone(),
        consumer.executable.clone(),
        artifact,
        descriptor.clone(),
    ]
    .into_iter()
    .map(|file| {
        let digest = blake3::hash(&std::fs::read(&file).unwrap());
        (file, digest)
    })
    .collect();
    assert_eq!(identities[0].1, identities[1].1);
    assert_eq!(identities[1].1, identities[2].1);
    for detached in [false, true] {
        if detached {
            for public in [&leaf, &wrapper, &consumer] {
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
                assert!(!public.project.exists());
            }
            assert!(!b.path.exists() && !c.path.exists());
        }
        for n in [-7, 42] {
            let arguments = consumer.input(
                &format!("opaque-{detached}-{n}-arguments.json"),
                &json!([n]).to_string(),
            );
            let result = consumer
                .root
                .path()
                .join(format!("opaque-{detached}-{n}-result.json"));
            assert!(!result.exists());
            let mut command = if detached {
                vec!["run", "--deployment", path(&descriptor)]
            } else {
                vec!["run", "nominal-opaque"]
            };
            command.extend([
                "--arguments-file",
                path(&arguments),
                "--result-file",
                path(&result),
            ]);
            let records = consumer.cli(&command, true);
            std::fs::write(
                consumer
                    .root
                    .path()
                    .join(format!("opaque-{detached}-{n}-execution.log")),
                format!("target=nominal-opaque detached={detached}\n{records:#?}\n"),
            )
            .unwrap();
            assert_eq!(
                std::fs::read(&arguments).unwrap(),
                json!([n]).to_string().as_bytes()
            );
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(result).unwrap()).unwrap(),
                json!({"before":n,"after":n})
            );
            let execution = compact_record(&records, "execution");
            if detached {
                joined(execution);
            } else {
                assert_eq!(compact_field(execution, "differential"), "equal");
            }
        }
        for (file, digest) in &identities {
            assert_eq!(blake3::hash(&std::fs::read(file).unwrap()), *digest);
        }
    }
}

// Textual edits of a canonical draft preserve every supplied identity. The product
// resolves, admits and compiles the changed private layout and its native bodies.
fn wrap_private_metadata(public: &Native, package: &str) {
    let mut draft = unchanged(public, "owned-nominals");
    let packet = declaration(public, "owned-nominals", "Packet");
    let record = form_span(&draft, &format!("(owned-record edit {packet} Packet"));
    let heading = draft[record]
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("(field edit ") && line.ends_with(" stamp"))
        .unwrap()
        .to_owned();
    let field = heading.split_whitespace().nth(2).unwrap();
    let exact = format!("{package}/{field})");
    let span = form_span(&draft, &heading);
    let metadata = draft[span.clone()].replacen("(type I64)", "(type (record (value I64)))", 1);
    assert_ne!(&draft[span.clone()], metadata);
    draft.replace_range(span, &metadata);
    let stamp = draft
        .lines()
        .find_map(|line| {
            let words: Vec<_> = line.split_whitespace().collect();
            (words.first() == Some(&"(reference")
                && words.get(2) == Some(&"field")
                && words
                    .get(3)
                    .is_some_and(|reference| *reference == exact.as_str()))
            .then(|| words[1].to_owned())
        })
        .unwrap();
    for name in ["make", "stamp", "take"] {
        let id = declaration(public, "owned-nominals", name);
        let span = form_span(&draft, &format!("(function edit {id} {name}"));
        let mut function = draft[span.clone()].to_owned();
        match name {
            "make" => {
                let field = form_span(&function, &format!("(field {stamp} "));
                let old = &function[field.clone()];
                let value = &old[format!("(field {stamp} ").len()..old.len() - 1];
                let changed = format!("(field {stamp} (record structural (field value {value})))");
                function.replace_range(field, &changed);
            }
            "stamp" => {
                let body = form_span(&function, "(body ");
                let old = &function[body.clone()];
                let expression = &old[6..old.len() - 1];
                let changed = format!("(body (field {expression} (name value)))");
                function.replace_range(body, &changed);
            }
            "take" => {
                let field = form_span(&function, &format!("(field {stamp} "));
                let changed = function[field.clone()].replacen(
                    "(type I64)",
                    "(type (record (value I64)))",
                    1,
                );
                function.replace_range(field, &changed);
            }
            _ => unreachable!(),
        }
        draft.replace_range(span, &function);
    }
    let input = public.input("private-metadata-evolution.lkjc", &draft);
    public.apply(&input, &public.plan(&input, true), true);
    public.cli(&["check"], true);
}

#[test]
fn native_owned_nominals_export_generic_identity_and_detached_results() {
    let mut nominal = Nominals::new();
    let packet = declaration(&nominal.supplier, "owned-nominals", "Packet");
    let twin = declaration(&nominal.supplier, "owned-nominals", "Twin");
    let reader = declaration(&nominal.supplier, "owned-nominals", "Reader");
    let scheme = declaration(&nominal.supplier, "owned-nominals", "PacketReader");
    assert_ne!(
        packet, twin,
        "matching layouts retain distinct nominal identities"
    );
    reject(
        &nominal.consumer,
        "same-layout-is-not-identity",
        &format!(
            "{}{}",
            nominal.imports(),
            r#"declarations.begin
(units (module create identity-rejected
  (function create bad (visibility private) (effect pure)
    (parameter create value (type (nominal::Packet OwnedI64Cell)) (use consume))
    (returns (nominal::Twin OwnedI64Cell)) (body (local value)))))
declarations.end
"#
        ),
    );
    reject(
        &nominal.consumer,
        "same-shaped-self-is-not-reader-witness",
        &format!(
            "{}{}",
            nominal.imports(),
            r#"declarations.begin
(units (module create identity-rejected
  (function create bad (visibility private) (effect pure)
    (parameter create value (type (nominal::Twin OwnedI64Cell)) (use borrow)) (returns I64)
    (body (implementation-call nominal::observe (types (nominal::Twin OwnedI64Cell))
      (implementations (implementation nominal::PacketReader (types OwnedI64Cell)))
      (local value))))))
declarations.end
"#
        ),
    );
    let literal = std::fs::read(nominal.consumer.root.path().join("application.lkjc")).unwrap();
    let downstream = unchanged(&nominal.consumer, "nominal-consumer");
    for n in [i64::MIN, -257, 0, 42, i64::MAX] {
        nominal.expected("nominal-main", &json!([n]), &snapshot(n), false);
    }
    // Change an ordinary private metadata field from I64 to a record wrapper.
    // Public factory/accessor contracts and exact declaration identities are preserved.
    wrap_private_metadata(&nominal.supplier, &nominal.generic.package);
    assert_eq!(
        declaration(&nominal.supplier, "owned-nominals", "Packet"),
        packet
    );
    assert_eq!(
        declaration(&nominal.supplier, "owned-nominals", "Reader"),
        reader
    );
    assert_eq!(
        declaration(&nominal.supplier, "owned-nominals", "PacketReader"),
        scheme
    );
    let updated = export(&nominal.supplier);
    assert_eq!(updated.package, nominal.generic.package);
    assert_ne!(updated.revision, nominal.generic.revision);
    stage(&nominal.consumer, &updated);
    accept(
        &nominal.consumer,
        "explicit-dependency-update",
        &format!(
            "replace.dependency package={} semantic-revision={} package-revision={}\n",
            updated.package, updated.semantic, updated.revision
        ),
    );
    assert_eq!(
        std::fs::read(nominal.consumer.root.path().join("application.lkjc")).unwrap(),
        literal,
        "downstream literal input changed during representation evolution"
    );
    let after = unchanged(&nominal.consumer, "nominal-consumer");
    assert_eq!(
        downstream.lines().skip(1).collect::<Vec<_>>(),
        after.lines().skip(1).collect::<Vec<_>>(),
        "private representation update changed downstream authored meaning"
    );
    let evolved = nominal.consumer.root.path().join("evolved.lkja");
    nominal
        .consumer
        .cli(&["build", "--output", path(&evolved)], true);
    let original = deployment(&nominal.consumer, "evolved.lkja", "nominal-main");
    // Preserve the descriptor bound to the original artifact as well as the evolved one.
    let evolved_descriptor = nominal.consumer.root.path().join("evolved.deployment.json");
    std::fs::rename(original, &evolved_descriptor).unwrap();
    deployment(&nominal.consumer, "owned-nominals.lkja", "nominal-main");
    for file in [evolved, evolved_descriptor] {
        let digest = blake3::hash(&std::fs::read(&file).unwrap());
        nominal.identities.push((file, digest));
    }
    for n in [i64::MIN, 0, 42, i64::MAX] {
        nominal.expected("nominal-main", &json!([n]), &snapshot(n), false);
    }
    nominal.detach();
    for n in [i64::MIN, -257, 0, 42, i64::MAX] {
        for target in ["nominal-main", "evolved"] {
            nominal.expected(target, &json!([n]), &snapshot(n), true);
        }
    }
    let value = json!({"name":"detached λ","values":[i64::MIN,0,i64::MAX]});
    nominal.expected(
        "nominal-data",
        &json!([value]),
        &json!({"before":value,"after":value}),
        true,
    );
    nominal.verify_identity();
    opaque_three_package_execution();
}

#[test]
fn native_owned_nominals_keep_representation_private() {
    let nominal = Nominals::new();
    let packet = declaration(&nominal.supplier, "owned-nominals", "Packet");
    let fields = nominal.supplier.cli(
        &["query", "find", "field", "stamp", "--parent", &packet],
        true,
    );
    let field = compact_field(compact_record(&fields, "owner"), "id");
    for (label, selector) in [
        ("named", "owned-nominals::Packet::stamp".to_owned()),
        ("guessed-exact", "guessed".to_owned()),
    ] {
        let reference = format!(
            "(reference guessed field {}/{field})",
            nominal.generic.package
        );
        reject(
            &nominal.supplier,
            &format!("same-package-other-module-{label}"),
            &format!(
                r#"declarations.begin
(units {reference} (module create forbidden
  (function create read (visibility private) (effect pure)
    (parameter create packet (type (owned-nominals::Packet OwnedI64Cell)) (use borrow))
    (returns I64) (body (field (local packet) {selector})))))
declarations.end
"#
            ),
        );
    }
    for (label, body) in [
        ("metadata", "(field (local packet) nominal::Packet::stamp)"),
        ("structural-metadata", "(field (local packet) (name stamp))"),
        (
            "owned-borrow",
            "(borrow-owned-field (type (nominal::Packet OwnedI64Cell)) (local packet) (field value (binding view (type OwnedI64Cell))) (in (i64 0)))",
        ),
        (
            "unpack",
            "(unpack-owned (type (nominal::Packet OwnedI64Cell)) (local packet) (field value (binding value (type OwnedI64Cell))) (field stamp (binding stamp (type I64))) (in (local stamp)))",
        ),
    ] {
        let mode = if label == "unpack" {
            "consume"
        } else {
            "borrow"
        };
        reject(
            &nominal.consumer,
            &format!("foreign-{label}"),
            &format!(
                "{}declarations.begin\n(units (module create forbidden\n(function create bad (visibility private) (effect pure)\n(parameter create packet (type (nominal::Packet OwnedI64Cell)) (use {mode}))\n(returns I64) (body {body}))))\ndeclarations.end\n",
                nominal.imports()
            ),
        );
    }
    reject(
        &nominal.consumer,
        "foreign-pack",
        &format!(
            "{}{}",
            nominal.imports(),
            r#"declarations.begin
(units (module create forbidden
  (function create bad (visibility private) (effect pure)
    (parameter create value (type OwnedI64Cell) (use consume)) (returns (nominal::Packet OwnedI64Cell))
    (body (pack-owned (type (nominal::Packet OwnedI64Cell))
      (field value (local value)) (field stamp (i64 7)))))))
declarations.end
"#
        ),
    );
    let reference = format!(
        "(reference guessed field {}/{field})",
        nominal.generic.package
    );
    let records = reject(
        &nominal.consumer,
        "foreign-guessed-exact-field",
        &format!(
            "{}declarations.begin\n(units {reference} (module create forbidden\n(function create bad (visibility private) (effect pure)\n(parameter create packet (type (nominal::Packet OwnedI64Cell)) (use borrow))\n(returns I64) (body (field (local packet) guessed)))))\ndeclarations.end\n",
            nominal.imports()
        ),
    );
    assert_eq!(
        compact_field(compact_record(&records, "diagnostic"), "code"),
        "kernel_owned_record"
    );
    nominal.expected("nominal-main", &json!([7]), &snapshot(7), false);
    nominal.verify_identity();
}

#[test]
fn native_owned_nominals_nested_borrows_preserve_source_until_consumption() {
    let nominal = Nominals::new();
    let supplier = unchanged(&nominal.supplier, "owned-nominals");
    assert!(supplier.contains("(returns ") && supplier.contains("(read-from "));
    for (label, source) in [
        (
            "consume-parent-during-child-loan",
            r#"declarations.begin
(units (module create forbidden
  (function create bad (visibility private) (effect pure)
    (parameter create envelope (type (nominal::Envelope OwnedI64Cell)) (use consume)) (returns Unit)
    (body (borrow-call (call nominal::nested-view (types OwnedI64Cell) (local envelope))
      (binding child (type OwnedI64Cell))
      (in (sequence (call nominal::unwrap (types OwnedI64Cell) (local envelope)) (unit))))))))
declarations.end
"#,
        ),
        (
            "wrong-borrow-origin",
            r#"declarations.begin
(units (module create forbidden
  (function create bad (visibility private) (effect pure)
    (parameter create first (type (nominal::Envelope OwnedI64Cell)) (use borrow))
    (parameter create second (type (nominal::Envelope OwnedI64Cell)) (use borrow))
    (returns OwnedI64Cell (borrow-from first))
    (body (borrow-call (call nominal::nested-view (types OwnedI64Cell) (local second))
      (binding child (type OwnedI64Cell)) (in (local child)))))))
declarations.end
"#,
        ),
        (
            "borrowed-child-escapes-as-owner",
            r#"declarations.begin
(units (module create forbidden
  (function create bad (visibility private) (effect pure)
    (parameter create envelope (type (nominal::Envelope OwnedI64Cell)) (use borrow))
    (returns OwnedI64Cell)
    (body (borrow-call (call nominal::nested-view (types OwnedI64Cell) (local envelope))
      (binding child (type OwnedI64Cell)) (in (local child)))))))
declarations.end
"#,
        ),
    ] {
        reject(
            &nominal.consumer,
            label,
            &format!("{}{source}", nominal.imports()),
        );
    }
    for detached in [false, true] {
        if detached {
            nominal.detach();
        }
        for n in [i64::MIN, -1, 0, 7, i64::MAX] {
            nominal.expected("nominal-main", &json!([n]), &snapshot(n), detached);
        }
        nominal.verify_identity();
    }
}

#[test]
fn native_owned_nominals_admit_complete_layouts_and_phantom_arguments() {
    let nominal = Nominals::new();
    let invalid = [
        (
            "unconstrained-phantom",
            "(owned-record create Bad (visibility private) (type-parameter create T))",
        ),
        (
            "capture-only-phantom",
            "(owned-record create Bad (visibility private) (type-parameter create T (constraint capture-safe)))",
        ),
        (
            "secret-field",
            "(owned-record create Bad (visibility private) (field create hidden (type Secret)))",
        ),
        (
            "hidden-secret-field",
            "(owned-record create Bad (visibility private) (field create hidden (type (record (visible I64) (hidden (list Secret))))))",
        ),
        (
            "callable-field",
            "(owned-record create Bad (visibility private) (field create hidden (type (function () I64))))",
        ),
        (
            "stream-field",
            "(owned-record create Bad (visibility private) (field create hidden (type (stream I64))))",
        ),
        (
            "direct-recursion",
            "(owned-record create Bad (visibility private) (field create next (type Bad)))",
        ),
        (
            "sequence-recursion",
            "(owned-record create Bad (visibility private) (field create next (type (owned-sequence Bad))))",
        ),
        (
            "mutual-recursion",
            "(owned-record create Bad (visibility private) (field create next (type Other))) (owned-record create Other (visibility private) (field create next (type Bad)))",
        ),
        (
            "mutual-sequence-recursion",
            "(owned-record create Bad (visibility private) (field create next (type (owned-sequence Other)))) (owned-record create Other (visibility private) (field create next (type (owned-sequence Bad))))",
        ),
        (
            "generic-recursion",
            "(owned-record create Bad (visibility private) (type-parameter create T (constraint owned)) (field create next (type (owned-sequence (Bad T)))))",
        ),
    ];
    for (label, declaration) in invalid {
        reject(
            &nominal.consumer,
            label,
            &format!(
                "declarations.begin\n(units (module create invalid-layout {declaration}))\ndeclarations.end\n"
            ),
        );
    }
    for (label, ty) in [
        (
            "owned-phantom-secret",
            "(nominal::Phantom (owned-sequence Secret))",
        ),
        (
            "owned-phantom-hidden-secret",
            "(nominal::Phantom (owned-sequence (record (hidden (list Secret)))))",
        ),
        (
            "ordinary-phantom-callable",
            "(nominal::Marker (function () I64))",
        ),
        ("ordinary-phantom-stream", "(nominal::Marker (stream I64))"),
        ("owned-in-ordinary-container", "(list nominal::Empty)"),
        ("missing-argument", "nominal::Packet"),
        ("extra-argument", "(nominal::Packet OwnedI64Cell I64)"),
    ] {
        reject(
            &nominal.consumer,
            label,
            &format!(
                "{}declarations.begin\n(units (module create invalid-unused\n(function create unused (visibility private) (effect pure)\n(parameter create ignored (type {ty}) (use consume)) (returns Unit) (body (unit)))))\ndeclarations.end\n",
                nominal.imports()
            ),
        );
    }
    reject(
        &nominal.consumer,
        "unused-call-substitution",
        &format!(
            "{}{}",
            nominal.imports(),
            r#"declarations.begin
(units (module create invalid-unused
  (function create ignore (visibility private) (effect pure)
    (type-parameter create T (constraint owned)) (returns Unit) (body (unit)))
  (function create bad (visibility private) (effect pure) (returns Unit)
    (body (call ignore (types (nominal::Phantom (owned-sequence Secret))))))))
declarations.end
"#
        ),
    );
    reject(
        &nominal.consumer,
        "untaken-inadmissible-phantom",
        &format!(
            "{}{}",
            nominal.imports(),
            r#"declarations.begin
(units (module create invalid-untaken
  (function create bad (visibility private) (effect pure) (returns Unit)
    (body (if (bool false)
      (sequence (call nominal::phantom (types (owned-sequence Secret))) (unit))
      (unit))))))
declarations.end
"#
        ),
    );
    for (label, ty, make, finish) in [
        (
            "empty-is-affine",
            "nominal::Empty",
            "(call nominal::empty)",
            "(call nominal::finish-empty (local value))",
        ),
        (
            "data-only-is-affine",
            "(nominal::Data I64)",
            "(call nominal::data (types I64) (i64 7))",
            "(call nominal::finish-data (types I64) (local value))",
        ),
    ] {
        reject(
            &nominal.consumer,
            label,
            &format!(
                "{}declarations.begin\n(units (module create invalid-affinity\n(function create bad (visibility private) (effect pure) (returns Unit)\n(body (let (binding value (type {ty}) {make})\n(in (sequence {finish} {finish} (unit))))))))\ndeclarations.end\n",
                nominal.imports()
            ),
        );
    }
    let value = json!({"name":"ordinary first-order payload","values":[-7,0,42]});
    for detached in [false, true] {
        if detached {
            nominal.detach();
        }
        nominal.expected(
            "nominal-data",
            &json!([value]),
            &json!({"before":value,"after":value}),
            detached,
        );
        nominal.expected("nominal-main", &json!([42]), &snapshot(42), detached);
        nominal.verify_identity();
    }
}

#[test]
fn native_owned_nominals_transfer_traps_budget_join_cleanup_and_recovery() {
    let nominal = Nominals::new();
    let descriptor = nominal
        .consumer
        .root
        .path()
        .join("nominal-fuel.deployment.json");
    let mut limited: Value = serde_json::from_slice(&std::fs::read(descriptor).unwrap()).unwrap();
    for detached in [false, true] {
        if detached {
            nominal.detach();
        }
        for n in [i64::MIN, 0, 42, i64::MAX] {
            let records = nominal.expected(
                "nominal-transfer",
                &json!([n]),
                &json!({"stamp":n,"payload":n,"side":n}),
                detached,
            );
            let observation: Value = serde_json::from_str(compact_field(
                compact_record(&records, "execution"),
                "production-observation",
            ))
            .unwrap();
            assert_eq!(observation["parallel_scopes"], 1);
            assert!(observation["parallel_worker_dispatches"].as_u64().unwrap() > 0);
        }
        let (records, missing) = nominal.run("nominal-trap", &json!([[7, 9]]), detached, false);
        assert!(missing.is_none());
        let diagnostic = compact_record(&records, "diagnostic");
        assert_eq!(compact_field(diagnostic, "class"), "semantic");
        assert_eq!(compact_field(diagnostic, "code"), "normalized_list_index");
        if detached {
            failed_join(diagnostic);
        }
        let (records, _) = nominal.run(
            "nominal-trap",
            &json!([[7, "invalid-final-value"]]),
            detached,
            false,
        );
        let diagnostic = compact_record(&records, "diagnostic");
        assert_eq!(compact_field(diagnostic, "class"), "source");
        assert_eq!(
            compact_field(diagnostic, "code"),
            "normalized_json_type",
            "complete argument admission must precede the active nominal-loan trap"
        );
        nominal.expected("nominal-main", &json!([7]), &snapshot(7), detached);
        nominal.expected("nominal-fuel", &json!([256]), &json!(42), detached);
        nominal.verify_identity();
    }
    for fuel in [100, 4096] {
        limited["execution"] = json!({"instruction_fuel":fuel,"maximum_call_depth":4096,"maximum_value_stack":1000000});
        nominal
            .consumer
            .input("limited.deployment.json", &limited.to_string());
        let (records, _) = nominal.run("limited", &json!([4096]), true, false);
        let diagnostic = compact_record(&records, "diagnostic");
        assert_eq!(compact_field(diagnostic, "class"), "resource");
        assert_eq!(
            compact_field(diagnostic, "code"),
            "normalized_instruction_steps"
        );
        failed_join(diagnostic);
        nominal.expected("nominal-fuel", &json!([256]), &json!(42), true);
        nominal.expected(
            "nominal-transfer",
            &json!([42]),
            &json!({"stamp":42,"payload":42,"side":42}),
            true,
        );
    }
    nominal.verify_identity();
}
