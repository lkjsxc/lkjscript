//! Finite matrices use public host adapters or the exact selected candidate, with independent expectations.
use super::*;

pub struct BatchCase {
    pub id: String,
    pub arguments_json: String,
    pub target: Option<String>,
    pub execution: Option<Value>,
    pub deadline_milliseconds: Option<u64>,
    pub(super) expected: Expected,
}

pub(super) enum Expected {
    Success(Value),
    Failure(&'static str, &'static str),
}

impl BatchCase {
    pub fn success(id: impl Into<String>, arguments: &Value, expected: Value) -> Self {
        Self {
            id: id.into(),
            arguments_json: arguments.to_string(),
            target: None,
            execution: None,
            deadline_milliseconds: None,
            expected: Expected::Success(expected),
        }
    }

    pub fn failure(
        id: impl Into<String>,
        arguments: &Value,
        class: &'static str,
        code: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            arguments_json: arguments.to_string(),
            target: None,
            execution: None,
            deadline_milliseconds: None,
            expected: Expected::Failure(class, code),
        }
    }

    pub(super) fn request(&self) -> Value {
        let mut row = serde_json::json!({
            "id": self.id,
            "arguments_json": self.arguments_json,
        });
        if let Some(target) = &self.target {
            row["target"] = target.clone().into();
        }
        if let Some(execution) = &self.execution {
            row["execution"] = execution.clone();
        }
        if let Some(deadline) = self.deadline_milliseconds {
            row["deadline_milliseconds"] = deadline.into();
        }
        row
    }
}

impl Native {
    pub fn batch(&self, label: &str, descriptor: &Path, cases: &[BatchCase]) -> Vec<String> {
        execute_in(
            self.mode,
            &self.root,
            &self.root.join("lkjscript"),
            label,
            descriptor,
            cases,
        )
    }

