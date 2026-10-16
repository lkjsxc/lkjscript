//! Finite differential rows share one exact, independently reconstructed source closure.

use super::pure_batch::{
    MAXIMUM_BATCH_BYTES, Request, Response, append_response, batch_cancellation_checkpoint,
    bounded_path, decode_requests, failure_bytes, row_control,
};
use super::*;
use crate::platform::contract::exit_status_for;
use crate::platform::execution::normalized::{
    NormalizedObservedCommandReceipt, execution_diagnostic, require_pure_artifact_target,
};
use std::sync::Arc;

pub enum RunBatchOptions {
    Deployment(PureRunBatchOptions),
    Source(SourceRunBatchOptions),
}

pub struct SourceRunBatchOptions {
    project: SourceProject,
    target: Name,
    requests: Vec<Request>,
    output: PathBuf,
}

enum SourceProject {
    Disk(PathBuf),
    Memory(crate::platform::MemProjectStore),
}

pub fn parse_run_batch(arguments: &[String]) -> Result<RunBatchOptions, Diagnostic> {
    if arguments.iter().any(|argument| argument == "--deployment") {
        return parse_pure_run_batch(arguments).map(RunBatchOptions::Deployment);
    }
    parse_source_batch(arguments, None)
}

/// Select memory authority explicitly before the source preparation worker starts.
/// The ordinary framing, output, differential execution and cleanup checks still apply.
pub fn parse_run_batch_in_memory(
    project: &crate::platform::MemProjectStore,
    arguments: &[String],
) -> Result<RunBatchOptions, Diagnostic> {
    if arguments
        .iter()
        .any(|argument| matches!(argument.as_str(), "--project" | "--deployment"))
    {
        return Err(usage_error(
            "an explicit memory source batch cannot select --project or --deployment",
        ));
    }
    parse_source_batch(arguments, Some(project.clone()))
}

fn parse_source_batch(
    arguments: &[String],
    memory: Option<crate::platform::MemProjectStore>,
) -> Result<RunBatchOptions, Diagnostic> {
    let (arguments, project) = extract_global_project(arguments.to_vec())?;
    if arguments.first().map(String::as_str) != Some("run") {
        return Err(usage_error("source batches require the run operation"));
    }
    let target = arguments
        .get(1)
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("source batches require one target name"))?;
    let target = Name::new(target.clone())?;
    ensure_options(
        &arguments[2..],
        &["--batch-input-file", "--batch-result-file"],
        &[],
    )?;
    let project = match memory {
        Some(memory) => SourceProject::Memory(memory),
        None => {
            let project =
                project.ok_or_else(|| usage_error("source batches require explicit --project"))?;
            if !project.is_absolute() {
                return Err(usage_error(
                    "source batch --project must be an absolute path",
                ));
            }
            SourceProject::Disk(project)
        }
    };
    let input = bounded_path(required_option(&arguments[2..], "--batch-input-file")?)?;
    let output = bounded_path(required_option(&arguments[2..], "--batch-result-file")?)?;
    // Destination and the complete framing precede repository reads, compilation or execution.
    let output = super::super::owned_output::inspect_create_new(&output)?;
    let bytes = read_bounded(&input, MAXIMUM_BATCH_BYTES, "source batch input")?;
    let requests = decode_requests(&bytes)?;
    Ok(RunBatchOptions::Source(SourceRunBatchOptions {
        project,
        target,
        requests,
        output,
    }))
}

pub async fn execute_run_batch(
    options: RunBatchOptions,
    cancellation: impl std::future::Future<Output = ()>,
) -> Result<Vec<u8>, Diagnostic> {
    match options {
        RunBatchOptions::Deployment(options) => execute_pure_run_batch(options, cancellation).await,
        RunBatchOptions::Source(options) => execute_source_run_batch(options, cancellation).await,
    }
}

