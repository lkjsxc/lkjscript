//! Mixed ordinary/owned contracts, independently authored storage and retained values.
use super::native_byte_buffer::{Export, dependency, export, stage};
use super::native_owned_worklists::{deployment, rejected, standard, unchanged};
use super::*;
use serde_json::json;
use std::cell::{Cell, RefCell};
use std::io::Write;
use std::time::{Duration, Instant};

#[path = "native_indexed_data/seed.rs"]
mod seed;

const LIBRARY: &str = include_str!("../../examples/indexed-data-storage/library.lkjc");
const FLAT: &str = include_str!("../../examples/indexed-data-storage/flat.lkjc");
const CHUNKED: &str = include_str!("../../examples/indexed-data-storage/chunked.lkjc");
const FORWARD: &str = include_str!("../../examples/indexed-data-storage/forward.lkjc");
const APPLICATION: &str = include_str!("../../examples/indexed-data-storage/application.lkjc");
const COSTS: &str = include_str!("../../examples/indexed-data-storage/costs.lkjc");
const BLOCKREAD: &str = include_str!("../../examples/indexed-data-storage/blockread.lkjc");
const TARGETS: [&str; 15] = [
    "indexed-integers",
    "indexed-scalar-flat",
    "indexed-scalar-chunked",
    "indexed-booleans",
    "indexed-payloads",
    "indexed-packets",
    "indexed-item-flat",
    "indexed-item-chunked",
    "indexed-item-nominal",
    "indexed-captured",
    "indexed-cost-direct",
    "indexed-cost-flat",
    "indexed-cost-chunked",
    "indexed-blocks-flat",
    "indexed-blocks-chunked",
];

// Diagnostic output for one long-running matrix, not a runtime or operation quota.
// Other IndexedData tests construct Packages through the default-off stage().
struct MatrixProgress {
    started: Instant,
    records: Cell<usize>,
}

impl MatrixProgress {
    fn new() -> Self {
        let progress = Self {
            started: Instant::now(),
            records: Cell::new(0),
        };
        progress.emit(json!({"event":"matrix-start"}));
        progress
    }

    fn emit(&self, mut record: Value) {
        const MAXIMUM_RECORDS: usize = 650;
        const MAXIMUM_RECORD_BYTES: usize = 1024;
        let ordinal = self.records.get();
        assert!(ordinal < MAXIMUM_RECORDS, "bounded matrix progress records");
        self.records.set(ordinal + 1);
        record["version"] = json!(1);
        record["record"] = json!(ordinal);
        record["elapsed_ms"] = json!(milliseconds(self.started.elapsed()));
        let line = format!("indexed-data-progress {record}\n");
        assert!(
            line.len() <= MAXIMUM_RECORD_BYTES,
            "bounded matrix progress record bytes"
        );
        // Print macros are captured by libtest until a case finishes. Direct locked
        // stderr writes reach the checker log even if this case later times out.
        let mut stderr = std::io::stderr().lock();
        stderr.write_all(line.as_bytes()).unwrap();
        stderr.flush().unwrap();
    }

    fn stage(&self, stage: &'static str) -> StageProgress<'_> {
        self.emit(json!({"event":"stage-start","stage":stage}));
        StageProgress {
            progress: self,
            stage,
            started: Instant::now(),
        }
    }

    fn identity(serial: usize, target: &str, input: &Value, detached: bool) -> Value {
        let bytes = input.to_string();
        json!({
            "serial":serial,
            "target":target,
            "lane":if detached {"detached"} else {"attached"},
            "input_bytes":bytes.len(),
            "input_blake3":blake3::hash(bytes.as_bytes()).to_hex().to_string(),
        })
    }

    fn run(&self, serial: usize, target: &str, input: &Value, detached: bool) -> RunProgress<'_> {
        let identity = Self::identity(serial, target, input, detached);
        let mut record = identity.clone();
        record["event"] = json!("run-start");
        self.emit(record);
        RunProgress {
            progress: self,
            identity,
            started: Instant::now(),
        }
    }
}

fn milliseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

struct StageProgress<'a> {
    progress: &'a MatrixProgress,
    stage: &'static str,
    started: Instant,
}

impl StageProgress<'_> {
    fn complete(self) {
        self.progress.emit(json!({
            "event":"stage-complete",
            "stage":self.stage,
            "duration_ms":milliseconds(self.started.elapsed()),
        }));
    }
}

fn start_stage<'a>(
    progress: Option<&'a MatrixProgress>,
    stage: &'static str,
) -> Option<StageProgress<'a>> {
    progress.map(|progress| progress.stage(stage))
}

fn complete_stage(stage: Option<StageProgress<'_>>) {
    if let Some(stage) = stage {
        stage.complete();
    }
}

