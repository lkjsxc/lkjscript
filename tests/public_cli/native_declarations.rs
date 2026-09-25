//! Public native authoring, canonical re-entry and publication failures.
use super::*;

fn compact_field<'a>(record: &'a CompactRecord, name: &str) -> &'a str {
    super::compact_field(record, name).unwrap()
}

struct Native {
    root: tempfile::TempDir,
    executable: PathBuf,
    project: PathBuf,
}

impl Native {
    fn new() -> Self {
        Self::template("minimal")
    }

    fn template(template: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("lkjscript");
        copy_executable(&binary(), &executable);
        let project = root.path().join("project");
        let this = Self {
            root,
            executable,
            project,
        };
        this.cli(
            &[
                "new",
                path(&this.project),
                "--template",
                template,
                "--name",
                "native",
            ],
            true,
        );
        this
    }

    fn cli(&self, arguments: &[&str], success: bool) -> Vec<CompactRecord> {
        let mut command = Command::new(&self.executable);
        if !arguments.starts_with(&["package", "builtin"])
            && !arguments.starts_with(&["run", "--deployment"])
        {
            command.arg("--project").arg(&self.project);
        }
        let output = support::output(
            command
                .args(arguments)
                .current_dir(self.root.path())
                .env_clear()
                .env("PATH", ""),
        )
        .unwrap();
        assert_eq!(
            output.status.success(),
            success,
            "{arguments:?}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        parse_records("native public result", &output.stdout).unwrap()
    }

    fn revision(&self) -> String {
        compact_field(
            compact_record(&self.cli(&["status"], true), "revision"),
            "id",
        )
        .to_owned()
    }

    fn input(&self, name: &str, source: &str) -> PathBuf {
        let input = self.root.path().join(name);
        std::fs::write(&input, source).unwrap();
        input
    }

    fn plan(&self, input: &Path, success: bool) -> Vec<CompactRecord> {
        self.cli(&["change", "plan", "--input-file", path(input)], success)
    }

    fn apply(&self, input: &Path, plan: &[CompactRecord], success: bool) -> Vec<CompactRecord> {
        self.cli(
            &[
                "change",
                "apply",
                "--input-file",
                path(input),
                "--plan",
                compact_field(compact_record(plan, "plan"), "token"),
            ],
            success,
        )
    }
}

// Editor operations on these literal arithmetic fixtures only. Bodies are authored below;
// this helper neither resolves names nor constructs semantic declarations from host data.
fn form_span(source: &str, prefix: &str) -> std::ops::Range<usize> {
    let start = source.find(prefix).unwrap();
    let mut depth = 0;
    for (offset, byte) in source.as_bytes()[start..].iter().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return start..start + offset + 1;
                }
            }
            _ => {}
        }
    }
    unreachable!("complete numeric fixture form")
}

fn insert_argument(source: &mut String, callee: &str, argument: &str) {
    let prefix = format!("(call {callee} ");
    let mut cursor = 0;
    while source[cursor..].contains(&prefix) {
        let span = form_span(&source[cursor..], &prefix);
        let end = cursor + span.end;
        source.insert_str(end - 1, argument);
        cursor = end + argument.len();
    }
}

fn draft_reference(source: &str, name: &str) -> String {
    source
        .lines()
        .find_map(|line| {
            let mut words = line.split_whitespace();
            (words.next() == Some("(reference"))
                .then(|| words.next())
                .flatten()
                .filter(|word| word.starts_with(&format!("ref_{name}_")))
                .map(str::to_owned)
        })
        .unwrap()
}

const COMPLETE: &str = r#"declarations.begin
(units
  (module create app (as $module)
    (interface create Input (visibility private)
      (operation create read (returns I64) (idempotency idempotent) (external-visibility none)))
    (external create add (visibility private) (implementation core.i64.add) (returns I64)
      (parameter create left (type I64)) (parameter create right (type I64)))
    (constant create answer (as $constant) (visibility private) (type I64) (value (i64 42)))
    (function create read-input (visibility private)
      (requirement-parameter create R (interface Input) (operations Input::read))
      (returns I64) (effect (task (requirement R)))
      (body (capability-call R Input::read)))
    (function create hello (as $hello) (visibility public)
      (parameter create value (as $parameter) (type I64))
      (returns I64) (effect pure)
      (body (call add (local value) (constant answer))))
    (test create arithmetic (as $test) (visibility private)
      (actual (call hello (i64 0))) (expected (i64 42)))
    (component create command (visibility private)
      (requirement create io (interface Input) (operations Input::read) (limits (calls 8 calls)))
      (port create main (as $port) (type (function (I64) I64)) (value (function-value hello)))))
  (target create main (as $target) (component app::command) (runner command) (port app::command::main)))
declarations.end
"#;

