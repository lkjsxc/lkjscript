use super::*;
use std::cell::Cell;

const LIBRARY: &str = include_str!("../../../examples/data-sequences/library.lkjc");
const SOURCES: [(&str, &str); 4] = [
    (
        "trace",
        include_str!("../../../examples/data-sequences/trace.lkjc"),
    ),
    (
        "application",
        include_str!("../../../examples/data-sequences/application.lkjc"),
    ),
    (
        "tasks",
        include_str!("../../../examples/data-sequences/tasks.lkjc"),
    ),
    (
        "owned",
        include_str!("../../../examples/data-sequences/owned.lkjc"),
    ),
];
const TARGETS: [&str; 12] = [
    "data-trace",
    "data-trace-batch",
    "data-scalar",
    "data-bool",
    "data-nested",
    "data-get",
    "data-replace",
    "data-empty-get",
    "data-order",
    "data-transfer",
    "data-shared",
    "data-owned-replace",
];

pub(super) struct DataSequences {
    pub supplier: Native,
    pub consumer: Native,
    pub generic: Export,
    identities: Vec<(PathBuf, blake3::Hash)>,
    serial: Cell<usize>,
}

fn native() -> Native {
    let mut public = Native::template("command");
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        public.root.disable_cleanup(true);
        println!(
            "retained native data-sequence evidence: {}",
            public.root.path().display()
        );
    }
    public
}

pub(super) fn accept(public: &Native, label: &str, source: &str) {
    let input = public.input(
        &format!("{label}.lkjc"),
        &format!("request base={}\n{source}", public.revision()),
    );
    let plan = public.plan(&input, true);
    public.apply(&input, &plan, true);
    let checked = public.cli(&["check"], true);
    let tests = compact_record(&checked, "tests");
    assert_eq!(compact_field(tests, "failed"), "0");
    assert_eq!(compact_field(tests, "differential"), "equal");
}

impl DataSequences {
    pub fn new() -> Self {
        let supplier = native();
        accept(&supplier, "library", LIBRARY);
        let draft = unchanged(&supplier, "data-sequences");
        assert!(draft.contains("(constraint transferable)"));
        for form in [
            "sequence-empty",
            "sequence-length",
            "sequence-push",
            "sequence-pop",
            "sequence-get",
            "sequence-replace",
        ] {
            assert!(
                draft.contains(form),
                "independently exported generic form {form}"
            );
        }
        for forbidden in [
            "OwnedI64Cell",
            "ByteBuffer",
            "owned-implementation",
            "Payload",
        ] {
            assert!(
                !draft.contains(forbidden),
                "generic library requires concrete consumer {forbidden}"
            );
        }
        // This exact transport exists before the concrete consumer and its types exist.
        let generic = export(&supplier);
        let consumer = native();
        stage(&consumer, &generic);
        let import = format!(
            "declarations.begin\n(units (use data-sequences {} {}))\ndeclarations.end\n",
            generic.package, generic.revision
        );
        for (offset, (label, source)) in SOURCES.into_iter().enumerate() {
            let dependency = if offset == 0 {
                dependency(&generic)
            } else {
                String::new()
            };
            accept(&consumer, label, &format!("{dependency}{import}{source}"));
        }
        let artifact = consumer.root.path().join("data-sequences.lkja");
        consumer.cli(&["build", "--output", path(&artifact)], true);
        let mut files = vec![
            supplier.executable.clone(),
            consumer.executable.clone(),
            artifact,
        ];
        for target in TARGETS {
            files.push(deployment(&consumer, "data-sequences.lkja", target));
        }
        let identities: Vec<_> = files
            .into_iter()
            .map(|file| {
                let hash = blake3::hash(&std::fs::read(&file).unwrap());
                (file, hash)
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

    pub fn detach(&self) {
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

    pub fn verify_identity(&self) {
        for (file, expected) in &self.identities {
            assert_eq!(&blake3::hash(&std::fs::read(file).unwrap()), expected);
        }
    }

    pub fn run(
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
        let mut command = if detached {
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
            if detached {
                joined(execution);
            } else {
                assert_eq!(compact_field(execution, "differential"), "equal");
            }
            Some(serde_json::from_slice(&std::fs::read(result).unwrap()).unwrap())
        } else {
            assert!(
                !result.exists(),
                "failed execution exposed a partial result"
            );
            None
        };
        (records, value)
    }

    pub fn expected(
        &self,
        target: &str,
        input: &Value,
        expected: &Value,
        detached: bool,
    ) -> Vec<CompactRecord> {
        let (records, actual) = self.run(target, input, detached, true);
        assert_eq!(&actual.unwrap(), expected, "{target}, detached={detached}");
        records
    }
}

pub(super) fn joined(execution: &CompactRecord) {
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

pub(super) fn failed_join(diagnostic: &CompactRecord) {
    let notes: Vec<String> = serde_json::from_str(compact_field(diagnostic, "notes")).unwrap();
    assert!(notes.iter().any(|note| note
        == "foreground cleanup: admission-stopped=true remaining-owned-tasks=0 failures=0"));
    assert!(notes.iter().any(|note| note == "structured worker cleanup: dispatch-stopped=true active=0 remaining-workers=0 joined-workers=0"));
}
