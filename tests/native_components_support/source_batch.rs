//! Public source differential batches, independently expected and source-bound.
use super::batch::{BatchCase, Expected, batch_operation, decode_rows, host_operation};
use super::*;

impl Native {
    pub fn source_batch(
        &self,
        label: &str,
        project: &Path,
        target: &str,
        cases: &[BatchCase],
    ) -> Vec<String> {
        execute_in(
            self.mode,
            &self.root,
            &self.root.join("lkjscript"),
            label,
            project,
            target,
            cases,
        )
    }
}

struct Binding {
    authority: BTreeMap<&'static str, String>,
    artifact: BTreeMap<&'static str, String>,
}

impl Binding {
    fn from_output(stdout: &str) -> Self {
        Self {
            authority: ["repository", "package", "revision", "state"]
                .into_iter()
                .map(|key| (key, field(stdout, "authority", key).to_owned()))
                .collect(),
            artifact: ["manifest", "bundle", "bytes", "packages", "closure-objects"]
                .into_iter()
                .map(|key| (key, field(stdout, "artifact", key).to_owned()))
                .collect(),
        }
    }

    fn require(&self, stdout: &str) {
        for (key, expected) in &self.authority {
            assert_eq!(field(stdout, "authority", key), expected, "source {key}");
        }
        for (key, expected) in &self.artifact {
            assert_eq!(field(stdout, "artifact", key), expected, "artifact {key}");
        }
    }
}

/// Explicit executable callers retain the real process boundary.
pub fn execute(
    root: &Path,
    executable: &Path,
    label: &str,
    project: &Path,
    target: &str,
    cases: &[BatchCase],
) -> Vec<String> {
    execute_in(
        semantic_cli::Mode::CandidateProcess,
        root,
        executable,
        label,
        project,
        target,
        cases,
    )
}