#[test]
fn native_command_argument_files_preserve_large_inputs_and_pre_effect_admission() {
    let public = Native::template("command");
    let input = public.input(
        "measure.lkjc",
        &format!(
            r#"request base={}
declarations.begin
(units
  (use std builtin)
  (module create text-input
    (function create measure (visibility private)
      (parameter create value (type Text))
      (returns I64) (effect pure)
      (body (call std::text-length (local value))))
    (component create console (visibility private)
      (port create measure (type (function (Text) I64)) (function measure))))
  (target create measure (component text-input::console) (runner command)
    (port text-input::console::measure)))
declarations.end
"#,
            public.revision()
        ),
    );
    public.apply(&input, &public.plan(&input, true), true);
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let bundle = public.root.path().join("bundle");
    std::fs::create_dir(&bundle).unwrap();
    public.cli(
        &["build", "--output", path(&bundle.join("command.lkja"))],
        true,
    );
    let mut descriptor: Value = serde_json::from_slice(
        &std::fs::read(public.project.join("command.deployment.json")).unwrap(),
    )
    .unwrap();
    descriptor["artifact"] = serde_json::json!("command.lkja");
    descriptor["target"] = serde_json::json!("measure");
    let deployment = bundle.join("command.deployment.json");
    std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    let data = public.root.path().join("arguments.json");
    let routes = [
        vec!["run", "measure"],
        vec!["run", "--deployment", path(&deployment)],
    ];

    // Host code supplies ordinary input data; the native program computes its length.
    for (bytes, expected) in [
        (
            serde_json::to_vec(&vec!["x".repeat(200_000)]).unwrap(),
            "200000",
        ),
        (
            {
                let mut exact = b"[\"ok\"]".to_vec();
                exact.resize(1_048_576, b' ');
                exact
            },
            "2",
        ),
    ] {
        std::fs::write(&data, bytes).unwrap();
        for route in &routes {
            let mut arguments = route.clone();
            // The file is relative to the process directory, not project or descriptor.
            arguments.extend(["--arguments-file", "arguments.json"]);
            let output = public.cli(&arguments, true);
            assert_eq!(
                compact_field(compact_record(&output, "execution"), "value"),
                expected
            );
        }
    }
    for (bytes, code) in [
        (vec![b' '; 1_048_577], "read_limit"),
        (b"[\"unterminated]".to_vec(), "json_decode"),
        (b"[] []".to_vec(), "json_trailing"),
        (b"[{\"a\":1,\"a\":2}]".to_vec(), "json_decode"),
        (vec![b'[', b'"', 0xff, b'"', b']'], "json_decode"),
        (b"[0]".to_vec(), "normalized_json_type"),
    ] {
        std::fs::write(&data, bytes).unwrap();
        for route in &routes {
            let mut arguments = route.clone();
            arguments.extend(["--arguments-file", path(&data)]);
            let output = public.cli(&arguments, false);
            assert_eq!(
                compact_field(compact_record(&output, "diagnostic"), "code"),
                code
            );
        }
    }
    descriptor["secrets"] = serde_json::json!([
        {"name":"must-not-load", "variable":"LKJSCRIPT_ARGUMENT_FILE_ABSENT_SECRET"}
    ]);
    let protected = bundle.join("protected.deployment.json");
    std::fs::write(&protected, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    for (value, code) in [
        ("[0]", "normalized_json_type"),
        ("[\"ok\"]", "secret_missing"),
    ] {
        std::fs::write(&data, value).unwrap();
        let output = public.cli(
            &[
                "run",
                "--deployment",
                path(&protected),
                "--arguments-file",
                path(&data),
            ],
            false,
        );
        assert_eq!(
            compact_field(compact_record(&output, "diagnostic"), "code"),
            code
        );
    }
    assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    std::fs::rename(&public.project, public.root.path().join("retained-project")).unwrap();
    let output = public.cli(
        &[
            "run",
            "--deployment",
            path(&deployment),
            "--arguments-file",
            path(&data),
        ],
        true,
    );
    assert_eq!(
        compact_field(compact_record(&output, "execution"), "value"),
        "2"
    );
    let cleanup: Value = serde_json::from_str(compact_field(
        compact_record(&output, "execution"),
        "cleanup",
    ))
    .unwrap();
    assert_eq!(cleanup["remaining_tasks"], 0);
    assert_eq!(cleanup["cleanup_failures"], serde_json::json!([]));
}

#[test]
fn native_command_result_files_preserve_typed_bytes_limits_and_detached_execution() {
    let public = Native::template("command");
    let input = public.input("result.lkjc", &format!(r#"request base={}
declarations.begin
(units
  (use std builtin)
  (module create results
    (function create double (visibility private)
      (parameter create value (type Text)) (returns Text) (effect pure)
      (body (call std::text-concat (local value) (local value))))
    (function create scalar (visibility private)
      (parameter create value (type F64)) (returns F64) (effect pure)
      (body (local value)))
    (function create invalid (visibility private)
      (returns F64) (effect pure) (body (f64 inf)))
    (component create console (visibility private)
      (port create double (type (function (Text) Text)) (function double))
      (port create scalar (type (function (F64) F64)) (function scalar))
      (port create invalid (type (function () F64)) (function invalid))))
  (target create double (component results::console) (runner command) (port results::console::double))
  (target create scalar (component results::console) (runner command) (port results::console::scalar))
  (target create invalid (component results::console) (runner command) (port results::console::invalid)))
declarations.end
"#, public.revision()));
    public.apply(&input, &public.plan(&input, true), true);
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let bundle = public.root.path().join("bundle");
    std::fs::create_dir(&bundle).unwrap();
    public.cli(
        &["build", "--output", path(&bundle.join("command.lkja"))],
        true,
    );
    let mut descriptor: Value = serde_json::from_slice(
        &std::fs::read(public.project.join("command.deployment.json")).unwrap(),
    )
    .unwrap();
    descriptor["artifact"] = serde_json::json!("command.lkja");
    let deployment = bundle.join("command.deployment.json");
    let arguments_file = public.root.path().join("arguments.json");
    let result_file = public.root.path().join("result.json");
    // Each expected result is fixed independently; numeric text must not be reparsed.
    let cases = [
        (
            "double",
            "[\"猫\\n\"]".to_owned(),
            "\"猫\\n猫\\n\"".to_owned(),
        ),
        (
            "double",
            format!("[\"{}\"]", "x".repeat(100_000)),
            format!("\"{}\"", "x".repeat(200_000)),
        ),
        (
            "double",
            format!("[\"{}\"]", "y".repeat(524_287)),
            format!("\"{}\"", "y".repeat(1_048_574)),
        ),
        ("scalar", "[-0]".to_owned(), "-0.0".to_owned()),
        (
            "scalar",
            "[1.0000000000000002]".to_owned(),
            "1.0000000000000002".to_owned(),
        ),
    ];
    for detached in [false, true] {
        if detached {
            assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
            std::fs::rename(&public.project, public.root.path().join("retained-project")).unwrap();
        }
        for (target, arguments, expected) in &cases {
            descriptor["target"] = serde_json::json!(target);
            std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
            std::fs::write(&arguments_file, arguments).unwrap();
            let mut route = if detached {
                vec!["run", "--deployment", path(&deployment)]
            } else {
                vec!["run", *target]
            };
            route.extend([
                "--arguments-file",
                "arguments.json",
                "--result-file",
                "result.json",
            ]);
            let records = public.cli(&route, true);
            assert_eq!(std::fs::read(&result_file).unwrap(), expected.as_bytes());
            let execution = compact_record(&records, "execution");
            assert!(super::compact_field(execution, "value").is_none());
            assert_eq!(compact_field(execution, "result-file"), path(&result_file));
            let output = compact_record(&records, "output");
            for (key, value) in [
                ("path", path(&result_file)),
                ("visibility", "created"),
                ("durability", "synchronized"),
                ("stage-cleanup", "removed"),
            ] {
                assert_eq!(compact_field(output, key), value);
            }
            assert_eq!(compact_field(output, "bytes"), expected.len().to_string());
            if detached {
                let cleanup: Value =
                    serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
                assert_eq!(cleanup["remaining_tasks"], 0);
                assert_eq!(cleanup["cleanup_failures"], serde_json::json!([]));
            } else {
                assert_eq!(compact_field(execution, "differential"), "equal");
            }
            let failed = public.cli(&route, false);
            assert_eq!(
                compact_field(compact_record(&failed, "diagnostic"), "code"),
                "output_conflict"
            );
            assert_eq!(std::fs::read(&result_file).unwrap(), expected.as_bytes());
            std::fs::remove_file(&result_file).unwrap();
        }
        for (target, arguments, code) in [
            (
                "double",
                format!("[\"{}\"]", "z".repeat(524_288)),
                "normalized_json_output_bytes",
            ),
            ("scalar", "[\"wrong\"]".to_owned(), "normalized_json_type"),
            ("invalid", "[]".to_owned(), "normalized_json_nonfinite"),
        ] {
            descriptor["target"] = serde_json::json!(target);
            std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
            std::fs::write(&arguments_file, arguments).unwrap();
            let mut route = if detached {
                vec!["run", "--deployment", path(&deployment)]
            } else {
                vec!["run", target]
            };
            route.extend([
                "--arguments-file",
                "arguments.json",
                "--result-file",
                "result.json",
            ]);
            let records = public.cli(&route, false);
            assert_eq!(
                compact_field(compact_record(&records, "diagnostic"), "code"),
                code
            );
            assert!(
                !records
                    .iter()
                    .any(|record| record.operation == "execution" || record.operation == "output")
            );
            assert!(!result_file.exists());
        }
    }
    descriptor["target"] = serde_json::json!("scalar");
    descriptor["secrets"] = serde_json::json!([{"name":"must-not-load", "variable":"LKJSCRIPT_RESULT_FILE_ABSENT_SECRET"}]);
    std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    std::fs::write(&result_file, b"preserve").unwrap();
    let route = [
        "run",
        "--deployment",
        path(&deployment),
        "--arguments",
        "[0]",
        "--result-file",
        "result.json",
    ];
    let rejected = public.cli(&route, false);
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        "output_conflict"
    );
    std::fs::remove_file(&result_file).unwrap();
    let rejected = public.cli(&route, false);
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        "secret_missing"
    );
    assert!(!result_file.exists());
    descriptor["secrets"] = serde_json::json!([]);
    std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    // Establish BrokenPipe before spawn; a post-spawn close races with a fast child.
    let (closed_stdout_reader, closed_stdout_writer) = std::io::pipe().unwrap();
    drop(closed_stdout_reader);
    let child = support::spawn(
        Command::new(&public.executable)
            .args([
                "run",
                "--deployment",
                path(&deployment),
                "--arguments",
                "[-0]",
                "--result-file",
                path(&result_file),
            ])
            .current_dir(public.root.path())
            .env_clear()
            .stdout(closed_stdout_writer)
            .stderr(Stdio::piped()),
    )
    .unwrap();
    let failed_delivery = child.wait_with_output().unwrap();
    assert_eq!(failed_delivery.status.code(), Some(6));
    let diagnostic: Value = serde_json::from_slice(&failed_delivery.stderr).unwrap();
    assert!(diagnostic["notes"].as_array().unwrap().iter().any(|note| {
        note.as_str()
            .unwrap()
            .contains("result file is already published")
    }));
    assert_eq!(std::fs::read(&result_file).unwrap(), b"-0.0");
    assert!(
        !std::fs::read_dir(public.root.path())
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("stage-"))
    );
}

