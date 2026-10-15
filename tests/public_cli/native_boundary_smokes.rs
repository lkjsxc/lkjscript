//! Small executable observations supplement the complete source semantic matrices.
//! Fixtures use only the same-source public host adapters; observations always spawn
//! the exact selected product with a closed environment and join it before reporting.
use super::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::time::{Duration, Instant};

#[path = "native_boundary_smokes/shared.rs"]
mod shared;

const EVIDENCE: &str = "LKJSCRIPT_NATIVE_BOUNDARY_EVIDENCE";

fn field<'a>(records: &'a [CompactRecord], record: &str, name: &str) -> &'a str {
    compact_field(compact_record(records, record), name).unwrap()
}

fn sha256(file: &Path) -> String {
    let mut file = File::open(file).unwrap();
    let mut digest = Sha256::new();
    let mut bytes = [0; 65536];
    loop {
        let count = file.read(&mut bytes).unwrap();
        if count == 0 {
            break;
        }
        digest.update(&bytes[..count]);
    }
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

struct Fixture {
    root: tempfile::TempDir,
    project: PathBuf,
}

impl Fixture {
    fn new(template: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        let this = Self { root, project };
        this.host(
            &[
                "new",
                path(&this.project),
                "--template",
                template,
                "--name",
                "boundary",
            ],
            true,
        );
        this
    }

    fn arguments(&self, arguments: &[&str]) -> Vec<String> {
        let mut full = Vec::new();
        if !arguments.starts_with(&["new"]) && !arguments.starts_with(&["package", "builtin"]) {
            full.extend(["--project".to_owned(), path(&self.project).to_owned()]);
        }
        full.extend(arguments.iter().map(|argument| (*argument).to_owned()));
        full
    }

    fn host(&self, arguments: &[&str], success: bool) -> Vec<CompactRecord> {
        let response = semantic_cli::execute(arguments, self.arguments(arguments))
            .expect("fixture operation must have a public host adapter");
        assert_eq!(
            response.success,
            success,
            "{arguments:?}: {}",
            String::from_utf8_lossy(&response.stdout)
        );
        assert!(response.stderr.is_empty());
        parse_records("public host fixture", &response.stdout).unwrap()
    }

    fn input(&self, name: &str, body: &str) -> PathBuf {
        let file = self.root.path().join(name);
        std::fs::write(&file, body).unwrap();
        file
    }

    fn revision(&self) -> String {
        field(&self.host(&["status"], true), "revision", "id").to_owned()
    }

    fn plan(&self, input: &Path) -> Vec<CompactRecord> {
        self.host(&["change", "plan", "--input-file", path(input)], true)
    }

    fn apply(&self, input: &Path, plan: &[CompactRecord], success: bool) -> Vec<CompactRecord> {
        self.host(
            &[
                "change",
                "apply",
                "--input-file",
                path(input),
                "--plan",
                field(plan, "plan", "token"),
            ],
            success,
        )
    }

    fn author(&self, source: &str) {
        let input = self.input(
            "author.lkjc",
            &format!("request base={}\n{source}", self.revision()),
        );
        self.apply(&input, &self.plan(&input), true);
        self.host(&["check"], true);
    }

    fn build(&self, name: &str) -> PathBuf {
        let artifact = self.root.path().join(name);
        self.host(&["build", "--output", path(&artifact)], true);
        artifact
    }

    fn detach(&self) {
        std::fs::remove_dir_all(&self.project).unwrap();
        // Public authoring leaves its proposal beside the project; detached bytes
        // must also stand without that source or exported supplier transports.
        for entry in std::fs::read_dir(self.root.path()).unwrap() {
            let file = entry.unwrap().path();
            if matches!(
                file.extension().and_then(|value| value.to_str()),
                Some("lkjc" | "lkjp")
            ) {
                let metadata = std::fs::symlink_metadata(&file).unwrap();
                assert!(metadata.is_file() && !metadata.file_type().is_symlink());
                std::fs::remove_file(file).unwrap();
            }
        }
        self.assert_detached();
    }

    fn assert_detached(&self) {
        assert!(!self.project.exists());
        for entry in std::fs::read_dir(self.root.path()).unwrap() {
            assert!(!matches!(
                entry
                    .unwrap()
                    .path()
                    .extension()
                    .and_then(|value| value.to_str()),
                Some("lkjc" | "lkjp")
            ));
        }
    }
}

struct Smoke {
    root: tempfile::TempDir,
    selected: PathBuf,
    executable: PathBuf,
    digest: String,
    observations: Vec<Value>,
}

struct Running {
    child: support::SpawnedChild,
    arguments: Vec<String>,
    stdout: PathBuf,
    stderr: PathBuf,
}

impl Smoke {
    fn new() -> Self {
        if env::var_os(EVIDENCE).is_some() {
            assert_eq!(
                semantic_cli::Mode::selected(),
                semantic_cli::Mode::CandidateProcess
            );
        }
        let selected = binary();
        let digest = sha256(&selected);
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("lkjscript");
        copy_executable(&selected, &executable);
        assert_eq!(sha256(&executable), digest);
        Self {
            root,
            selected,
            executable,
            digest,
            observations: Vec::new(),
        }
    }

    fn spawn(&self, arguments: &[&str]) -> Running {
        assert_eq!(sha256(&self.executable), self.digest);
        let serial = self.observations.len();
        let stdout = self.root.path().join(format!("{serial}.stdout"));
        let stderr = self.root.path().join(format!("{serial}.stderr"));
        let child = support::spawn(
            Command::new(&self.executable)
                .args(arguments)
                .current_dir(self.root.path())
                .env_clear()
                .env("PATH", "")
                .env("LC_ALL", "C")
                .env("TZ", "UTC")
                .stdin(Stdio::null())
                .stdout(File::create(&stdout).unwrap())
                .stderr(File::create(&stderr).unwrap()),
        )
        .unwrap();
        Running {
            child,
            arguments: arguments
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect(),
            stdout,
            stderr,
        }
    }

    fn join(&mut self, mut running: Running, limit: Duration) -> Output {
        let deadline = Instant::now() + limit;
        while running.child.try_wait().unwrap().is_none() {
            assert!(
                Instant::now() < deadline,
                "bounded native boundary child: {:?}",
                running.arguments
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut output = running.child.wait_with_output().unwrap();
        assert!(std::fs::metadata(&running.stdout).unwrap().len() <= 16 * 1024 * 1024);
        assert!(std::fs::metadata(&running.stderr).unwrap().len() <= 16 * 1024 * 1024);
        output.stdout = std::fs::read(running.stdout).unwrap();
        output.stderr = std::fs::read(running.stderr).unwrap();
        assert_eq!(sha256(&self.executable), self.digest);
        self.observations.push(json!({"arguments":running.arguments, "exit_code":output.status.code().expect("normal child exit"), "joined":true, "environment_cleared":true, "executable_sha256":self.digest}));
        output
    }

    fn call(&mut self, arguments: &[&str], success: bool) -> Vec<CompactRecord> {
        let running = self.spawn(arguments);
        let output = self.join(running, Duration::from_secs(60));
        assert_eq!(
            output.status.success(),
            success,
            "{arguments:?}: {} / {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        parse_records("actual selected product", &output.stdout).unwrap()
    }

    fn project(
        &mut self,
        fixture: &Fixture,
        arguments: &[&str],
        success: bool,
    ) -> Vec<CompactRecord> {
        let full = fixture.arguments(arguments);
        self.call(
            &full.iter().map(String::as_str).collect::<Vec<_>>(),
            success,
        )
    }

    fn finish(self, id: &str, fixtures: Vec<Fixture>) {
        assert_eq!(self.observations.len(), 1);
        assert_eq!(sha256(&self.selected), self.digest);
        assert_eq!(sha256(&self.executable), self.digest);
        for fixture in fixtures {
            fixture.root.close().unwrap();
        }
        self.root.close().unwrap();
        if let Some(directory) = env::var_os(EVIDENCE) {
            let directory = PathBuf::from(directory);
            assert!(directory.is_absolute());
            let metadata = std::fs::symlink_metadata(&directory).unwrap();
            assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
            let bytes = serde_json::to_vec(&json!({"schema":{"identity":"lkjscript-native-boundary-smoke","version":1}, "test":format!("native_boundary_smokes::{id}"), "candidate_sha256":self.digest, "observations":self.observations, "cleanup_complete":true})).unwrap();
            assert!(bytes.len() <= 1024 * 1024);
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.join(format!("{id}.json")))
                .unwrap();
            file.write_all(&bytes).unwrap();
            file.sync_all().unwrap();
        }
    }
}

const REFRESH: &str = "declarations.begin\n(units (module create review (function create left (visibility private) (returns I64) (effect pure) (body (i64 10))) (function create right (visibility private) (returns I64) (effect pure) (body (i64 20)))))\ndeclarations.end\n";

fn edited(fixture: &Fixture, name: &str, before: i64, after: i64) -> PathBuf {
    let draft = fixture.root.path().join(format!("{name}.lkjc"));
    fixture.host(
        &[
            "change",
            "draft",
            "--declaration",
            &format!("review::{name}"),
            "--output",
            path(&draft),
        ],
        true,
    );
    let source = std::fs::read_to_string(&draft).unwrap();
    assert_eq!(source.matches(&format!("(i64 {before})")).count(), 1);
    std::fs::write(
        &draft,
        source.replace(&format!("(i64 {before})"), &format!("(i64 {after})")),
    )
    .unwrap();
    draft
}

#[test]
fn review_refresh() {
    let fixture = Fixture::new("command");
    fixture.author(REFRESH);
    let left = edited(&fixture, "left", 10, 11);
    let literal = std::fs::read(&left).unwrap();
    let original_file = fixture.root.path().join("original.lkjplan");
    let original = fixture.host(
        &[
            "change",
            "plan",
            "--input-file",
            path(&left),
            "--output",
            path(&original_file),
        ],
        true,
    );
    let right = edited(&fixture, "right", 20, 22);
    fixture.apply(&right, &fixture.plan(&right), true);
    let onto = fixture.revision();
    let before = content_inventory(&fixture.project);
    let exported = fixture.root.path().join("refreshed.lkjplan");
    let mut smoke = Smoke::new();
    let refreshed = smoke.project(
        &fixture,
        &[
            "change",
            "refresh",
            "--input-file",
            path(&left),
            "--plan",
            field(&original, "plan", "token"),
            "--onto",
            &onto,
            "--output",
            path(&exported),
        ],
        true,
    );
    assert_ne!(
        field(&refreshed, "plan", "token"),
        field(&original, "plan", "token")
    );
    assert_eq!(field(&refreshed, "refresh", "onto"), onto);
    assert_eq!(
        field(&refreshed, "refresh", "original-plan"),
        field(&original, "plan", "token")
    );
    assert_eq!(
        decode_logical_change_plan(BufReader::new(File::open(&exported).unwrap()))
            .unwrap()
            .token,
        field(&refreshed, "plan", "token")
    );
    let allocations = |file: &Path| {
        parse_records("complete retained plan", &std::fs::read(file).unwrap())
            .unwrap()
            .into_iter()
            .filter(|row| row.operation == "logical-plan.allocation")
            .map(|row| {
                (
                    row.operation,
                    row.fields
                        .into_iter()
                        .map(|field| (field.name, field.value))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(allocations(&original_file), allocations(&exported));
    assert_eq!(std::fs::read(&left).unwrap(), literal);
    assert_eq!(content_inventory(&fixture.project), before);
    fixture.apply(&left, &refreshed, true);
    fixture.host(&["check"], true);
    smoke.finish("review_refresh", vec![fixture]);
}

fn fifo(fixture: &Fixture) -> PathBuf {
    let current = fixture.project.join("derived/compiler/CURRENT");
    std::fs::remove_file(&current).unwrap();
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        &current,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .unwrap();
    current
}

#[test]
fn cache_read_fifo() {
    let fixture = Fixture::new("command");
    fixture.host(&["check"], true);
    let original = fixture.build("before.lkja");
    let before = content_inventory(&fixture.project);
    let current = fifo(&fixture);
    let mut smoke = Smoke::new();
    let checked = smoke.project(&fixture, &["check"], true);
    assert_eq!(field(&checked, "compilation", "cache"), "clean-recovery");
    assert_eq!(field(&checked, "tests", "failed"), "0");
    assert!(std::fs::symlink_metadata(current).unwrap().is_file());
    assert_eq!(content_inventory(&fixture.project), before);
    assert_eq!(
        std::fs::read(original).unwrap(),
        std::fs::read(fixture.build("after.lkja")).unwrap()
    );
    assert_eq!(
        field(&fixture.host(&["check"], true), "compilation", "cache"),
        "exact-current"
    );
    smoke.finish("cache_read_fifo", vec![fixture]);
}

#[test]
fn cache_write_fifo() {
    let fixture = Fixture::new("command");
    fixture.host(&["check"], true);
    let input = fixture.input("keyed.lkjc", &format!("request base={} idempotency=boundary-cache-once\ndeclarations.begin\n(units (module create cache-boundary))\ndeclarations.end\n", fixture.revision()));
    let plan = fixture.plan(&input);
    let current = fifo(&fixture);
    let before_fifo = std::fs::symlink_metadata(&current).unwrap();
    let mut smoke = Smoke::new();
    let applied = smoke.project(
        &fixture,
        &[
            "change",
            "apply",
            "--input-file",
            path(&input),
            "--plan",
            field(&plan, "plan", "token"),
        ],
        true,
    );
    assert_eq!(field(&applied, "result", "status"), "accepted");
    assert_eq!(field(&applied, "derived-cache", "status"), "failed");
    assert_eq!(
        field(&applied, "derived-cache", "diagnostic-class"),
        "corrupt"
    );
    assert_eq!(
        field(&applied, "derived-cache", "diagnostic-code"),
        "compilation_cache_regular_type"
    );
    let head = std::fs::read(fixture.project.join("HEAD")).unwrap();
    let replay = fixture.apply(&input, &fixture.plan(&input), true);
    assert_eq!(field(&replay, "result", "status"), "already-accepted");
    assert_eq!(
        field(&replay, "derived-cache", "status"),
        "not-attempted-replay"
    );
    for name in ["digest", "revision-record"] {
        assert_eq!(
            field(&applied, "receipt", name),
            field(&replay, "receipt", name)
        );
    }
    let after_fifo = std::fs::symlink_metadata(&current).unwrap();
    assert!(after_fifo.file_type().is_fifo());
    assert_eq!(
        (before_fifo.dev(), before_fifo.ino()),
        (after_fifo.dev(), after_fifo.ino())
    );
    assert_eq!(
        field(&fixture.host(&["check"], true), "compilation", "cache"),
        "clean-recovery"
    );
    assert_eq!(std::fs::read(fixture.project.join("HEAD")).unwrap(), head);
    smoke.finish("cache_write_fifo", vec![fixture]);
}

#[test]
fn publication_lock() {
    use fs2::FileExt;
    let fixture = Fixture::new("command");
    let base = fixture.revision();
    let left = fixture.input(
        "left.lkjc",
        &format!("request base={base}\ncreate.module as=$left name=left\n"),
    );
    let right = fixture.input(
        "right.lkjc",
        &format!("request base={base}\ncreate.module as=$right name=right\n"),
    );
    let left_plan = fixture.plan(&left);
    let right_plan = fixture.plan(&right);
    let lock = File::open(fixture.project.join("LOCK")).unwrap();
    lock.lock_exclusive().unwrap();
    let mut smoke = Smoke::new();
    let full = fixture.arguments(&[
        "change",
        "apply",
        "--input-file",
        path(&left),
        "--plan",
        field(&left_plan, "plan", "token"),
    ]);
    let mut child = smoke.spawn(&full.iter().map(String::as_str).collect::<Vec<_>>());
    let (host, actual) = std::thread::scope(|scope| {
        // Scope failure must release the lock before joining a host operation
        // which may itself be waiting for this exact lock.
        struct HeldLock(File);
        impl Drop for HeldLock {
            fn drop(&mut self) {
                let _ = FileExt::unlock(&self.0);
            }
        }
        let lock = HeldLock(lock);
        let (started, ready) = std::sync::mpsc::sync_channel(1);
        let fixture = &fixture;
        let right = &right;
        let right_plan = &right_plan;
        let host = scope.spawn(move || {
            started.send(()).unwrap();
            semantic_cli::execute(
                &[
                    "change",
                    "apply",
                    "--input-file",
                    path(right),
                    "--plan",
                    field(right_plan, "plan", "token"),
                ],
                fixture.arguments(&[
                    "change",
                    "apply",
                    "--input-file",
                    path(right),
                    "--plan",
                    field(right_plan, "plan", "token"),
                ]),
            )
            .unwrap()
        });
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let candidate_waiting = child.child.try_wait().unwrap().is_none();
        let host_waiting = !host.is_finished();
        FileExt::unlock(&lock.0).unwrap();
        assert!(candidate_waiting, "candidate must wait for repository lock");
        assert!(host_waiting, "public host must wait for repository lock");
        let actual = smoke.join(child, Duration::from_secs(30));
        (host.join().unwrap(), actual)
    });
    assert!(actual.stderr.is_empty() && host.stderr.is_empty());
    assert_ne!(
        host.success,
        actual.status.success(),
        "one complete publication wins"
    );
    let actual_records = parse_records("candidate competing publication", &actual.stdout).unwrap();
    let host_records = parse_records("host competing publication", &host.stdout).unwrap();
    let (winner, loser) = if actual.status.success() {
        (&actual_records, &host_records)
    } else {
        (&host_records, &actual_records)
    };
    assert_eq!(field(winner, "result", "status"), "accepted");
    assert_eq!(field(loser, "result", "status"), "failure");
    assert_eq!(field(loser, "diagnostic", "class"), "semantic");
    assert!(matches!(
        field(loser, "diagnostic", "code"),
        "change_authored_stale_base" | "change_stale_base"
    ));
    if !actual.status.success() {
        assert_eq!(actual.status.code(), Some(7));
    }
    if !host.success {
        assert_eq!(host.code, Some(7));
    }
    assert_eq!(fixture.revision(), field(winner, "revision", "result"));
    let (present, absent) = if actual.status.success() {
        ("left", "right")
    } else {
        ("right", "left")
    };
    let found = fixture.host(&["query", "find", "module", present], true);
    assert_eq!(field(&found, "owner", "name"), present);
    let missing = fixture.host(&["query", "find", "module", absent], true);
    assert_eq!(field(&missing, "summary", "returned"), "0");
    assert!(!missing.iter().any(|record| record.operation == "owner"));
    let head = std::fs::read(fixture.project.join("HEAD")).unwrap();
    fixture.apply(
        if actual.status.success() {
            &right
        } else {
            &left
        },
        if actual.status.success() {
            &right_plan
        } else {
            &left_plan
        },
        false,
    );
    assert_eq!(std::fs::read(fixture.project.join("HEAD")).unwrap(), head);
    fixture.host(&["check"], true);
    smoke.finish("publication_lock", vec![fixture]);
}

// The protected field is actually projected; merely including a read-view declaration
// in an otherwise unrelated artifact would not observe the read-view boundary.
const VIEW: &str = r#"
    (read-view create Probe (visibility private)
      (scope-parameter read_scope_e1000000000000000000000000000001 s)
      (view-field field_e1000000000000000000000000000001 source (type ByteBuffer)
        (read-reference (read-scope (declaration Probe) read_scope_e1000000000000000000000000000001)))
      (view-field field_e1000000000000000000000000000002 length (type I64) (ordinary)))
    (function create make (visibility private) (effect pure)
      (scope-parameter read_scope_e1000000000000000000000000000002 s)
      (parameter create source (type ByteBuffer) (use borrow))
      (scope-source source (read-scope (declaration make) read_scope_e1000000000000000000000000000002))
      (returns (read-view-application Probe (types) (read-scope (declaration make) read_scope_e1000000000000000000000000000002))
        (view-from source (read-scope (declaration make) read_scope_e1000000000000000000000000000002)))
      (body (view-create Probe (types) (scope (read-scope (declaration make) read_scope_e1000000000000000000000000000002)) (source source)
        (field field_e1000000000000000000000000000001 (local source))
        (field field_e1000000000000000000000000000002 (call std::buffer-length (local source))))))
    (function create observe (visibility private) (effect pure)
      (scope-parameter read_scope_e1000000000000000000000000000003 s)
      (parameter create view (type (read-view-application Probe (types) (read-scope (declaration observe) read_scope_e1000000000000000000000000000003))) (use borrow))
      (scope-source view (read-scope (declaration observe) read_scope_e1000000000000000000000000000003))
      (returns (record (length I64) (first I64)) (value))
      (body (borrow-view-field (local view) Probe field_e1000000000000000000000000000001
        (binding source (type ByteBuffer))
        (in (record structural
          (field length (view-data-field (local view) Probe field_e1000000000000000000000000000002))
          (field first (call std::buffer-get (i64 0) (local source))))))))
"#;

const BATCH: &str = r#"
    (type-alias Result (record (payload Bytes) (view (record (length I64) (first I64))) (quotient I64)))
    (function create calculate (visibility private) (effect pure)
      (parameter create divisor (type I64)) (returns Result)
      (body (let
        (binding empty (type ByteBuffer) (call std::buffer-empty))
        (binding first (type ByteBuffer) (call std::buffer-push (i64 2) (local empty)))
        (binding source (type ByteBuffer)
          (call std::buffer-push (i64 3) (local first)))
        (binding view (as $batch-view-scope)
          (type (read-view-application Probe (types) (lexical-read-scope calculate $batch-view-scope)))
          (borrow-call (call make (scopes (lexical-read-scope calculate $batch-view-scope)) (local source))
            (binding constructed (type (read-view-application Probe (types) (lexical-read-scope calculate $batch-view-scope))))
            (in (view-move (local constructed)))))
        (binding observed (type (record (length I64) (first I64)))
          (call observe (scopes (lexical-read-scope calculate $batch-view-scope)) (local view)))
        (binding quotient (type I64) (call std::divide (i64 6) (local divisor)))
        (binding payload (type Bytes) (sequence (view-drop (local view)) (call std::buffer-freeze (local source))))
        (in (record structural (field payload (local payload)) (field view (local observed)) (field quotient (local quotient)))))))
    (function create recovery (visibility private) (effect pure) (returns Result)
      (body (call calculate (i64 3))))
    (component create command (visibility private)
      (port create calculate (type (function (I64) Result)) (function calculate))
      (port create recovery (type (function () Result)) (function recovery))))
  (target create boundary (component boundary::command) (runner command) (port boundary::command::calculate))
  (target create boundary-recovery (component boundary::command) (runner command) (port boundary::command::recovery))
"#;

fn command_bundle(fixture: &Fixture) -> PathBuf {
    fixture.author(&format!("declarations.begin\n(units (use std builtin) (module create boundary\n{VIEW}{BATCH})\ndeclarations.end\n"));
    fixture.build("application.lkja");
    let mut descriptor: Value = serde_json::from_slice(
        &std::fs::read(fixture.project.join("command.deployment.json")).unwrap(),
    )
    .unwrap();
    descriptor["artifact"] = json!("application.lkja");
    descriptor["target"] = json!("boundary");
    let descriptor = fixture.input("command.json", &descriptor.to_string());
    fixture.detach();
    descriptor
}

fn clean_execution(records: &[CompactRecord]) {
    let production: Value =
        serde_json::from_str(field(records, "execution", "production-observation")).unwrap();
    for name in [
        "live_handles_after",
        "live_locals_after",
        "live_operands_after",
        "live_call_frames_after",
        "live_transactions_after",
        "live_type_bindings_after",
    ] {
        assert_eq!(production[name], 0, "{name}");
    }
    let cleanup: Value = serde_json::from_str(field(records, "execution", "cleanup")).unwrap();
    assert_eq!(cleanup["remaining_tasks"], 0);
    assert_eq!(cleanup["cleanup_failures"], json!([]));
    let executor: Value =
        serde_json::from_str(field(records, "execution", "executor-observation")).unwrap();
    assert_eq!(executor["dispatch_open"], false);
    assert_eq!(executor["active_dispatches"], 0);
    assert_eq!(executor["remaining_workers"], 0);
    assert_eq!(executor["workers_received"], 0);
    assert_eq!(executor["workers_handed_off"], 0);
    assert_eq!(executor["joined_workers"], executor["workers_started"]);
}

#[test]
fn detached_foreground() {
    shared::foreground();
}

#[test]
fn deployment_batch() {
    let fixture = Fixture::new("command");
    fixture.host(&["check"], true);
    let descriptor = command_bundle(&fixture);
    let requests = format!(
        "{}\n{}\n{}\n",
        json!({"id":"before","arguments_json":"[2]"}),
        json!({"id":"refusal","arguments_json":"[0]"}),
        json!({"id":"recovery","arguments_json":"[]","target":"boundary-recovery"})
    );
    let input = fixture.input("requests.jsonl", &requests);
    let output = fixture.root.path().join("results.jsonl");
    let artifact = sha256(&fixture.root.path().join("application.lkja"));
    let descriptor_bytes = std::fs::read(&descriptor).unwrap();
    let mut smoke = Smoke::new();
    let records = smoke.call(
        &[
            "run",
            "--deployment",
            path(&descriptor),
            "--batch-input-file",
            path(&input),
            "--batch-result-file",
            path(&output),
        ],
        true,
    );
    assert_eq!(field(&records, "result", "command"), "run.batch");
    for (name, expected) in [
        ("rows", "3"),
        ("passed", "2"),
        ("failed", "1"),
        ("artifact-preparations", "1"),
    ] {
        assert_eq!(field(&records, "batch", name), expected);
    }
    assert_eq!(field(&records, "batch", "artifact-sha256"), artifact);
    let rows = std::fs::read_to_string(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 3);
    for (row, id) in rows.iter().zip(["before", "refusal", "recovery"]) {
        assert_eq!(
            row.as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["id", "exit_code", "stdout", "result_json"])
        );
        assert_eq!(row["id"], id);
        let result = parse_records(
            "complete detached row",
            row["stdout"].as_str().unwrap().as_bytes(),
        )
        .unwrap();
        if id == "refusal" {
            assert_eq!(row["exit_code"], 2);
            assert!(row["result_json"].is_null());
            assert_eq!(field(&result, "diagnostic", "class"), "semantic");
            assert_eq!(
                field(&result, "diagnostic", "code"),
                "normalized_integer_division"
            );
            let notes: Vec<String> =
                serde_json::from_str(field(&result, "diagnostic", "notes")).unwrap();
            assert!(notes.iter().any(|note| note == "foreground cleanup: admission-stopped=true remaining-owned-tasks=0 failures=0"));
            assert!(notes.iter().any(|note| note.starts_with("structured worker cleanup: dispatch-stopped=true active=0 remaining-workers=0 joined-workers=")));
        } else {
            assert_eq!(row["exit_code"], 0);
            assert_eq!(
                serde_json::from_str::<Value>(row["result_json"].as_str().unwrap()).unwrap(),
                json!({"payload":{"$bytes":"AgM="},"view":{"length":2,"first":2},"quotient":if id == "before" {3} else {2}})
            );
            assert_eq!(
                field(&result, "execution", "target"),
                if id == "before" {
                    "boundary"
                } else {
                    "boundary-recovery"
                }
            );
            clean_execution(&result);
        }
    }
    assert_eq!(std::fs::read(input).unwrap(), requests.as_bytes());
    assert_eq!(std::fs::read(descriptor).unwrap(), descriptor_bytes);
    assert_eq!(
        sha256(&fixture.root.path().join("application.lkja")),
        artifact
    );
    fixture.assert_detached();
    smoke.finish("deployment_batch", vec![fixture]);
}

#[test]
fn shared_parallel_http() {
    shared::execute();
}