/// An ordinary run and package export witness bind the batch to this exact source.
/// Host calls retain the public differential owner; candidate calls execute the selected bytes.
pub fn execute_in(
    mode: semantic_cli::Mode,
    root: &Path,
    executable: &Path,
    label: &str,
    project: &Path,
    target: &str,
    cases: &[BatchCase],
) -> Vec<String> {
    assert!(project.is_absolute());
    assert!(!cases.is_empty());
    let head_path = project.join("HEAD");
    let head = fs::read(&head_path).unwrap();
    let candidate_digest = digest(executable);
    let stable = || {
        assert_eq!(
            fs::read(&head_path).unwrap(),
            head,
            "exact source HEAD bytes"
        );
        assert_eq!(
            digest(executable),
            candidate_digest,
            "exact selected executable"
        );
    };

    let smoke = cases
        .iter()
        .find(|case| matches!(&case.expected, Expected::Success(_)))
        .unwrap_or_else(|| {
            panic!("source batch retains an independently expected ordinary boundary smoke")
        });
    let smoke_target = smoke.target.as_deref().unwrap_or(target);
    let arguments_path = root.join(format!("source-batch-{label}.smoke.arguments.json"));
    let smoke_result = root.join(format!("source-batch-{label}.smoke.result.json"));
    assert!(!arguments_path.exists() && !smoke_result.exists());
    fs::write(&arguments_path, &smoke.arguments_json).unwrap();
    let ordinary = batch_operation(
        mode,
        root,
        executable,
        &format!("source-{label}-ordinary"),
        &[
            "--project",
            path(project),
            "run",
            smoke_target,
            "--arguments-file",
            path(&arguments_path),
            "--result-file",
            path(&smoke_result),
        ],
        true,
    );
    assert_eq!(field(&ordinary, "result", "command"), "run");
    assert_eq!(field(&ordinary, "execution", "target"), smoke_target);
    assert_eq!(field(&ordinary, "execution", "differential"), "equal");
    let Expected::Success(expected) = &smoke.expected else {
        unreachable!()
    };
    assert_eq!(
        &serde_json::from_slice::<Value>(&fs::read(&smoke_result).unwrap()).unwrap(),
        expected
    );
    assert_eq!(
        fs::read(&arguments_path).unwrap(),
        smoke.arguments_json.as_bytes()
    );
    let binding = Binding::from_output(&ordinary);
    stable();

    let exported_path = root.join(format!("source-batch-{label}.witness.lkjp"));
    assert!(!exported_path.exists());
    let exported = batch_operation(
        mode,
        root,
        executable,
        &format!("source-{label}-package"),
        &[
            "--project",
            path(project),
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            path(&exported_path),
        ],
        true,
    );
    assert_eq!(
        field(&exported, "package", "id"),
        binding.authority["package"]
    );
    assert_eq!(
        field(&exported, "package", "revision"),
        binding.authority["revision"]
    );
    assert!(!fs::read(&exported_path).unwrap().is_empty());
    let exported_digest = digest(&exported_path);
    stable();

    let request_path = root.join(format!("source-batch-{label}.input.jsonl"));
    let result_path = root.join(format!("source-batch-{label}.result.jsonl"));
    assert!(!request_path.exists() && !result_path.exists());
    let mut requests = String::new();
    for case in cases {
        requests.push_str(&case.request().to_string());
        requests.push('\n');
    }
    fs::write(&request_path, &requests).unwrap();
    let summary = batch_operation(
        mode,
        root,
        executable,
        &format!("source-{label}"),
        &[
            "--project",
            path(project),
            "run",
            target,
            "--batch-input-file",
            path(&request_path),
            "--batch-result-file",
            path(&result_path),
        ],
        true,
    );
    assert_eq!(field(&summary, "result", "status"), "success");
    assert_eq!(field(&summary, "result", "command"), "run.batch");
    binding.require(&summary);
    assert_eq!(field(&summary, "batch", "mode"), "source-differential");
    assert_eq!(field(&summary, "batch", "source-preparations"), "1");
    assert_eq!(
        field(&summary, "batch", "package-revision"),
        field(&exported, "package", "package-revision")
    );
    assert_eq!(
        field(&summary, "batch", "schema"),
        lkjscript::platform::canonical_schema::CANONICAL_SCHEMA_HASH_HEX
    );
    let validator = lkjscript::platform::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(field(&summary, "batch", "validator"), validator);
    assert_eq!(field(&summary, "batch", "result-file"), path(&result_path));
    assert_eq!(field(&summary, "batch", "rows"), cases.len().to_string());
    let passed = cases
        .iter()
        .filter(|case| matches!(&case.expected, Expected::Success(_)))
        .count();
    assert_eq!(field(&summary, "batch", "passed"), passed.to_string());
    assert_eq!(
        field(&summary, "batch", "failed"),
        (cases.len() - passed).to_string()
    );
    let bytes = fs::read(&result_path).unwrap();
    assert!(bytes.len() <= 64 * 1024 * 1024);
    assert_eq!(bytes.last(), Some(&b'\n'));
    assert_eq!(field(&summary, "output", "bytes"), bytes.len().to_string());
    let outputs = String::from_utf8(bytes).unwrap();
    let rows = decode_rows(&outputs);
    assert_eq!(rows.len(), cases.len(), "complete source row inventory");
    let mut seen = BTreeSet::new();
    let mut observed = Vec::with_capacity(rows.len());
    for (row, case) in rows.iter().zip(cases) {
        assert_eq!(
            row.as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["id", "exit_code", "stdout", "result_json"])
        );
        assert_eq!(row["id"].as_str(), Some(case.id.as_str()));
        assert!(seen.insert(&case.id), "unique source row identity");
        let stdout = row["stdout"].as_str().unwrap();
        // Source differential execution owns VM/reference/executor state, not a resident taskgroup.
        assert!(!stdout.contains(" cleanup="));
        assert!(!stdout.contains("foreground cleanup:"));
        match &case.expected {
            Expected::Success(expected) => {
                assert_eq!(row["exit_code"], 0, "{}: {stdout}", case.id);
                assert_eq!(field(stdout, "result", "status"), "success");
                assert_eq!(field(stdout, "result", "command"), "run");
                assert_eq!(
                    field(stdout, "execution", "target"),
                    case.target.as_deref().unwrap_or(target),
                    "{}: exact selected target",
                    case.id
                );
                assert_eq!(field(stdout, "execution", "differential"), "equal");
                assert_eq!(
                    &serde_json::from_str::<Value>(row["result_json"].as_str().unwrap()).unwrap(),
                    expected,
                    "{}",
                    case.id
                );
                assert_eq!(
                    field(stdout, "execution", "value-location"),
                    "batch-row",
                    "{}: complete result is carried by result_json",
                    case.id
                );
                binding.require(stdout);
                let production = quoted(stdout, "production-observation");
                let reference = quoted(stdout, "reference-observation");
                require_observation(&production, false, &case.id);
                require_observation(&reference, true, &case.id);
                require_executor(&quoted(stdout, "executor-observation"), &case.id);
                assert_eq!(
                    field(stdout, "execution", "production-instructions"),
                    production["instructions"].to_string()
                );
                assert_eq!(
                    field(stdout, "execution", "reference-expressions"),
                    reference["expressions"].to_string()
                );
            }
            Expected::Failure(class, code) => {
                assert_ne!(row["exit_code"], 0, "{}", case.id);
                assert_eq!(field(stdout, "result", "status"), "failure");
                assert_eq!(field(stdout, "diagnostic", "class"), *class, "{}", case.id);
                assert_eq!(field(stdout, "diagnostic", "code"), *code, "{}", case.id);
                assert!(row["result_json"].is_null());
                require_failure_observations(stdout, code, &case.id, case.deadline_milliseconds);
            }
        }
        observed.push(stdout.to_owned());
    }
    assert_eq!(fs::read(&request_path).unwrap(), requests.as_bytes());
    assert_eq!(digest(&exported_path), exported_digest);
    stable();
    // The export is a complete source transport. Keep its actual command evidence and digest,
    // then remove its bytes before callers prove detached execution without authoring sources.
    fs::write(
        root.join(format!("source-batch-{label}.witness.identity.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "path": exported_path,
            "sha256": exported_digest,
            "package_revision": field(&exported, "package", "package-revision"),
            "execution_mode": mode.label(),
            "boundary_executable_sha256": candidate_digest,
            "source_head_sha256": Sha256::digest(&head).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
            "removed_before_detached_execution": true,
        })).unwrap(),
    ).unwrap();
    fs::remove_file(&exported_path).unwrap();
    assert!(!exported_path.exists());
    observed
}