    pub fn batch_controls(&self, descriptor: &Path, arguments: &Value, expected: &Value) {
        let valid = serde_json::json!({"id":"valid","arguments_json":arguments.to_string()});
        let valid_line = format!("{valid}\n");
        let malformed = format!("{valid_line}{{\"id\":\"late\",\"arguments_json\":\n");
        for (name, text, code) in [
            ("late-malformed", malformed.as_str(), "run_batch_request"),
            (
                "repeated-id",
                &format!("{valid_line}{valid_line}"),
                "run_batch_id",
            ),
            ("empty", "", "run_batch_request"),
            (
                "unknown",
                "{\"id\":\"unknown\",\"arguments_json\":\"[]\",\"unknown\":true}\n",
                "run_batch_request",
            ),
            (
                "repeated-key",
                "{\"id\":\"a\",\"id\":\"b\",\"arguments_json\":\"[]\"}\n",
                "run_batch_request",
            ),
        ] {
            self.batch_refused(name, descriptor, text, Some(("source", code)));
        }
        let mut malformed_json =
            BatchCase::failure("malformed-json", arguments, "source", "json_decode");
        malformed_json.arguments_json = "[".to_owned();
        let mut cancelled =
            BatchCase::failure("cancelled", arguments, "cancelled", "execution_deadline");
        cancelled.deadline_milliseconds = Some(0);
        self.batch(
            "row-recovery",
            descriptor,
            &[
                malformed_json,
                BatchCase::success("after-json", arguments, expected.clone()),
                cancelled,
                BatchCase::success("after-deadline", arguments, expected.clone()),
            ],
        );

        let input = self.input("conflict.input.jsonl", &valid_line);
        let conflict = self.input("conflict.result.jsonl", "preserved result");
        let output = self.batch_control(
            "conflict",
            &[
                "run",
                "--deployment",
                path(descriptor),
                "--batch-input-file",
                path(&input),
                "--batch-result-file",
                path(&conflict),
            ],
            false,
        );
        assert_eq!(field(&output, "diagnostic", "code"), "output_conflict");
        assert_eq!(fs::read(&conflict).unwrap(), b"preserved result");

        let deployment: Value = serde_json::from_slice(&fs::read(descriptor).unwrap()).unwrap();
        let artifact = descriptor
            .parent()
            .unwrap()
            .join(deployment["artifact"].as_str().unwrap());
        let original = fs::read(&artifact).unwrap();
        self.input("original-artifact.sha256", &digest(&artifact));
        fs::write(self.root.join("original-artifact.lkja"), &original).unwrap();
        struct Restore<'a>(&'a Path, &'a [u8]);
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                fs::write(self.0, self.1).unwrap();
            }
        }
        {
            let _restore = Restore(&artifact, &original);
            let mut corrupted = original.clone();
            let last = corrupted.len() - 1;
            corrupted[last] ^= 1;
            fs::write(&artifact, corrupted).unwrap();
            // A late invalid envelope is rejected before even a corrupt artifact is admitted.
            self.batch_refused(
                "outer-before-artifact",
                descriptor,
                &malformed,
                Some(("source", "run_batch_request")),
            );
            self.batch_refused(
                "corrupt-artifact",
                descriptor,
                &valid_line,
                Some(("corrupt", "")),
            );
        }
        assert_eq!(fs::read(&artifact).unwrap(), original);
        self.batch(
            "after-corrupt-artifact",
            descriptor,
            &[BatchCase::success("recovered", arguments, expected.clone())],
        );
    }

    fn batch_refused(
        &self,
        name: &str,
        descriptor: &Path,
        requests: &str,
        diagnostic: Option<(&str, &str)>,
    ) {
        let input = self.input(&format!("refused-{name}.input.jsonl"), requests);
        let result = self.root.join(format!("refused-{name}.result.jsonl"));
        assert!(!result.exists());
        let output = self.batch_control(
            &format!("refused-{name}"),
            &[
                "run",
                "--deployment",
                path(descriptor),
                "--batch-input-file",
                path(&input),
                "--batch-result-file",
                path(&result),
            ],
            false,
        );
        if let Some((class, code)) = diagnostic {
            assert_eq!(field(&output, "diagnostic", "class"), class, "{name}");
            if !code.is_empty() {
                assert_eq!(field(&output, "diagnostic", "code"), code, "{name}");
            }
        }
        assert!(
            !result.exists(),
            "invalid batch exposed a partial aggregate: {name}"
        );
        assert_eq!(fs::read(&input).unwrap(), requests.as_bytes());
    }

    fn batch_control(&self, label: &str, arguments: &[&str], success: bool) -> String {
        if self.mode == semantic_cli::Mode::HostSemantic {
            batch_operation(
                self.mode,
                &self.root,
                &self.root.join("lkjscript"),
                label,
                arguments,
                success,
            )
        } else {
            // Preserve the original boundary controls' exact process environment and logs.
            self.process_cli(&self.root, arguments, success)
        }
    }
}

/// Explicit executable callers retain the real process boundary.
pub fn execute(
    root: &Path,
    executable: &Path,
    label: &str,
    descriptor: &Path,
    cases: &[BatchCase],
) -> Vec<String> {
    execute_in(
        semantic_cli::Mode::CandidateProcess,
        root,
        executable,
        label,
        descriptor,
        cases,
    )
}

pub fn execute_in(
    mode: semantic_cli::Mode,
    root: &Path,
    executable: &Path,
    label: &str,
    descriptor: &Path,
    cases: &[BatchCase],
) -> Vec<String> {
    execute_refs_in(
        mode,
        root,
        executable,
        label,
        descriptor,
        &cases.iter().collect::<Vec<_>>(),
    )
}

pub(super) fn execute_refs(
    root: &Path,
    executable: &Path,
    label: &str,
    descriptor: &Path,
    cases: &[&BatchCase],
) -> Vec<String> {
    execute_refs_in(
        semantic_cli::Mode::CandidateProcess,
        root,
        executable,
        label,
        descriptor,
        cases,
    )
}