async fn execute_source_run_batch(
    options: SourceRunBatchOptions,
    cancellation: impl std::future::Future<Output = ()>,
) -> Result<Vec<u8>, Diagnostic> {
    let control = ExecutionControl::uncancelled();
    let preparation_control = control.clone();
    let project = options.project;
    let target = options.target;
    let mut preparing = tokio::task::spawn_blocking(move || {
        let repository = match project {
            SourceProject::Disk(project) => open_normalized_repository(Some(project))?,
            SourceProject::Memory(project) => GraphRepository::open_memory(&project)?,
        };
        let prepared = super::super::normalized_lifecycle::prepare_repository_with_control(
            repository,
            &preparation_control,
        )?;
        require_pure_artifact_target(&prepared.program, &target)?;
        Ok::<_, Diagnostic>((prepared, target))
    });
    tokio::pin!(cancellation);
    let (prepared, target) = tokio::select! {
        biased;
        result = &mut preparing => result.map_err(foreground_join_error)??,
        () = cancellation.as_mut() => {
            control.cancel();
            let joined = preparing.await.map_err(foreground_join_error)?;
            let mut error = cancelled("source batch preparation was cancelled");
            if let Err(failure) = joined {
                retain_joined_failure(&mut error, failure);
            }
            return Err(error);
        }
    };
    let prepared = Arc::new(prepared);
    let mut encoded = Vec::new();
    let mut passed = 0_usize;
    let mut failed = 0_usize;
    let rows = options.requests.len();
    for request in options.requests {
        batch_cancellation_checkpoint(cancellation.as_mut()).await?;
        let Request {
            id,
            target: row_target,
            arguments_json,
            execution,
            deadline_milliseconds,
        } = request;
        let row_control = row_control(deadline_milliseconds);
        let outcome = match row_control {
            Ok(control) => {
                let row_control = control.clone();
                let owner = Arc::clone(&prepared);
                let target = target.clone();
                let mut running = tokio::task::spawn_blocking(move || {
                    row_control.check().map_err(execution_diagnostic)?;
                    let target = row_target.map(Name::new).transpose()?.unwrap_or(target);
                    // Callable kind and component requirements reject even when a task row is empty.
                    require_pure_artifact_target(&owner.program, &target)?;
                    let policy = NormalizedCommandPolicy {
                        execution: execution.unwrap_or_default().into(),
                        ..Default::default()
                    };
                    owner.run_observed(&target, arguments_json.as_bytes(), policy, &row_control)
                });
                tokio::select! {
                    biased;
                    result = &mut running => result.map_err(foreground_join_error)?,
                    () = cancellation.as_mut() => {
                        control.cancel();
                        let joined = running.await.map_err(foreground_join_error)?;
                        let mut error = cancelled("source batch was cancelled after its current row joined");
                        match joined {
                            Ok(receipt) => retain_joined_observations(&mut error, &receipt),
                            Err(failure) => retain_joined_failure(&mut error, failure),
                        }
                        return Err(error);
                    }
                }
            }
            Err(error) => Err(error),
        };
        // A changed source invalidates the aggregate; never retarget the retained reference closure.
        if let Err(error) = &outcome
            && error.code == "lifecycle_authority_changed"
        {
            return Err(error.clone());
        }
        let (exit_code, stdout, result_json) = match outcome {
            Ok(mut receipt) => {
                let result = String::from_utf8(std::mem::take(&mut receipt.command.result_json))
                    .map_err(|_| {
                        batch_error(
                            DiagnosticClass::Corrupt,
                            "run_result_utf8",
                            "typed source batch result is not UTF-8 JSON",
                        )
                    })?;
                match source_run_receipt_output(
                    &prepared,
                    receipt.command,
                    None,
                    Some(&receipt.executor),
                    true,
                ) {
                    Ok(stdout) => {
                        passed += 1;
                        (0, stdout, Some(result))
                    }
                    Err(error) => {
                        failed += 1;
                        (exit_status_for(error.class), failure_bytes(&error)?, None)
                    }
                }
            }
            Err(error) => {
                failed += 1;
                (exit_status_for(error.class), failure_bytes(&error)?, None)
            }
        };
        let stdout = String::from_utf8(stdout).map_err(|_| {
            batch_error(
                DiagnosticClass::Infrastructure,
                "run_batch_response",
                "source batch row projection is not UTF-8",
            )
        })?;
        append_response(
            &mut encoded,
            &Response {
                id,
                exit_code,
                stdout,
                result_json,
            },
        )?;
    }
    finish_source_batch(
        &prepared,
        &options.output,
        &encoded,
        rows,
        passed,
        failed,
        cancellation.as_mut(),
    )
    .await
}