#[test]
fn native_command_map_results_count_json_entry_arrays_before_publication() {
    let public = Native::template("command");
    let input = public.input(
        "counts.lkjc",
        &format!(
            r#"request base={}
declarations.begin
(units
  (use std builtin)
  (module create counts
    (function create step (visibility private)
      (parameter create totals (type (map Text I64)))
      (parameter create item (type Text))
      (returns (map Text I64)) (effect pure)
      (body (call std::map-insert (types Text I64) (local totals) (local item)
        (call std::add
          (call std::map-get-or (types Text I64) (local totals) (local item) (i64 0))
          (i64 1)))))
    (function create frequencies (visibility private)
      (parameter create items (type (list Text)))
      (returns (map Text I64)) (effect pure)
      (body (call std::list-fold-left (types Text (map Text I64))
        (local items) (map Text I64) (function-value step))))
    (component create console (visibility private)
      (port create count (type (function ((list Text)) (map Text I64))) (function frequencies))))
  (target create count (component counts::console) (runner command) (port counts::console::count)))
declarations.end
"#,
            public.revision()
        ),
    );
    public.apply(&input, &public.plan(&input, true), true);
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    public.cli(&["build", "--output", "counts.lkja"], true);
    let mut descriptor: Value = serde_json::from_slice(
        &std::fs::read(public.project.join("command.deployment.json")).unwrap(),
    )
    .unwrap();
    descriptor["artifact"] = serde_json::json!("counts.lkja");
    descriptor["target"] = serde_json::json!("count");
    let deployment = public.root.path().join("counts.deployment.json");
    std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    let result = public.root.path().join("result.json");
    for detached in [false, true] {
        if detached {
            std::fs::rename(&public.project, public.root.path().join("retained-project")).unwrap();
        }
        for count in [33_333, 33_334] {
            let keys = (0..count)
                .map(|index| format!("key-{index:05}"))
                .collect::<Vec<_>>();
            let input = serde_json::to_vec(&[&keys]).unwrap();
            public.input("arguments.json", std::str::from_utf8(&input).unwrap());
            let mut route = if detached {
                vec!["run", "--deployment", path(&deployment)]
            } else {
                vec!["run", "count"]
            };
            route.extend([
                "--arguments-file",
                "arguments.json",
                "--result-file",
                "result.json",
            ]);
            let records = public.cli(&route, count == 33_333);
            if count == 33_333 {
                // One outer entry-array member plus its two members: 99,999 items.
                let expected =
                    serde_json::to_vec(&keys.iter().map(|key| (key, 1)).collect::<Vec<_>>())
                        .unwrap();
                assert_eq!(expected.len(), 533_329);
                assert_eq!(std::fs::read(&result).unwrap(), expected);
                lkjscript::platform::json::decode_application(&expected, Default::default())
                    .unwrap();
                std::fs::remove_file(&result).unwrap();
            } else {
                // 100,002 actual JSON items must fail before create-new publication.
                let diagnostic = compact_record(&records, "diagnostic");
                assert_eq!(compact_field(diagnostic, "code"), "normalized_json_type");
                assert!(compact_field(diagnostic, "message").contains("item-count limit"));
                assert!(!result.exists());
                assert!(records.iter().all(|record| record.operation != "execution"));
            }
            assert!(std::fs::read_dir(public.root.path()).unwrap().all(|entry| {
                !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".lkjscript-output-stage-")
            }));
        }
        let project = if detached {
            public.root.path().join("retained-project")
        } else {
            public.project.clone()
        };
        assert_eq!(std::fs::read(project.join("HEAD")).unwrap(), head);
    }
}