fn execute_refs_in(
    mode: semantic_cli::Mode,
    root: &Path,
    executable: &Path,
    label: &str,
    descriptor: &Path,
    cases: &[&BatchCase],
) -> Vec<String> {
    assert!(!cases.is_empty());
    let descriptor_bytes = fs::read(descriptor).unwrap();
    let descriptor_value: Value = serde_json::from_slice(&descriptor_bytes).unwrap();
    let artifact = descriptor
        .parent()
        .unwrap()
        .join(descriptor_value["artifact"].as_str().unwrap());
    let artifact_digest = digest(&artifact);
    let candidate_digest = digest(executable);
    let request_path = root.join(format!("batch-{label}.input.jsonl"));
    let result_path = root.join(format!("batch-{label}.result.jsonl"));
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
        label,
        &[
            "run",
            "--deployment",
            path(descriptor),
            "--batch-input-file",
            path(&request_path),
            "--batch-result-file",
            path(&result_path),
        ],
        true,
    );
    assert_eq!(field(&summary, "result", "command"), "run.batch");
    assert_eq!(field(&summary, "batch", "artifact-sha256"), artifact_digest);
    assert_eq!(field(&summary, "batch", "artifact-preparations"), "1");
    assert_eq!(field(&summary, "batch", "rows"), cases.len().to_string());
    let wanted_passed = cases
        .iter()
        .filter(|case| matches!(&case.expected, Expected::Success(_)))
        .count();
    assert_eq!(
        field(&summary, "batch", "passed"),
        wanted_passed.to_string()
    );
    assert_eq!(
        field(&summary, "batch", "failed"),
        (cases.len() - wanted_passed).to_string()
    );
    let outputs = fs::read_to_string(&result_path).unwrap();
    let rows = decode_rows(&outputs);
    assert_eq!(rows.len(), cases.len(), "complete candidate row inventory");
    let mut seen = BTreeSet::new();
    let mut observed = Vec::with_capacity(rows.len());
    for (row, case) in rows.iter().zip(cases) {
        assert_eq!(
            row.as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["id", "exit_code", "stdout", "result_json"]),
            "closed candidate row outcome"
        );
        assert_eq!(row["id"].as_str(), Some(case.id.as_str()));
        assert!(seen.insert(case.id.as_str()), "repeated candidate row");
        let stdout = row["stdout"].as_str().unwrap();
        match &case.expected {
            Expected::Success(expected) => {
                assert_eq!(row["exit_code"], 0, "{}: {stdout}", case.id);
                assert_eq!(field(stdout, "result", "status"), "success", "{}", case.id);
                assert_eq!(
                    field(stdout, "execution", "target"),
                    case.target
                        .as_deref()
                        .unwrap_or_else(|| descriptor_value["target"].as_str().unwrap()),
                    "{}: exact selected target",
                    case.id
                );
                let result = row["result_json"]
                    .as_str()
                    .unwrap_or_else(|| panic!("successful candidate result"));
                assert_eq!(
                    &serde_json::from_str::<Value>(result).unwrap(),
                    expected,
                    "{}",
                    case.id
                );
                let production = quoted(stdout, "production-observation");
                for name in [
                    "live_handles_after",
                    "live_locals_after",
                    "live_operands_after",
                    "live_call_frames_after",
                    "live_transactions_after",
                    "live_type_bindings_after",
                ] {
                    assert_eq!(production[name], 0, "{}: {name}", case.id);
                }
                let cleanup = quoted(stdout, "cleanup");
                assert_eq!(cleanup["remaining_tasks"], 0, "{}", case.id);
                assert_eq!(cleanup["cleanup_failures"], serde_json::json!([]));
                let executor = quoted(stdout, "executor-observation");
                assert_eq!(executor["remaining_workers"], 0, "{}", case.id);
                assert_eq!(executor["active_dispatches"], 0, "{}", case.id);
                assert_eq!(executor["dispatch_open"], false, "{}", case.id);
            }
            Expected::Failure(class, code) => {
                assert_ne!(row["exit_code"], 0, "{}", case.id);
                assert_eq!(field(stdout, "result", "status"), "failure", "{}", case.id);
                assert!(row["result_json"].is_null(), "failed row exposed a result");
                assert_eq!(field(stdout, "diagnostic", "class"), *class, "{}", case.id);
                assert_eq!(field(stdout, "diagnostic", "code"), *code, "{}", case.id);
            }
        }
        observed.push(stdout.to_owned());
    }
    assert_eq!(fs::read(&request_path).unwrap(), requests.as_bytes());
    assert_eq!(fs::read(descriptor).unwrap(), descriptor_bytes);
    assert_eq!(digest(&artifact), artifact_digest);
    assert_eq!(digest(executable), candidate_digest);
    observed
}