struct RunProgress<'a> {
    progress: &'a MatrixProgress,
    identity: Value,
    started: Instant,
}

impl RunProgress<'_> {
    fn complete(self) {
        let mut record = self.identity;
        record["event"] = json!("run-complete");
        record["duration_ms"] = json!(milliseconds(self.started.elapsed()));
        self.progress.emit(record);
    }
}

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
    progress: Option<MatrixProgress>,
    matrix_batch: RefCell<Option<MatrixBatch>>,
}

struct MatrixBatch {
    detached: bool,
    cases: Vec<(Value, BatchCase)>,
}

impl Packages {
    fn stage() -> Self {
        seed::stage()
    }

    fn stage_with_progress(progress: Option<MatrixProgress>) -> Self {
        let phase = start_stage(progress.as_ref(), "supplier-indexed-data");
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
        complete_stage(phase);
        let mut authors = vec![library];
        let mut exports = vec![generic];
        let mut standards = vec![supplier.path];
        for (module, literal) in [
            ("indexed-flat", FLAT),
            ("indexed-chunked", CHUNKED),
            ("indexed-forward", FORWARD),
        ] {
            let phase = start_stage(progress.as_ref(), module);
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
            complete_stage(phase);
        }
        let phase = start_stage(progress.as_ref(), "consumer-template-and-imports");
        let consumer = native();
        for package in &exports {
            stage(&consumer, package);
        }
        complete_stage(phase);
        let mut this = Self {
            authors,
            consumer,
            exports,
            standards,
            identities: vec![],
            serial: Cell::new(0),
            progress,
            matrix_batch: RefCell::new(None),
        };
        let phase = start_stage(this.progress.as_ref(), "consumer-author-and-check");
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
        accept(&this.consumer, "blockread", &this.source(BLOCKREAD));
        let draft = unchanged(&this.consumer, "indexed-blockread");
        assert!(draft.contains("associated-witness-projection"));
        assert!(draft.contains("assoc_type_a3000000000000000000000000000001"));
        assert!(!draft.contains("(field chunks ") && !draft.contains("(field items "));
        complete_stage(phase);
        let phase = start_stage(this.progress.as_ref(), "consumer-build-first");
        let artifact = this.consumer.root.path().join("indexed-data.lkja");
        this.consumer
            .cli(&["build", "--output", path(&artifact)], true);
        complete_stage(phase);
        let phase = start_stage(this.progress.as_ref(), "consumer-build-second-and-equality");
        let second = this.consumer.root.path().join("indexed-data-second.lkja");
        this.consumer
            .cli(&["build", "--output", path(&second)], true);
        assert_eq!(
            std::fs::read(&artifact).unwrap(),
            std::fs::read(second).unwrap()
        );
        complete_stage(phase);
        let phase = start_stage(this.progress.as_ref(), "deployment-and-identity-inventory");
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
        complete_stage(phase);
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
        self.run_with_records(target, arguments, detached, success)
            .1
    }