fn require_observation(observation: &Value, reference: bool, id: &str) {
    assert!(observation.is_object(), "{id}: actual observation object");
    let fields: &[&str] = if reference {
        &[
            "live_call_frames_after",
            "live_control_frames_after",
            "live_local_scopes_after",
            "live_type_scopes_after",
            "live_effect_scopes_after",
            "live_allowances_after",
            "live_transactions_after",
            "live_handles_after",
        ]
    } else {
        &[
            "live_call_frames_after",
            "live_locals_after",
            "live_type_bindings_after",
            "live_operands_after",
            "live_transactions_after",
            "live_handles_after",
        ]
    };
    for name in fields {
        assert_eq!(observation[name], 0, "{id}: unreleased {name}");
    }
    assert_eq!(observation["capability_calls"], 0, "{id}: pure execution");
}

fn require_executor(executor: &Value, id: &str) {
    assert_eq!(executor["dispatch_open"], false, "{id}: dispatch closed");
    assert_eq!(executor["active_dispatches"], 0, "{id}: no active dispatch");
    assert_eq!(executor["remaining_workers"], 0, "{id}: all workers joined");
}

fn require_failure_observations(stdout: &str, code: &str, id: &str, deadline: Option<u64>) {
    let notes = if stdout.contains(" notes=") {
        quoted(stdout, "notes")
    } else {
        serde_json::json!([])
    };
    let notes = notes.as_array().unwrap();
    let mut names = BTreeSet::new();
    for note in notes {
        let note = note.as_str().unwrap();
        for (name, reference) in [
            ("production-observation", false),
            ("reference-observation", true),
        ] {
            if let Some(json) = note.strip_prefix(&format!("{name}: ")) {
                assert!(names.insert(name), "{id}: one actual {name}");
                require_observation(&serde_json::from_str(json).unwrap(), reference, id);
            }
        }
        if let Some(json) = note.strip_prefix("executor-observation: ") {
            assert!(names.insert("executor-observation"));
            require_executor(&serde_json::from_str(json).unwrap(), id);
        }
    }
    if code == "normalized_instruction_steps" {
        assert!(
            names.contains("production-observation"),
            "{id}: failed VM was observed"
        );
        assert!(
            names.contains("executor-observation"),
            "{id}: failed executor was joined"
        );
    }
    if matches!(
        code,
        "normalized_json_type" | "json_decode" | "normalized_runner_grants_required"
    ) || (code == "execution_deadline" && deadline == Some(0))
    {
        assert!(
            names.is_empty(),
            "{id}: uninvoked row has no invented observations"
        );
    }
}

/// Once per selected boundary matrix: setup and controls all use the actual candidate.
pub fn controls(candidate: &Path) {
    controls_in(semantic_cli::Mode::CandidateProcess, candidate);
}