async fn finish_source_batch(
    prepared: &PreparedApplication,
    output_path: &Path,
    encoded: &[u8],
    rows: usize,
    passed: usize,
    failed: usize,
    mut cancellation: std::pin::Pin<&mut impl std::future::Future<Output = ()>>,
) -> Result<Vec<u8>, Diagnostic> {
    batch_cancellation_checkpoint(cancellation.as_mut()).await?;
    prepared.check_authority()?;
    let root = prepared.program.artifact().root_package().ok_or_else(|| {
        batch_error(
            DiagnosticClass::Corrupt,
            "run_batch_source_binding",
            "source batch artifact has no exact root package",
        )
    })?;
    if root.repository_id != prepared.repository_id
        || root.package != prepared.package
        || root.semantic_revision != prepared.revision
        || root.semantic_state != prepared.semantic_state
    {
        return Err(batch_error(
            DiagnosticClass::Corrupt,
            "run_batch_source_binding",
            "source batch artifact differs from the exact accepted root binding",
        ));
    }
    let mut output = compact_response_writer()?;
    append_compact_record(
        &mut output,
        "result",
        &[
            ("status", "success".to_owned()),
            ("command", "run.batch".to_owned()),
        ],
    )?;
    append_lifecycle_records(&mut output, prepared)?;
    append_compact_record(
        &mut output,
        "batch",
        &[
            ("mode", "source-differential".to_owned()),
            ("package-revision", root.package_revision.to_string()),
            (
                "schema",
                super::super::canonical_schema::SCHEMA_EPOCH.to_string(),
            ),
            (
                "validator",
                encode_hex(&super::super::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH),
            ),
            ("rows", rows.to_string()),
            ("passed", passed.to_string()),
            ("failed", failed.to_string()),
            ("source-preparations", "1".to_owned()),
            ("result-file", output_path.display().to_string()),
        ],
    )?;
    // Recheck both cancellation and authority after rendering, immediately before aggregate publication.
    batch_cancellation_checkpoint(cancellation.as_mut()).await?;
    prepared.check_authority()?;
    batch_cancellation_checkpoint(cancellation.as_mut()).await?;
    let publication = publish_create_new(
        output_path,
        encoded,
        MAXIMUM_BATCH_BYTES,
        "source batch result",
    )?;
    append_compact_record(
        &mut output,
        "output",
        &[
            ("path", publication.path.display().to_string()),
            ("bytes", publication.bytes.to_string()),
            ("visibility", publication.visibility.to_owned()),
            ("durability", publication.durability.to_owned()),
            ("stage-cleanup", publication.stage_cleanup.to_owned()),
        ],
    )?;
    Ok(output.finish())
}

pub(super) fn observation_json(observation: &impl serde::Serialize) -> Result<String, Diagnostic> {
    serde_json::to_string(observation).map_err(|_| {
        batch_error(
            DiagnosticClass::Infrastructure,
            "run_batch_response",
            "source execution observation could not be encoded",
        )
    })
}

fn cancelled(message: &str) -> Diagnostic {
    batch_error(DiagnosticClass::Cancelled, "execution_cancelled", message)
}

fn retain_joined_failure(error: &mut Diagnostic, failure: Diagnostic) {
    error.notes.push(format!(
        "joined source row failed with safe code '{}': {}",
        failure.code, failure.message
    ));
    error.notes.extend(failure.notes);
}

fn retain_joined_observations(error: &mut Diagnostic, receipt: &NormalizedObservedCommandReceipt) {
    for (name, observation) in [
        (
            "production-observation",
            observation_json(&receipt.command.production),
        ),
        (
            "reference-observation",
            observation_json(&receipt.command.reference),
        ),
        ("executor-observation", observation_json(&receipt.executor)),
    ] {
        match observation {
            Ok(json) => error.notes.push(format!("{name}: {json}")),
            Err(failure) => retain_joined_failure(error, failure),
        }
    }
}