pub(super) fn decode_rows(outputs: &str) -> Vec<Value> {
    use lkjscript::platform::json::{JsonIntegerPolicy, decode_strict_with_integer_policy};
    assert!(outputs.ends_with('\n'), "complete JSONL response framing");
    assert!(outputs.len() <= 64 * 1024 * 1024, "bounded batch aggregate");
    outputs
        .lines()
        .map(|line| {
            decode_strict_with_integer_policy(
                line.as_bytes(),
                JsonLimits {
                    maximum_bytes: 4 * 1024 * 1024,
                    maximum_string_bytes: 4 * 1024 * 1024,
                    ..JsonLimits::default()
                },
                JsonIntegerPolicy::SignedOrUnsigned64,
            )
            .unwrap()
        })
        .collect()
}

pub(super) fn batch_operation(
    mode: semantic_cli::Mode,
    root: &Path,
    executable: &Path,
    label: &str,
    arguments: &[&str],
    success: bool,
) -> String {
    if mode != semantic_cli::Mode::HostSemantic {
        return batch_process(root, executable, label, arguments, success);
    }
    host_operation(root, label, arguments, success, "public-cli", || {
        let command_arguments = if arguments.first() == Some(&"--project") {
            &arguments[2..]
        } else {
            arguments
        };
        semantic_cli::execute(
            command_arguments,
            arguments
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect(),
        )
        .unwrap_or_else(|| panic!("closed host batch route requires absolute explicit paths"))
    })
}

pub(super) fn host_operation(
    root: &Path,
    label: &str,
    arguments: &[&str],
    success: bool,
    adapter: &str,
    operation: impl FnOnce() -> semantic_cli::Response,
) -> String {
    let evidence = root.join(format!("batch-{label}.command.json"));
    assert!(!evidence.exists());
    fs::write(
        &evidence,
        serde_json::to_vec_pretty(&serde_json::json!({
            "arguments": arguments,
            "execution_mode": semantic_cli::Mode::HostSemantic.label(),
            "adapter": adapter,
            "expected_success": success,
        }))
        .unwrap(),
    )
    .unwrap();
    let response = operation();
    fs::write(root.join(format!("batch-{label}.stdout")), &response.stdout).unwrap();
    fs::write(root.join(format!("batch-{label}.stderr")), &response.stderr).unwrap();
    fs::write(
        root.join(format!("batch-{label}.terminal.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "execution_mode": semantic_cli::Mode::HostSemantic.label(),
            "adapter_status": response.code,
            "success": response.success,
        }))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        response.success,
        success,
        "{arguments:?}\n{}\n{}",
        String::from_utf8_lossy(&response.stdout),
        String::from_utf8_lossy(&response.stderr)
    );
    assert!(response.stderr.is_empty());
    String::from_utf8(response.stdout).unwrap()
}

pub(super) fn batch_process(
    root: &Path,
    executable: &Path,
    label: &str,
    arguments: &[&str],
    success: bool,
) -> String {
    let evidence = root.join(format!("batch-{label}.command.json"));
    assert!(!evidence.exists());
    fs::write(
        &evidence,
        serde_json::to_vec_pretty(&serde_json::json!({
            "executable": executable, "directory": root, "arguments": arguments,
            "environment_cleared": true, "environment": {"PATH":"","LANG":"C"},
            "execution_mode": if arguments.contains(&"--batch-input-file") { "executable-batch" } else { "executable-boundary" }, "expected_success": success,
        }))
        .unwrap(),
    )
    .unwrap();
    let output = Command::new(executable)
        .current_dir(root)
        .env_clear()
        .env("PATH", "")
        .env("LANG", "C")
        .args(arguments)
        .output()
        .unwrap();
    fs::write(root.join(format!("batch-{label}.stdout")), &output.stdout).unwrap();
    fs::write(root.join(format!("batch-{label}.stderr")), &output.stderr).unwrap();
    fs::write(
        root.join(format!("batch-{label}.terminal.json")),
        serde_json::to_vec_pretty(
            &serde_json::json!({"status":output.status.code(),"success":output.status.success()}),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        output.status.success(),
        success,
        "{arguments:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout).unwrap()
}