/// Preserve the complete refusal/recovery matrix in host mode without claiming process evidence.
pub fn controls_in(mode: semantic_cli::Mode, candidate: &Path) {
    let public = Native::create(candidate, mode);
    let executable = public.root.join("lkjscript");
    let project = public.project("source-batch-control");
    public.author(&project, r#"declarations.begin
(units
  (module create source-batch-control
    (external create add (visibility private) (implementation core.i64.add) (returns I64)
      (parameter create left (type I64)) (parameter create right (type I64)))
    (function create plus-forty-two (visibility private)
      (parameter create value (type I64)) (returns I64) (effect pure)
      (body (call add (local value) (i64 42))))
    (function create echo (visibility private)
      (parameter create value (type I64)) (returns I64) (effect pure)
      (body (local value)))
    (function create empty-task (visibility private) (returns I64) (effect (task))
      (body (i64 999)))
    (component create pure (visibility private)
      (port create main (type (function (I64) I64)) (function plus-forty-two))
      (port create echo (type (function (I64) I64)) (function echo)))
    (component create tasks (visibility private)
      (port create main (type (task-function () I64 (row))) (function empty-task))))
  (target create main (component source-batch-control::pure) (runner command) (port source-batch-control::pure::main))
  (target create echo (component source-batch-control::pure) (runner command) (port source-batch-control::pure::echo))
  (target create empty-task (component source-batch-control::tasks) (runner command) (port source-batch-control::tasks::main)))
declarations.end
"#, &[], true);
    public.check(&project);
    let head_path = project.join("HEAD");
    let head = fs::read(&head_path).unwrap();
    let candidate_digest = digest(&executable);
    let valid = serde_json::json!({"id":"valid","arguments_json":"[7]"});
    let valid_line = format!("{valid}\n");
    let late_malformed = format!("{valid_line}{{\"id\":\"late\",\"arguments_json\":\n");
    let absent = public.root.join("absent-project");
    refused(
        mode,
        &public.root,
        &executable,
        "absent-before-open",
        Some(&absent),
        "main",
        &late_malformed,
        "source",
        "run_batch_request",
    );
    assert!(!absent.exists());
    for (name, input, code) in [
        (
            "late-malformed",
            late_malformed.as_str(),
            "run_batch_request",
        ),
        (
            "repeated-id",
            &format!("{valid_line}{valid_line}"),
            "run_batch_id",
        ),
        ("empty", "", "run_batch_request"),
        (
            "unknown",
            "{\"id\":\"unknown\",\"arguments_json\":\"[7]\",\"unknown\":true}\n",
            "run_batch_request",
        ),
        (
            "repeated-key",
            "{\"id\":\"a\",\"id\":\"b\",\"arguments_json\":\"[7]\"}\n",
            "run_batch_request",
        ),
        (
            "repeated-policy-key",
            "{\"id\":\"a\",\"arguments_json\":\"[7]\",\"execution\":{\"instruction_fuel\":1,\"instruction_fuel\":2,\"maximum_call_depth\":4,\"maximum_value_stack\":4}}\n",
            "run_batch_request",
        ),
    ] {
        refused(
            mode,
            &public.root,
            &executable,
            name,
            Some(&project),
            "main",
            input,
            "source",
            code,
        );
    }
    // Even a valid current directory must not silently turn this route into discovery.
    refused(
        mode,
        &project,
        &executable,
        "explicit-project",
        None,
        "main",
        &valid_line,
        "source",
        "cli_usage",
    );
    refused(
        mode,
        &public.root,
        &executable,
        "absolute-project",
        Some(Path::new("source-batch-control")),
        "main",
        &valid_line,
        "source",
        "cli_usage",
    );
    refused(
        mode,
        &public.root,
        &executable,
        "task-base",
        Some(&project),
        "empty-task",
        &valid_line,
        "capability",
        "normalized_runner_grants_required",
    );

    let arguments = serde_json::json!([7]);
    let mut malformed = BatchCase::failure("malformed-json", &arguments, "source", "json_decode");
    malformed.arguments_json = "[".into();
    let wrong_type = BatchCase::failure(
        "wrong-type",
        &serde_json::json!([true]),
        "source",
        "normalized_json_type",
    );
    let mut quota = BatchCase::failure(
        "fuel",
        &arguments,
        "resource",
        "normalized_instruction_steps",
    );
    quota.execution = Some(
        serde_json::json!({"instruction_fuel":1,"maximum_call_depth":4096,"maximum_value_stack":1000000}),
    );
    let mut deadline =
        BatchCase::failure("deadline", &arguments, "cancelled", "execution_deadline");
    deadline.deadline_milliseconds = Some(0);
    let mut task = BatchCase::failure(
        "task-override",
        &serde_json::json!([]),
        "capability",
        "normalized_runner_grants_required",
    );
    task.target = Some("empty-task".into());
    let mut echo = BatchCase::success("different-target", &arguments, serde_json::json!(7));
    echo.target = Some("echo".into());
    public.source_batch(
        "controls",
        &project,
        "main",
        &[
            BatchCase::success("first", &arguments, serde_json::json!(49)),
            malformed,
            BatchCase::success("after-json", &arguments, serde_json::json!(49)),
            wrong_type,
            BatchCase::success("after-type", &arguments, serde_json::json!(49)),
            quota,
            BatchCase::success("after-fuel", &arguments, serde_json::json!(49)),
            deadline,
            BatchCase::success("after-deadline", &arguments, serde_json::json!(49)),
            task,
            BatchCase::success("after-task", &arguments, serde_json::json!(49)),
            echo,
        ],
    );

    let input = public.root.join("source-conflict.input.jsonl");
    let conflict = public.root.join("source-conflict.result.jsonl");
    fs::write(&input, &valid_line).unwrap();
    fs::write(&conflict, b"preserved source result").unwrap();
    let output = batch_operation(
        mode,
        &public.root,
        &executable,
        "source-conflict",
        &[
            "--project",
            path(&project),
            "run",
            "main",
            "--batch-input-file",
            path(&input),
            "--batch-result-file",
            path(&conflict),
        ],
        false,
    );
    assert_eq!(field(&output, "diagnostic", "code"), "output_conflict");
    assert_eq!(fs::read(&conflict).unwrap(), b"preserved source result");

    struct Restore<'a>(&'a Path, &'a [u8]);
    impl Drop for Restore<'_> {
        fn drop(&mut self) {
            fs::write(self.0, self.1).unwrap();
        }
    }
    {
        let _restore = Restore(&head_path, &head);
        let mut corrupted = head.clone();
        *corrupted.last_mut().unwrap() ^= 1;
        fs::write(&head_path, &corrupted).unwrap();
        refused(
            mode,
            &public.root,
            &executable,
            "corrupt-before-open",
            Some(&project),
            "main",
            &late_malformed,
            "source",
            "run_batch_request",
        );
        assert_eq!(fs::read(&head_path).unwrap(), corrupted);
        refused(
            mode,
            &public.root,
            &executable,
            "corrupt-source",
            Some(&project),
            "main",
            &valid_line,
            "corrupt",
            "packed_checksum",
        );
    }
    assert_eq!(fs::read(&head_path).unwrap(), head);
    public.source_batch(
        "after-corrupt-source",
        &project,
        "main",
        &[BatchCase::success(
            "recovered",
            &arguments,
            serde_json::json!(49),
        )],
    );
    assert_eq!(fs::read(&head_path).unwrap(), head);
    assert_eq!(digest(&executable), candidate_digest);
}