#[tokio::test]
async fn result_file_conflict_after_preflight_preserves_the_committed_effect() {
    use lkjscript::platform::{execute_foreground_run, parse_foreground_run};
    let public = Native::template("command");
    let input = public.input("save.lkjc", &format!(r#"request base={}
declarations.begin
(units
  (use std builtin)
  (module create saved-result
    (constant create key (visibility private) (type (list std::DataKeyPart))
      (value (list std::DataKeyPart (variant std::DataKeyPart::Text (text "value")))))
    (function create add-two (visibility private)
      (parameter create value (type I64)) (returns I64) (effect (task))
      (body (call std::add (local value) (i64 2))))
    (function create save (visibility private)
      (returns (std::TransactionOutcome I64)) (effect (task (requirement store::data)))
      (body (call std::data-cell-try-update (types I64) (effects (row)) (requirements store::data)
        (static-text "result") (i64 40) (function-value add-two) (constant key))))
    (function create read (visibility private)
      (returns I64) (effect (task (requirement store::data)))
      (body (call std::data-decode-or (types I64)
        (field (call std::list-get (types std::DataEntry)
          (capability-call store::data std::DataStore::get (static-text "result") (constant key))
          (i64 0)) std::DataEntry::value) (i64 -1))))
    (component create store (visibility private)
      (requirement create data (interface std::DataStore)
        (operations std::DataStore::get std::DataStore::put std::DataStore::transaction std::DataStore::require-transaction)
        (limits (maximum_calls 64 calls)))
      (port create save (type (task-function () (std::TransactionOutcome I64) (row (requirement data)))) (function save))
      (port create read (type (task-function () I64 (row (requirement data)))) (function read))))
  (target create save (component saved-result::store) (runner command) (port saved-result::store::save))
  (target create read (component saved-result::store) (runner command) (port saved-result::store::read)))
declarations.end
"#, public.revision()));
    public.apply(&input, &public.plan(&input, true), true);
    let artifact = public.root.path().join("command.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let data = public.root.path().join("data");
    compact_success_at(
        &public.executable,
        public.root.path(),
        &["data", "initialize", "--root", path(&data)],
    );
    let before = std::fs::read(data.join("HEAD")).unwrap();
    let deployment = public.root.path().join("save.deployment.json");
    let mut descriptor = serde_json::json!({
        "artifact":"command.lkja", "target":"save", "listen":null, "http":null, "session":null, "worker":null,
        "streams":lkjscript::platform::stream::StreamLimits::default(), "configuration":{}, "secrets":[],
        "grants":[{"requirement":"data", "sharing_domain":"saved-result", "authority_revision":"52".repeat(32),
            "adapter":{"kind":"data", "root":"data", "namespace":"saved-result", "limits":lkjscript::platform::data::DataLimits::default()}}]
    });
    std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    std::fs::rename(&public.project, public.root.path().join("retained-project")).unwrap();
    let output = public.root.path().join("result.json");
    // Exercise the actual public preparation/execution boundary deterministically.
    // The competitor appears after CLI preflight; no invocation is replayed.
    let options = parse_foreground_run(
        &[
            "run",
            "--deployment",
            path(&deployment),
            "--result-file",
            path(&output),
        ]
        .map(str::to_owned),
    )
    .unwrap();
    assert!(!output.exists());
    std::fs::write(&output, b"competing writer").unwrap();
    let failure = execute_foreground_run(options, std::future::pending())
        .await
        .unwrap_err();
    assert_eq!(failure.code, "output_conflict");
    assert!(
        failure
            .notes
            .iter()
            .any(|note| note.contains("earlier application effects may already be visible"))
    );
    assert!(failure.notes.iter().any(|note| note
        == "foreground cleanup: admission-stopped=true remaining-owned-tasks=0 failures=0"));
    assert_eq!(std::fs::read(&output).unwrap(), b"competing writer");
    assert_ne!(std::fs::read(data.join("HEAD")).unwrap(), before);
    descriptor["target"] = serde_json::json!("read");
    std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    let read = public.cli(&["run", "--deployment", path(&deployment)], true);
    assert_eq!(
        compact_field(compact_record(&read, "execution"), "value"),
        "42"
    );
    assert!(
        !std::fs::read_dir(public.root.path())
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("stage-"))
    );
}

fn identity(records: &[CompactRecord], symbol: &str) -> String {
    compact_field(
        records
            .iter()
            .find(|record| {
                record.operation == "identity" && compact_field(record, "symbol") == symbol
            })
            .unwrap(),
        "id",
    )
    .to_owned()
}

#[test]
fn native_draft_preserves_http_routes_and_large_canonical_strings() {
    let http = Native::template("http");
    let mut owners = Vec::new();
    for kind in ["module", "target"] {
        owners.extend(
            http.cli(&["query", "owners", "--kind", kind], true)
                .into_iter()
                .filter(|r| r.operation == "owner")
                .map(|r| compact_field(&r, "id").to_owned()),
        );
    }
    let draft = http.root.path().join("http.lkjc");
    let mut args = vec!["change", "draft", "--output", path(&draft)];
    for owner in &owners {
        args.extend(["--owner", owner]);
    }
    http.cli(&args, true);
    let source = std::fs::read_to_string(&draft).unwrap();
    assert!(source.contains("(route edit "));
    assert_eq!(
        compact_field(
            compact_record(&http.plan(&draft, true), "result"),
            "outcome"
        ),
        "unchanged"
    );

    let public = Native::new();
    // Escaped source exceeds the compact physical-line limit while decoded content stays
    // within the existing canonical inline-text contract. Larger content still rejects.
    let too_large = "quoted \\\"value\\\" 日本語 ".repeat(4000);
    let rejected = public.input("oversized-text.lkjc", &format!("request base={}\ndeclarations.begin\n(units (module create large (constant create text (visibility private) (type Text) (value (text \"{too_large}\")))))\ndeclarations.end\n", public.revision()));
    assert_eq!(
        compact_field(
            compact_record(&public.plan(&rejected, false), "diagnostic"),
            "code"
        ),
        "kernel_expression_inline_text_limit"
    );
    let value = "quoted \\\"value\\\" 日本語 ".repeat(2500);
    let input = public.input("large.lkjc", &format!("request base={}\ndeclarations.begin\n(units (module create large (as $m) (constant create text (visibility private) (type Text) (value (text \"{value}\")))))\ndeclarations.end\n", public.revision()));
    let created = public.apply(&input, &public.plan(&input, true), true);
    let draft = public.root.path().join("large-draft.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--owner",
            &identity(&created, "$m"),
            "--output",
            path(&draft),
        ],
        true,
    );
    let source = std::fs::read_to_string(&draft).unwrap();
    assert!(source.contains(&value));
    assert!(source.len() > 65_536);
    assert_eq!(
        compact_field(
            compact_record(&public.plan(&draft, true), "result"),
            "outcome"
        ),
        "unchanged"
    );
}

#[test]
fn native_transaction_outcome_preserves_inline_generic_type_through_drafting() {
    let public = Native::template("command");
    let input = public.input(
        "transaction-outcome.lkjc",
        &format!(
            r#"request base={}
declarations.begin
(units
  (use standard builtin)
  (module create transactions (as $module)
    (function create retain (as $function) (visibility public)
      (type-parameter create T)
      (requirement-parameter create R
        (interface standard::DataStore) (operations standard::DataStore::transaction))
      (parameter create value (type T))
      (returns (standard::TransactionOutcome (list T)))
      (effect (task (requirement R)))
      (body
        (transaction-outcome R (types (list T))
          (outcome standard::TransactionOutcome standard::TransactionAbortReason
            standard::TransactionOutcome::Committed standard::TransactionOutcome::Aborted
            standard::TransactionAbortReason::ConditionFailed standard::TransactionAbortReason::Conflict)
          (binding owner)
          (list T (local value)))))))
declarations.end
"#,
            public.revision()
        ),
    );
    let applied = public.apply(&input, &public.plan(&input, true), true);
    let accepted = public.revision();
    public.cli(&["check"], true);
    for symbol in ["$function", "$module"] {
        let draft = public.root.path().join(format!("{symbol}.lkjc"));
        public.cli(
            &[
                "change",
                "draft",
                "--owner",
                &identity(&applied, symbol),
                "--output",
                path(&draft),
            ],
            true,
        );
        assert_eq!(
            compact_field(
                compact_record(&public.plan(&draft, true), "result"),
                "outcome"
            ),
            "unchanged"
        );
    }
    assert_eq!(public.revision(), accepted);
}

#[test]
fn native_draft_aliases_cannot_shadow_signature_or_declaration_names() {
    let public = Native::new();
    let input = public.input(
        "alias-names.lkjc",
        &format!(
            r#"request base={}
declarations.begin
(units
  (module create aliases (as $module)
    (function create keep (as $keep) (visibility public)
      (type-parameter create type_0)
      (type-parameter create type_1)
      (parameter create values (type (list I64)))
      (returns (list I64)) (effect pure) (body (local values)))
    (function create foo (visibility private)
      (returns I64) (effect pure) (body (i64 7)))
    (function create ref_foo_0 (as $reference) (visibility public)
      (returns I64) (effect pure) (body (call foo)))
    (test create expected (visibility private)
      (actual (call ref_foo_0)) (expected (i64 7)))))
declarations.end
"#,
            public.revision()
        ),
    );
    let applied = public.apply(&input, &public.plan(&input, true), true);
    let accepted = public.revision();
    public.cli(&["check"], true);
    for symbol in ["$keep", "$reference", "$module"] {
        let owner = identity(&applied, symbol);
        let draft = public.root.path().join(format!("{owner}.lkjc"));
        public.cli(
            &[
                "change",
                "draft",
                "--owner",
                &owner,
                "--output",
                path(&draft),
            ],
            true,
        );
        assert_eq!(
            compact_field(
                compact_record(&public.plan(&draft, true), "result"),
                "outcome"
            ),
            "unchanged",
            "{symbol}"
        );
    }
    assert_eq!(public.revision(), accepted);
}

#[test]
fn native_lexical_names_may_begin_with_exact_owner_prefixes() {
    let public = Native::new();
    let input = public.input(
        "owner-prefix-names.lkjc",
        &format!(
            r#"request base={}
declarations.begin
(units
  (module create mod_values (as $module)
    (function create decl_keep (visibility public)
      (parameter create param_value (type I64))
      (returns I64) (effect pure) (body (local param_value)))
    (test create decl_expected (visibility private)
      (actual (call decl_keep (i64 7))) (expected (i64 7)))))
declarations.end
"#,
            public.revision()
        ),
    );
    let applied = public.apply(&input, &public.plan(&input, true), true);
    public.cli(&["check"], true);
    let draft = public.root.path().join("prefix-draft.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--owner",
            &identity(&applied, "$module"),
            "--output",
            path(&draft),
        ],
        true,
    );
    assert_eq!(
        compact_field(
            compact_record(&public.plan(&draft, true), "result"),
            "outcome"
        ),
        "unchanged"
    );
}