fn batch_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::control::{CompactRecord, parse_records};
    use serde_json::{Value, json};

    fn field<'a>(records: &'a [CompactRecord], operation: &str, name: &str) -> &'a str {
        records
            .iter()
            .find(|record| record.operation == operation)
            .unwrap()
            .fields
            .iter()
            .find(|field| field.name == name)
            .unwrap()
            .value
            .as_str()
    }

    #[tokio::test]
    async fn source_batch_large_typed_results_use_complete_rows_and_bounded_observation_records() {
        let temporary = tempfile::tempdir().unwrap();
        let project = temporary.path().join("project");
        let created = GraphRepository::create(
            &project,
            &crate::platform::kernel::tests::transport_snapshot(),
            None,
        )
        .unwrap();
        let source = format!(
            r#"request base={}
declarations.begin
(units
  (module create large-source-results
    (function create echo (visibility private) (effect pure)
      (parameter create value (type Text)) (returns Text) (body (local value)))
    (component create command (visibility private)
      (port create main (type (function (Text) Text)) (function echo))))
  (target create echo (component large-source-results::command) (runner command) (port large-source-results::command::main)))
declarations.end
"#,
            created.current.head.revision
        );
        let request = crate::platform::control::decode_compact_change(
            "large-source-results.lkjc",
            source.as_bytes(),
        )
        .unwrap();
        let prepared = created
            .repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        created.repository.publish(&prepared.publication).unwrap();
        let head = fs::read(project.join("HEAD")).unwrap();
        let values = [
            "first λ \"line\"\n".repeat(6_000),
            "second ☃ \\ tail\n".repeat(5_000),
        ];
        let mut requests = String::new();
        for (id, value) in values.iter().enumerate() {
            if id == 1 {
                requests
                    .push_str(&json!({"id":"wrong-type", "arguments_json":"[false]"}).to_string());
                requests.push('\n');
            }
            assert!(
                value.len() > 65_536,
                "regression crosses the compact record limit"
            );
            requests.push_str(
                &json!({"id":id.to_string(), "arguments_json":json!([value]).to_string()})
                    .to_string(),
            );
            requests.push('\n');
        }
        let input = temporary.path().join("input.jsonl");
        let output = temporary.path().join("result.jsonl");
        fs::write(&input, &requests).unwrap();
        let arguments = vec![
            "--project".to_owned(),
            project.display().to_string(),
            "run".to_owned(),
            "echo".to_owned(),
            "--batch-input-file".to_owned(),
            input.display().to_string(),
            "--batch-result-file".to_owned(),
            output.display().to_string(),
        ];
        let summary =
            execute_run_batch(parse_run_batch(&arguments).unwrap(), std::future::pending())
                .await
                .unwrap();
        let summary = parse_records("large source batch summary", &summary).unwrap();
        assert_eq!(field(&summary, "batch", "rows"), "3");
        assert_eq!(field(&summary, "batch", "passed"), "2");
        assert_eq!(field(&summary, "batch", "failed"), "1");
        assert_eq!(field(&summary, "batch", "source-preparations"), "1");
        let rows = fs::read_to_string(&output).unwrap();
        let rows = rows
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1]["id"], "wrong-type");
        assert_ne!(rows[1]["exit_code"], 0);
        assert!(rows[1]["result_json"].is_null());
        let refused = parse_records(
            "large source batch refused row",
            rows[1]["stdout"].as_str().unwrap().as_bytes(),
        )
        .unwrap();
        assert_eq!(field(&refused, "diagnostic", "class"), "source");
        assert_eq!(
            field(&refused, "diagnostic", "code"),
            "normalized_json_type"
        );
        for (id, (row, value)) in [&rows[0], &rows[2]].into_iter().zip(&values).enumerate() {
            assert_eq!(row["id"], id.to_string());
            assert_eq!(row["exit_code"], 0);
            let result = row["result_json"].as_str().unwrap();
            assert_eq!(serde_json::from_str::<Value>(result).unwrap(), json!(value));
            let stdout = row["stdout"].as_str().unwrap();
            assert!(
                stdout.len() < 65_536,
                "actual observations fit the unchanged compact record bound"
            );
            let records = parse_records("large source batch row", stdout.as_bytes()).unwrap();
            assert_eq!(field(&records, "execution", "value-location"), "batch-row");
            assert_eq!(field(&records, "execution", "differential"), "equal");
            let execution = records
                .iter()
                .find(|record| record.operation == "execution")
                .unwrap();
            assert!(
                !execution
                    .fields
                    .iter()
                    .any(|field| matches!(field.name.as_str(), "value" | "result-file"))
            );
            for (name, live) in [
                (
                    "production-observation",
                    &[
                        "live_call_frames_after",
                        "live_locals_after",
                        "live_type_bindings_after",
                        "live_operands_after",
                        "live_transactions_after",
                        "live_handles_after",
                    ][..],
                ),
                (
                    "reference-observation",
                    &[
                        "live_call_frames_after",
                        "live_control_frames_after",
                        "live_local_scopes_after",
                        "live_type_scopes_after",
                        "live_effect_scopes_after",
                        "live_allowances_after",
                        "live_transactions_after",
                        "live_handles_after",
                    ][..],
                ),
            ] {
                let observation: Value =
                    serde_json::from_str(field(&records, "execution", name)).unwrap();
                assert_eq!(observation["capability_calls"], 0);
                for name in live {
                    assert_eq!(observation[*name], 0, "unreleased {name}");
                }
            }
            assert!(
                field(&records, "execution", "production-instructions")
                    .parse::<u64>()
                    .unwrap()
                    > 0
            );
            assert!(
                field(&records, "execution", "reference-expressions")
                    .parse::<u64>()
                    .unwrap()
                    > 0
            );
            let executor: Value =
                serde_json::from_str(field(&records, "execution", "executor-observation")).unwrap();
            assert_eq!(executor["dispatch_open"], false);
            assert_eq!(executor["active_dispatches"], 0);
            assert_eq!(executor["remaining_workers"], 0);
        }
        assert_eq!(fs::read(input).unwrap(), requests.as_bytes());
        assert_eq!(fs::read(project.join("HEAD")).unwrap(), head);
    }

    fn prepared_source() -> (tempfile::TempDir, PreparedApplication) {
        let temporary = tempfile::tempdir().unwrap();
        let created = GraphRepository::create(
            &temporary.path().join("project"),
            &crate::platform::kernel::tests::transport_snapshot(),
            None,
        )
        .unwrap();
        let prepared = prepare_repository(created.repository).unwrap();
        (temporary, prepared)
    }

    #[tokio::test]
    async fn source_aggregate_cancellation_before_publication_exposes_no_output() {
        let (temporary, prepared) = prepared_source();
        let output = temporary.path().join("result.jsonl");
        let head = temporary.path().join("project/HEAD");
        let before = fs::read(&head).unwrap();
        let mut cancellation = std::pin::pin!(std::future::ready(()));
        let error = finish_source_batch(
            &prepared,
            &output,
            b"unpublished rows",
            1,
            1,
            0,
            cancellation.as_mut(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "execution_cancelled");
        assert!(!output.exists());
        assert_eq!(fs::read(head).unwrap(), before);
    }

    #[tokio::test]
    async fn source_aggregate_refuses_new_authority_without_exposing_earlier_rows() {
        let (temporary, prepared) = prepared_source();
        let output = temporary.path().join("result.jsonl");
        let source = format!(
            "request base={}\ndeclarations.begin\n(units (module create later-source))\ndeclarations.end\n",
            prepared.revision
        );
        let request =
            crate::platform::control::decode_compact_change("later-source.lkjc", source.as_bytes())
                .unwrap();
        let next = prepared
            .repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        prepared.repository.publish(&next.publication).unwrap();
        let mut cancellation = std::pin::pin!(std::future::pending());
        let error = finish_source_batch(
            &prepared,
            &output,
            b"unpublished earlier rows",
            1,
            1,
            0,
            cancellation.as_mut(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "lifecycle_authority_changed");
        assert!(!output.exists());
        assert_eq!(
            prepared.repository.view_current().unwrap().revision(),
            next.publication.head.revision
        );
    }

    #[test]
    fn source_batch_framing_and_destination_are_validated_before_source_is_opened() {
        let temporary = tempfile::tempdir().unwrap();
        let input = temporary.path().join("input.jsonl");
        let output = temporary.path().join("result.jsonl");
        let project = temporary.path().join("absent-project");
        let args = vec![
            "--project".into(),
            project.display().to_string(),
            "run".into(),
            "main".into(),
            "--batch-input-file".into(),
            input.display().to_string(),
            "--batch-result-file".into(),
            output.display().to_string(),
        ];
        fs::write(&input, "{\"id\":\"first\",\"arguments_json\":\"[]\"}\n{\n").unwrap();
        let error = parse_run_batch(&args).err().unwrap();
        assert_eq!(error.code, "run_batch_request");
        assert!(!output.exists());
        assert!(!project.exists());
        fs::write(&output, "preserved result").unwrap();
        let error = parse_run_batch(&args).err().unwrap();
        assert_eq!(error.code, "output_conflict");
        assert_eq!(fs::read(&output).unwrap(), b"preserved result");
        assert!(!project.exists());
    }

    #[test]
    fn source_batches_require_explicit_absolute_project_and_refuse_other_run_options() {
        for args in [
            vec![
                "run",
                "main",
                "--batch-input-file",
                "a",
                "--batch-result-file",
                "b",
            ],
            vec![
                "--project",
                "relative",
                "run",
                "main",
                "--batch-input-file",
                "a",
                "--batch-result-file",
                "b",
            ],
            vec![
                "--project",
                "/absent",
                "run",
                "main",
                "--arguments",
                "[]",
                "--batch-input-file",
                "a",
                "--batch-result-file",
                "b",
            ],
        ] {
            assert!(
                parse_run_batch(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err()
            );
        }
    }
}