    fn run_with_records(
        &self,
        target: &str,
        arguments: &Value,
        detached: bool,
        success: bool,
    ) -> (Vec<CompactRecord>, Option<Value>) {
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
            let value = serde_json::from_slice(&std::fs::read(result).unwrap()).unwrap();
            (records, Some(value))
        } else {
            assert!(!result.exists());
            let diagnostic = compact_record(&records, "diagnostic");
            if detached && compact_field(diagnostic, "class") != "source" {
                failed_join(diagnostic);
            }
            (records, None)
        }
    }

    fn rejected_input(&self, target: &str, input: &Value, detached: bool) {
        let (records, result) = self.run_with_records(target, input, detached, false);
        assert!(result.is_none());
        let diagnostic = compact_record(&records, "diagnostic");
        assert_eq!(compact_field(diagnostic, "class"), "source");
        assert_eq!(compact_field(diagnostic, "code"), "normalized_json_type");
    }

    fn expected(&self, target: &str, input: &Value, expected: &Value, detached: bool) {
        self.consumer.input(
            &format!("expected-{}.json", self.serial.get()),
            &expected.to_string(),
        );
        {
            let mut pending = self.matrix_batch.borrow_mut();
            if let Some(batch) = pending.as_mut() {
                assert_eq!(batch.detached, detached, "one exact matrix lane");
                let serial = self.serial.get();
                self.serial.set(serial + 1);
                let mut identity = MatrixProgress::identity(serial, target, input, detached);
                identity["event"] = json!("batch-row-queued");
                if let Some(progress) = &self.progress {
                    progress.emit(identity.clone());
                }
                let mut request = BatchCase::success(
                    format!("indexed-{serial}-{target}"),
                    input,
                    expected.clone(),
                );
                request.target = Some(target.to_owned());
                batch.cases.push((identity, request));
                return;
            }
        }
        let progress = self
            .progress
            .as_ref()
            .map(|progress| progress.run(self.serial.get(), target, input, detached));
        assert_eq!(
            &self.run(target, input, detached, true).unwrap(),
            expected,
            "{target}, detached={detached}"
        );
        // Completion follows the existing input checks, differential/cleanup
        // assertions, child/pipe joins and full independently expected result.
        if let Some(progress) = progress {
            progress.complete();
        }
    }

    fn begin_matrix_batch(&self, detached: bool) {
        assert!(
            self.matrix_batch
                .borrow_mut()
                .replace(MatrixBatch {
                    detached,
                    cases: Vec::new(),
                })
                .is_none()
        );
    }

    fn complete_matrix_batch(&self, detached: bool) {
        let batch = self
            .matrix_batch
            .borrow_mut()
            .take()
            .expect("explicit immutable success matrix");
        assert_eq!(batch.detached, detached);
        let (identities, requests): (Vec<_>, Vec<_>) = batch.cases.into_iter().unzip();
        assert_eq!(requests.len(), if detached { 152 } else { 158 });
        let phase = start_stage(
            self.progress.as_ref(),
            if detached {
                "detached-batch-execution"
            } else {
                "attached-batch-execution"
            },
        );
        // Both public batches share immutable preparation, while every row owns
        // fresh execution and the full independent expected JSON. Source rows
        // also check current authority and run both implementation-disjoint tiers.
        let rows = if detached {
            let descriptor = self
                .consumer
                .root
                .path()
                .join("indexed-integers.deployment.json");
            self.consumer
                .batch("indexed-success", &descriptor, &requests)
        } else {
            self.consumer
                .source_batch("indexed-success", "indexed-integers", &requests)
        };
        assert_eq!(rows.len(), identities.len());
        for (records, mut identity) in rows.iter().zip(identities) {
            let execution = compact_record(records, "execution");
            assert_eq!(
                compact_field(execution, "target"),
                identity["target"].as_str().unwrap(),
                "batch target preserves each named witness path"
            );
            if detached {
                joined(execution);
            } else {
                // The shared source reader also checks real VM/reference and
                // executor cleanup without inventing a resident taskgroup receipt.
                assert_eq!(compact_field(execution, "differential"), "equal");
            }
            identity["event"] = json!("batch-row-validated");
            if let Some(progress) = &self.progress {
                progress.emit(identity);
            }
        }
        complete_stage(phase);
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

fn item(seed: i64) -> Value {
    json!({
        "rows":[
            {"label":format!("row-{seed}-α"),"values":[i64::MIN,seed,i64::MAX]},
            {"label":"","values":[]}
        ],
        "lookup":[
            [i64::MIN,{"label":"empty","values":[]}],
            [7,{"label":format!("entry-{seed}"),"values":[seed,seed+1]}],
            [i64::MAX,{"label":"boundary","values":[i64::MIN,i64::MAX]}]
        ],
        "meta":{"name":format!("item-{seed}-β"),"flags":[true,false,true]}
    })
}

fn item_expected(value: &Value) -> Value {
    let entry = value["lookup"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| pair[0] == json!(7))
        .map(|pair| pair[1].clone())
        .unwrap_or_else(|| json!({"label":"missing","values":[]}));
    json!({"displaced":value,"first":value["rows"][0],"entry":entry,"name":value["meta"]["name"]})
}

#[test]
fn native_indexed_data_separate_packages_snapshots_forwarded_witnesses_and_detached_execution() {
    let packages = Packages::stage_with_progress(Some(MatrixProgress::new()));
    for detached in [false, true] {
        let lane = start_stage(
            packages.progress.as_ref(),
            if detached { "detached" } else { "attached" },
        );
        if detached {
            let phase = start_stage(packages.progress.as_ref(), "remove-authoring-source");
            packages.detach();
            complete_stage(phase);
        }
        packages.begin_matrix_batch(detached);
        for length in [0, 1, 31, 32, 33, 64, 65] {
            let values: Vec<i64> = (0..length).map(|n| n as i64 - 17).collect();
            let mut positions = if length == 0 {
                vec![0]
            } else {
                [0, 31, 32, length - 1]
                    .into_iter()
                    .filter(|index| *index < length)
                    .collect::<Vec<_>>()
            };
            positions.sort_unstable();
            positions.dedup();
            for index in positions {
                let mut after = values.clone();
                let displaced = if length == 0 {
                    0
                } else {
                    let previous = after[index];
                    after[index] = 99;
                    previous
                };
                for (target, chunked) in [
                    ("indexed-blocks-flat", false),
                    ("indexed-blocks-chunked", true),
                ] {
                    let lengths = if chunked {
                        // Independent scalar partition; no product storage metadata.
                        let mut remaining = length;
                        let mut lengths = Vec::new();
                        while remaining != 0 {
                            let block = remaining.min(32);
                            lengths.push(block);
                            remaining -= block;
                        }
                        lengths
                    } else if length == 0 {
                        vec![]
                    } else {
                        vec![length]
                    };
                    let expected = json!({
                        "before":{"values":values,"lengths":lengths},
                        "after":{"values":after,"lengths":lengths},
                        "displaced":displaced,"replaced":length != 0,"length":length,
                    });
                    // Each command rebuilds its own storage from the original input.
                    // Loans close before replace and before final ancestor destruction.
                    packages.expected(target, &json!([values, index, 99]), &expected, detached);
                }
            }
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
        for length in [1, 32, 33, 65] {
            let values: Vec<_> = (0..length).map(|seed| item(seed as i64)).collect();
            for index in [0, length - 1] {
                let expected = item_expected(&values[index]);
                let input = json!([values, index, item(-100)]);
                for target in ["indexed-item-flat", "indexed-item-chunked"] {
                    // The authored function discards every remaining storage owner
                    // before projecting nested fields or returning this complete Item.
                    packages.expected(target, &input, &expected, detached);
                }
            }
        }
        for mut value in [item(-7), item(42)] {
            packages.expected(
                "indexed-item-nominal",
                &json!([value]),
                &item_expected(&value),
                detached,
            );
            // Empty maps exercise the post-destruction fallback without replacing
            // the nested map/list/record result with a scalar-only witness.
            value["lookup"] = json!([]);
            packages.expected(
                "indexed-item-nominal",
                &json!([value]),
                &item_expected(&value),
                detached,
            );
        }
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
        packages.complete_matrix_batch(detached);
        packages.verify_identity();
        complete_stage(lane);
    }
    assert_eq!(packages.serial.get(), 310, "complete expanded matrix runs");
    let progress = packages.progress.as_ref().unwrap();
    assert_eq!(
        progress.records.get(),
        649,
        "paired stages and runs before completion"
    );
    progress.emit(json!({"event":"matrix-complete","runs":packages.serial.get()}));
    assert_eq!(
        progress.records.get(),
        650,
        "complete expanded progress transcript"
    );
    // One ordinary detached invocation also proves the source-free process
    // boundary independently of the batch request and aggregate result files.
    assert_eq!(
        packages.run(
            "indexed-captured",
            &json!([[i64::MIN, 0, i64::MAX], 2]),
            true,
            true
        ),
        Some(json!(i64::MAX))
    );
    packages.verify_identity();
    assert_eq!(
        packages.serial.get(),
        311,
        "310 matrix rows and one command boundary"
    );
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
        let nested: Vec<_> = (0..33).map(item).collect();
        for target in ["indexed-item-flat", "indexed-item-chunked"] {
            for index in [-1, 33] {
                assert!(
                    packages
                        .run(target, &json!([nested, index, item(-9)]), detached, false)
                        .is_none()
                );
            }
            for path in ["/rows/0/values/2", "/lookup/2/1/values/1", "/meta/flags/2"] {
                let mut invalid = nested.clone();
                let replacement = if path.ends_with("flags/2") {
                    json!(0)
                } else {
                    json!(false)
                };
                *invalid[32].pointer_mut(path).unwrap() = replacement;
                // Invalid unused final input must be rejected before index -1 traps.
                packages.rejected_input(target, &json!([invalid, -1, item(-9)]), detached);
                let mut invalid_replacement = item(-9);
                *invalid_replacement.pointer_mut(path).unwrap() = json!("wrong");
                packages.rejected_input(
                    target,
                    &json!([nested, -1, invalid_replacement]),
                    detached,
                );
            }
            packages.expected(
                target,
                &json!([nested, 32, item(-9)]),
                &item_expected(&nested[32]),
                detached,
            );
        }
        let mut invalid_nominal = item(9);
        invalid_nominal["lookup"][2][1]["values"][1] = json!(false);
        packages.rejected_input("indexed-item-nominal", &json!([invalid_nominal]), detached);
        let mut empty_rows = item(9);
        empty_rows["rows"] = json!([]);
        assert!(
            packages
                .run(
                    "indexed-item-nominal",
                    &json!([empty_rows]),
                    detached,
                    false
                )
                .is_none()
        );
        packages.expected(
            "indexed-item-nominal",
            &json!([item(9)]),
            &item_expected(&item(9)),
            detached,
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