#[test]
fn native_input_admission_has_an_exact_boundary_without_publication() {
    let public = Native::new();
    let before = public.revision();
    let mut source =
        format!("request base={before}\ndeclarations.begin\n(units (module create admitted))\n;");
    let ending = "\ndeclarations.end\n";
    source.extend(std::iter::repeat_n(
        ' ',
        4 * 1_048_576 - source.len() - ending.len(),
    ));
    source.push_str(ending);
    let exact = public.input("exact-input.lkjc", &source);
    public.plan(&exact, true);
    source.push('\n');
    let oversized = public.input("oversized-input.lkjc", &source);
    let rejected = public.plan(&oversized, false);
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        "read_limit"
    );
    assert_eq!(public.revision(), before);
}

#[test]
fn native_scopes_and_task_kind_reject_before_publication() {
    let public = Native::new();
    let before = public.revision();
    for (case, body, expected) in [
        (
            "generic-escape",
            "(units (module create app (function create identity (visibility public) (type-parameter create T) (parameter create value (type T)) (returns T) (effect pure) (body (local value))) (function create escaped (visibility private) (parameter create value (type T)) (returns T) (effect pure) (body (local value)))))",
            "change_unit_unresolved",
        ),
        (
            "pure-task",
            "(units (module create app (function create task (visibility private) (returns I64) (effect (task)) (body (i64 1))) (function create pure (visibility public) (returns I64) (effect pure) (body (call task)))))",
            "kernel_type_pure_task_call",
        ),
        (
            "ambiguous-scope",
            "(units (module create first (record create Item (visibility public))) (module create second (record create Item (visibility public))) (module create consumer (function create identity (visibility public) (parameter create value (type Item)) (returns Item) (effect pure) (body (local value)))))",
            "change_unit_unresolved",
        ),
    ] {
        let input = public.input(
            &format!("{case}.lkjc"),
            &format!("request base={before}\ndeclarations.begin\n{body}\ndeclarations.end\n"),
        );
        let rejected = public.plan(&input, false);
        assert!(
            rejected
                .iter()
                .any(|r| r.operation == "diagnostic" && compact_field(r, "code") == expected),
            "{rejected:?}"
        );
        assert_eq!(public.revision(), before);
    }
}