#[allow(
    clippy::too_many_arguments,
    reason = "closed actual-process controls preserve explicit boundary inputs"
)]
fn refused(
    mode: semantic_cli::Mode,
    root: &Path,
    executable: &Path,
    name: &str,
    project: Option<&Path>,
    target: &str,
    requests: &str,
    class: &str,
    code: &str,
) {
    let input = root.join(format!("source-refused-{name}.input.jsonl"));
    let result = root.join(format!("source-refused-{name}.result.jsonl"));
    assert!(!input.exists() && !result.exists());
    fs::write(&input, requests).unwrap();
    let mut arguments = Vec::new();
    if let Some(project) = project {
        arguments.extend(["--project", path(project)]);
    }
    arguments.extend([
        "run",
        target,
        "--batch-input-file",
        path(&input),
        "--batch-result-file",
        path(&result),
    ]);
    let label = format!("source-refused-{name}");
    let output = if mode == semantic_cli::Mode::HostSemantic
        && project.is_none_or(|project| !project.is_absolute())
    {
        // This public grammar rejects a missing/relative source project before reading any
        // repository or request path. Do not supply a host cwd or execute a prepared owner.
        host_operation(
            root,
            &label,
            &arguments,
            false,
            "public-batch-parser",
            || {
                let full = arguments
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect::<Vec<_>>();
                let error = lkjscript::platform::cli::parse_run_batch(&full)
                    .err()
                    .unwrap_or_else(|| {
                        panic!("missing/relative source project must refuse before preparation")
                    });
                semantic_cli::failure("run", &[error])
            },
        )
    } else {
        batch_operation(mode, root, executable, &label, &arguments, false)
    };
    assert_eq!(field(&output, "diagnostic", "class"), class, "{name}");
    assert_eq!(field(&output, "diagnostic", "code"), code, "{name}");
    assert!(!result.exists(), "{name}: no partial aggregate");
    assert_eq!(fs::read(&input).unwrap(), requests.as_bytes());
}