#[test]
fn native_declarations_create_resume_edit_and_reject_authority_changes() {
    let public = Native::new();
    let base = public.revision();
    let input = public.input(
        "create.lkjc",
        &format!("request base={base} idempotency=native-create\n{COMPLETE}"),
    );
    let plan = public.plan(&input, true);
    let applied = public.apply(&input, &plan, true);
    let accepted = public.revision();
    assert_ne!(base, accepted);
    let retried = public.apply(&input, &plan, true);
    assert_eq!(
        compact_field(compact_record(&applied, "revision"), "result"),
        compact_field(compact_record(&retried, "revision"), "result")
    );
    public.cli(&["check"], true);
    let module = identity(&applied, "$module");
    let target = identity(&applied, "$target");
    let hello = identity(&applied, "$hello");
    let parameter = identity(&applied, "$parameter");
    let draft_path = public.root.path().join("edit.lkjc");
    std::fs::remove_file(&input).unwrap();
    public.cli(
        &[
            "change",
            "draft",
            "--owner",
            &module,
            "--owner",
            &target,
            "--output",
            path(&draft_path),
        ],
        true,
    );
    let draft = std::fs::read_to_string(&draft_path).unwrap();
    let noop = public.plan(&draft_path, true);
    assert_eq!(
        compact_field(compact_record(&noop, "result"), "outcome"),
        "unchanged"
    );
    assert!(draft.contains(&format!("function edit {hello} hello")));
    assert!(draft.contains(&format!("parameter edit {parameter} value")));

    // Capacity is charged before destination publication; exact fit succeeds and one less fails.
    let exact = public.root.path().join("exact.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--owner",
            &target,
            "--owner",
            &module,
            "--bytes",
            &draft.len().to_string(),
            "--output",
            path(&exact),
        ],
        true,
    );
    assert_eq!(std::fs::read(&exact).unwrap(), draft.as_bytes());
    let rejected = public.root.path().join("too-small.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--owner",
            &module,
            "--owner",
            &target,
            "--bytes",
            &(draft.len() - 1).to_string(),
            "--output",
            path(&rejected),
        ],
        false,
    );
    assert!(!rejected.exists());
    public.cli(
        &[
            "change",
            "draft",
            "--owner",
            &module,
            "--output",
            path(&draft_path),
        ],
        false,
    );
    assert_eq!(std::fs::read_to_string(&draft_path).unwrap(), draft);

    for (name, bad) in [
        ("stale", draft.replace(&accepted, &base)),
        ("renamed", draft.replace(&format!("function edit {hello} hello"), &format!("function edit {hello} other"))),
        ("omitted", draft.replace(&format!("(parameter edit {parameter} value\n        (type I64)\n        (use unrestricted))"), "")),
        ("wrong-repository", draft.replace(draft.split_whitespace().find(|s| s.starts_with("repository=")).unwrap(), "repository=repo_11111111111111111111111111111111")),
        ("wrong-package", draft.replace(draft.split_whitespace().find(|s| s.starts_with("package=")).unwrap(), "package=pkg_11111111111111111111111111111111")),
    ] {
        assert_ne!(bad, draft, "negative fixture did not change its target");
        let input = public.input(&format!("{name}.lkjc"), &bad);
        public.plan(&input, false);
        assert_eq!(public.revision(), accepted);
    }

    // Edit a constant, its graph expectation and a port value without owner recreation.
    let mut edited = draft
        .replace("(value (i64 42))", "(value (i64 43))")
        .replace("(expected (i64 42))", "(expected (i64 43))");
    let port = form_span(
        &edited,
        &format!("(port edit {} main", identity(&applied, "$port")),
    );
    let mut port_unit = edited[port.clone()].to_owned();
    let value = form_span(&port_unit, "(value ");
    port_unit.replace_range(
        value,
        "(value (if (bool true) (function-value hello) (function-value hello)))",
    );
    edited.replace_range(port, &port_unit);
    edited = edited.replacen('\n', " idempotency=native-edit\n", 1);
    let edit = public.input("changed.lkjc", &edited);
    let plan = public.plan(&edit, true);
    let tampered = public.input("tampered.lkjc", &edited.replace("(i64 43)", "(i64 44)"));
    public.apply(&tampered, &plan, false);
    assert_eq!(public.revision(), accepted);
    let accepted_edit = public.apply(&edit, &plan, true);
    let retried_edit = public.apply(&edit, &plan, true);
    assert_eq!(
        compact_field(compact_record(&accepted_edit, "revision"), "result"),
        compact_field(compact_record(&retried_edit, "revision"), "result")
    );
    public.cli(&["check"], true);
    let definition = public.cli(
        &[
            "inspect",
            "owner",
            "pure_function",
            &hello,
            "--detail",
            "definition",
            "--limit",
            "1000",
            "--bytes",
            "1048576",
        ],
        true,
    );
    assert_eq!(
        compact_field(compact_record(&definition, "definition.function"), "id"),
        hello
    );
    assert_eq!(
        compact_field(compact_record(&definition, "definition.parameter"), "id"),
        parameter
    );
}

#[test]
fn native_lkjournal_draft_preserves_independent_canonical_signatures() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("lkjscript");
    copy_executable(&binary(), &executable);
    let project = root.path().join("lkjournal");
    copy_regular_tree(Path::new(APPLICATION), &project);
    let public = Native {
        root,
        executable,
        project,
    };
    let before = public.revision();
    let selections = [
        (
            "decl_0693166bd7c29bee83d2ead289148f65",
            "update-resource",
            vec!["request", "id"],
            "3",
        ),
        (
            "decl_53936ef7d46ee491d41aef8c37cdffef",
            "commit-resource-update",
            vec!["actor", "id", "input", "entry"],
            "1",
        ),
    ];
    let mut originals = Vec::new();
    for (owner, name, parameters, requirements) in &selections {
        let definition = public.cli(
            &[
                "inspect",
                "owner",
                "task_function",
                owner,
                "--detail",
                "definition",
                "--limit",
                "1000",
                "--bytes",
                "1048576",
            ],
            true,
        );
        assert_eq!(
            compact_field(compact_record(&definition, "page"), "complete"),
            "true"
        );
        let function = compact_record(&definition, "definition.function");
        assert_eq!(compact_field(function, "name"), *name);
        assert_eq!(compact_field(function, "requirements"), *requirements);
        let names: Vec<_> = definition
            .iter()
            .filter(|r| r.operation == "definition.parameter")
            .map(|r| compact_field(r, "name"))
            .collect();
        assert_eq!(&names, parameters);
        originals.push(definition);
    }
    let draft = public.root.path().join("lkjournal.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--owner",
            selections[0].0,
            "--owner",
            selections[1].0,
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
    assert_eq!(public.revision(), before);
    for ((owner, _, _, _), original) in selections.iter().zip(originals) {
        let after = public.cli(
            &[
                "inspect",
                "owner",
                "task_function",
                owner,
                "--detail",
                "definition",
                "--limit",
                "1000",
                "--bytes",
                "1048576",
            ],
            true,
        );
        assert_eq!(original, after);
    }
}

#[test]
fn native_generic_library_and_nominal_consumer_use_exact_transport_and_canonical_reentry() {
    let library = Native::new();
    let builtin = library.cli(&["package", "builtin", "inspect"], true);
    let builtin = compact_record(&builtin, "package");
    let standard = library.root.path().join("standard.lkjp");
    library.cli(
        &[
            "package",
            "builtin",
            "export",
            "--kind",
            "transport",
            "--output",
            path(&standard),
        ],
        true,
    );
    library.cli(
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            compact_field(builtin, "transport"),
            "--input-file",
            path(&standard),
        ],
        true,
    );
    let input = library.input("library.lkjc", &format!(
        "request base={}\nadd.dependency package={} semantic-revision={} package-revision={}\n{}",
        library.revision(), compact_field(builtin, "id"), compact_field(builtin, "revision"), compact_field(builtin, "package-revision"), include_str!("../fixtures/native-library.lkjc")));
    let plan = library.plan(&input, true);
    let created = library.apply(&input, &plan, true);
    let check = library.cli(&["check"], true);
    assert_eq!(
        compact_field(compact_record(&check, "tests"), "failed"),
        "0"
    );
    let transport = library.root.path().join("library.lkjp");
    let exported = library.cli(
        &[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            path(&transport),
        ],
        true,
    );
    let exported = compact_record(&exported, "package");
    let consumer = Native::new();
    consumer.cli(
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            compact_field(exported, "transport"),
            "--input-file",
            path(&transport),
        ],
        true,
    );
    let consumer_input = consumer.input("consumer.lkjc", &format!(
        "request base={}\nadd.dependency package={} semantic-revision={} package-revision={}\ndeclarations.begin\n(units (use lib {} {}))\ndeclarations.end\n{}",
        consumer.revision(), compact_field(exported, "id"), compact_field(exported, "revision"), compact_field(exported, "package-revision"), compact_field(exported, "id"), compact_field(exported, "package-revision"), include_str!("../fixtures/native-consumer.lkjc")));
    let plan = consumer.plan(&consumer_input, true);
    let consumer_created = consumer.apply(&consumer_input, &plan, true);
    let accepted_consumer = consumer.revision();
    let private = consumer.input("private-dependency.lkjc", &format!(
        "request base={accepted_consumer}\ndeclarations.begin\n(units (use lib {} {}) (module create invalid (function create private-call (visibility private) (returns I64) (effect pure) (body (call lib::step)))))\ndeclarations.end\n",
        compact_field(exported, "id"), compact_field(exported, "package-revision")));
    let private_rejected = consumer.plan(&private, false);
    assert!(private_rejected.iter().any(|r| r.operation == "diagnostic"
        && compact_field(r, "code") == "change_reference_not_exposed"));
    assert_eq!(consumer.revision(), accepted_consumer);
    consumer.cli(&["check"], true);
    let output = consumer.cli(&["run", "summary", "--arguments", "[]"], true);
    let execution = compact_record(&output, "execution");
    assert_eq!(compact_field(execution, "differential"), "equal");
    assert_eq!(
        serde_json::from_str::<Value>(compact_field(execution, "value")).unwrap(),
        serde_json::json!({"count":2,"sum":2})
    );
    let overflow = consumer.cli(&["run", "overflow", "--arguments", "[]"], false);
    assert_eq!(
        compact_field(compact_record(&overflow, "diagnostic"), "code"),
        "normalized_integer_overflow"
    );
    std::fs::remove_file(input).unwrap();
    std::fs::remove_file(consumer_input).unwrap();
    let library_draft = library.root.path().join("draft.lkjc");
    library.cli(
        &[
            "change",
            "draft",
            "--owner",
            &identity(&created, "$batches"),
            "--output",
            path(&library_draft),
        ],
        true,
    );
    let consumer_draft = consumer.root.path().join("draft.lkjc");
    consumer.cli(
        &[
            "change",
            "draft",
            "--owner",
            &identity(&consumer_created, "$readings"),
            "--owner",
            &identity(&consumer_created, "$app"),
            "--output",
            path(&consumer_draft),
        ],
        true,
    );
    for (public, draft) in [(&library, &library_draft), (&consumer, &consumer_draft)] {
        let before = public.revision();
        let plan = public.plan(draft, true);
        assert_eq!(
            compact_field(compact_record(&plan, "result"), "outcome"),
            "unchanged"
        );
        assert_eq!(public.revision(), before);
    }
    let signature = library.cli(
        &[
            "inspect",
            "owner",
            "pure_function",
            &identity(&created, "$summarize"),
            "--detail",
            "definition",
            "--limit",
            "1000",
            "--bytes",
            "1048576",
        ],
        true,
    );
    let names: BTreeSet<_> = signature
        .iter()
        .filter(|record| record.operation == "definition.parameter")
        .map(|record| compact_field(record, "name"))
        .collect();
    assert_eq!(names, BTreeSet::from(["batch", "projection"]));

    let old_bundle = consumer.root.path().join("old.lkja");
    consumer.cli(&["build", "--output", path(&old_bundle)], true);
    let mut edit = std::fs::read_to_string(&library_draft).unwrap();
    let function = format!(
        "(function edit {} summarize",
        identity(&created, "$summarize")
    );
    let span = form_span(&edit, &function);
    let mut changed = edit[span.clone()].to_owned();
    let body = form_span(&changed, "(body ");
    changed.replace_range(
        body,
        r#"(body
      (call std::list-fold-left (types T Summary)
        (field (local batch) Batch::items)
        (record Summary (field Summary::count (i64 0)) (field Summary::sum (i64 0)))
        (bind (function-value step-with-policy (types T))
          (local projection) (local include_excluded))))"#,
    );
    changed.insert_str(
        changed.len() - 1,
        "\n(parameter create include_excluded (type Bool))",
    );
    edit.replace_range(span, &changed);
    let summarize = draft_reference(&edit, "summarize");
    insert_argument(&mut edit, &summarize, " (bool false)");
    let module = form_span(
        &edit,
        &format!("(module edit {} batches", identity(&created, "$batches")),
    );
    edit.insert_str(
        module.end - 1,
        include_str!("../fixtures/native-library-evolution.lkjc"),
    );
    edit = edit.replacen("(units\n", "(units\n(use std builtin)\n", 1);
    let edit_input = library.input("library-evolve.lkjc", &edit);
    library.apply(&edit_input, &library.plan(&edit_input, true), true);
    library.cli(&["check"], true);
    let evolved_signature = library.cli(
        &[
            "inspect",
            "owner",
            "pure_function",
            &identity(&created, "$summarize"),
            "--detail",
            "definition",
            "--limit",
            "1000",
            "--bytes",
            "1048576",
        ],
        true,
    );
    for before in signature
        .iter()
        .filter(|r| r.operation == "definition.parameter")
    {
        let after = evolved_signature
            .iter()
            .find(|r| {
                r.operation == "definition.parameter"
                    && compact_field(r, "name") == compact_field(before, "name")
            })
            .unwrap();
        assert_eq!(compact_field(before, "id"), compact_field(after, "id"));
    }
    assert_eq!(
        evolved_signature
            .iter()
            .filter(|r| r.operation == "definition.parameter")
            .count(),
        3
    );
    let new_transport = library.root.path().join("new-library.lkjp");
    let new_export = library.cli(
        &[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            path(&new_transport),
        ],
        true,
    );
    let new_package = compact_record(&new_export, "package");
    consumer.cli(
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            compact_field(new_package, "transport"),
            "--input-file",
            path(&new_transport),
        ],
        true,
    );
    let dependency = format!(
        "replace.dependency package={} semantic-revision={} package-revision={}\n",
        compact_field(new_package, "id"),
        compact_field(new_package, "revision"),
        compact_field(new_package, "package-revision")
    );
    let before = consumer.revision();
    let incomplete = consumer.input(
        "dependency-only.lkjc",
        &format!("request base={before}\n{dependency}"),
    );
    let rejected = consumer.plan(&incomplete, false);
    assert!(rejected.iter().any(
        |r| r.operation == "diagnostic" && compact_field(r, "code") == "kernel_type_call_arity"
    ));
    assert_eq!(consumer.revision(), before);

    let mut edit = std::fs::read_to_string(&consumer_draft).unwrap();
    let summarize = draft_reference(&edit, "summarize");
    insert_argument(&mut edit, &summarize, " (bool true)");
    for (name, count, sum) in [("excluded", 1, 100), ("mixed", 3, 102)] {
        let heading = edit
            .lines()
            .map(str::trim)
            .find(|line| line.starts_with("(test edit ") && line.ends_with(&format!(" {name}")))
            .unwrap()
            .to_owned();
        let span = form_span(&edit, &heading);
        let mut test = edit[span.clone()].to_owned();
        let expected = form_span(&test, "(expected ");
        test.replace_range(expected, &format!("(expected (record lib::Summary (field lib::Summary::count (i64 {count})) (field lib::Summary::sum (i64 {sum}))))"));
        edit.replace_range(span, &test);
    }
    edit.insert_str(edit.find('\n').unwrap() + 1, &dependency);
    edit = edit.replacen(
        "(units\n",
        &format!(
            "(units\n(use lib {} {})\n",
            compact_field(new_package, "id"),
            compact_field(new_package, "package-revision")
        ),
        1,
    );
    let repaired = consumer.input("consumer-evolve.lkjc", &edit);
    consumer.apply(&repaired, &consumer.plan(&repaired, true), true);
    consumer.cli(&["check"], true);
    let new_bundle = consumer.root.path().join("new.lkja");
    consumer.cli(&["build", "--output", path(&new_bundle)], true);

    // Only this disposable fixture owns these projects. Detached runs cannot consult them.
    std::fs::remove_dir_all(&library.project).unwrap();
    std::fs::remove_dir_all(&consumer.project).unwrap();
    for (name, expected) in [
        ("old", serde_json::json!({"count":2,"sum":2})),
        ("new", serde_json::json!({"count":3,"sum":102})),
    ] {
        let deployment = consumer.input(&format!("{name}.json"), &serde_json::json!({
            "artifact":format!("{name}.lkja"), "target":"summary", "listen":null,
            "http":null, "session":null, "worker":null,
            "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":67108864,"maximum_live_streams":1024},
            "configuration":{}, "secrets":[], "grants":[]
        }).to_string());
        let result = consumer.cli(&["run", "--deployment", path(&deployment)], true);
        let execution = compact_record(&result, "execution");
        assert_eq!(
            serde_json::from_str::<Value>(compact_field(execution, "value")).unwrap(),
            expected
        );
    }
}

#[path = "native_bytes.rs"]
mod native_bytes;

#[path = "native_text.rs"]
mod native_text;

#[cfg(target_os = "linux")]
#[path = "resident_policy.rs"]
mod resident_policy;

#[path = "native_byte_conversion.rs"]
mod native_byte_conversion;

#[path = "native_forms.rs"]
mod native_forms;

#[path = "native_editor.rs"]
mod native_editor;

#[path = "native_http.rs"]
mod native_http;

#[path = "web_starter.rs"]
mod web_starter;
