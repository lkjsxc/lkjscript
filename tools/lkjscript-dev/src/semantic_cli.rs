//! Linked public operations produce return-value evidence; only real children produce process evidence.
use crate::{error::DevError, evidence, process};
use lkjscript::platform::contract::{diagnostic_class_name, exit_status_for};
use lkjscript::platform::control::{CompactResponseLimits, CompactResponseWriter};
use lkjscript::platform::{self, Diagnostic};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "engine", content = "observation", rename_all = "snake_case")]
pub(crate) enum CallObservation {
    Semantic(SemanticObservation),
    Process(process::ProcessObservation),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SemanticObservation {
    pub(crate) schema: String,
    pub(crate) backend: SemanticBackend,
    pub(crate) operation: String,
    pub(crate) arguments: Vec<String>,
    pub(crate) returned_success: bool,
    /// The public diagnostic protocol mapping, never an observed operating-system exit.
    pub(crate) protocol_code: u8,
    pub(crate) elapsed_nanoseconds: u64,
    pub(crate) response: evidence::FileProof,
    pub(crate) diagnostics: evidence::FileProof,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SemanticBackend {
    Filesystem,
    Memory,
}

impl CallObservation {
    /// Semantic response bytes and actual child stdout are distinct serialized fields.
    pub(crate) fn stdout(&self) -> &evidence::FileProof {
        match self {
            Self::Semantic(call) => &call.response,
            Self::Process(call) => &call.stdout,
        }
    }
    pub(crate) fn stderr(&self) -> &evidence::FileProof {
        match self {
            Self::Semantic(call) => &call.diagnostics,
            Self::Process(call) => &call.stderr,
        }
    }
    pub(crate) fn returned_success(&self) -> bool {
        match self {
            Self::Semantic(call) => call.returned_success,
            Self::Process(call) => call.status == process::ProcessStatus::Passed,
        }
    }
    pub(crate) fn reported_code(&self) -> Option<i32> {
        match self {
            Self::Semantic(call) => Some(i32::from(call.protocol_code)),
            Self::Process(call) => call.exit_code,
        }
    }
    pub(crate) fn elapsed_nanoseconds(&self) -> u64 {
        match self {
            Self::Semantic(call) => call.elapsed_nanoseconds,
            Self::Process(call) => call.elapsed_nanoseconds,
        }
    }
    pub(crate) fn as_process(&self) -> Option<&process::ProcessObservation> {
        match self {
            Self::Process(call) => Some(call),
            Self::Semantic(_) => None,
        }
    }
    pub(crate) fn as_semantic(&self) -> Option<&SemanticObservation> {
        match self {
            Self::Semantic(call) => Some(call),
            Self::Process(_) => None,
        }
    }
    pub(crate) fn require_outcome(&self, expected_success: bool) -> Result<(), DevError> {
        let valid = match self {
            Self::Semantic(call) => {
                call.schema == "lkjscript-linked-semantic-call-2"
                    && !call.operation.is_empty()
                    && call.returned_success == expected_success
                    && (call.protocol_code == 0) == expected_success
            }
            Self::Process(call) => {
                call.signal.is_none()
                    && call.status
                        == if expected_success {
                            process::ProcessStatus::Passed
                        } else {
                            process::ProcessStatus::Failed
                        }
                    && call
                        .exit_code
                        .is_some_and(|code| (code == 0) == expected_success)
            }
        };
        require(
            valid,
            "operation evidence has another outcome or execution owner",
        )
    }
    pub(crate) fn require_arguments(&self, arguments: &[String]) -> Result<(), DevError> {
        match self {
            Self::Semantic(call) => require(
                call.backend == SemanticBackend::Filesystem
                    && call.arguments == arguments
                    && call.operation == operation(arguments)?.0,
                "filesystem public operation backend or arguments changed",
            ),
            Self::Process(_) => Ok(()),
        }
    }
    pub(crate) fn require_public_response(
        &self,
        records: &[platform::control::CompactRecord],
    ) -> Result<(), DevError> {
        let Some(call) = self.as_semantic() else {
            return Ok(());
        };
        let first = records
            .iter()
            .find(|record| record.operation == "diagnostic");
        let field = |name: &str| {
            first
                .and_then(|record| record.fields.iter().find(|field| field.name == name))
                .map(|field| field.value.as_str())
        };
        let expected = if call.returned_success {
            0
        } else if matches!(
            field("code"),
            Some("change_stale_base" | "change_authored_stale_base")
        ) {
            7
        } else {
            match field("class") {
                Some("source" | "semantic") => 2,
                Some("capability" | "cancelled") => 3,
                Some("resource") => 4,
                Some("corrupt") => 5,
                Some("infrastructure") => 6,
                _ => {
                    return Err(DevError::corrupt(
                        "linked diagnostic omitted its exact public class",
                    ));
                }
            }
        };
        require(
            call.protocol_code == expected && (first.is_none() == call.returned_success),
            "linked call diagnostic protocol code or returned result substituted",
        )
    }
    pub(crate) fn verify_files(
        &self,
        files: &[evidence::FileProof],
        label: &str,
    ) -> Result<(), DevError> {
        for (proof, suffix) in [(self.stdout(), "stdout"), (self.stderr(), "stderr")] {
            require(
                proof.path == format!("{label}.{suffix}")
                    && proof.kind == evidence::FileKind::File
                    && files.iter().filter(|file| *file == proof).count() == 1,
                &format!(
                    "operation evidence is missing or substituted: expected {label}.{suffix}, observed {}",
                    proof.path
                ),
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn execute(
    arguments: &[String],
    response_path: &Path,
    diagnostic_path: &Path,
    evidence_root: &Path,
) -> Result<CallObservation, DevError> {
    execute_with_cancellation(
        arguments,
        None,
        response_path,
        diagnostic_path,
        evidence_root,
        std::future::pending(),
    )
}

/// Memory authority is explicit and produces a distinct backend tag. Disk receipt
/// readers cannot admit it as an observation of their retained project paths.
pub(crate) fn execute_with_cancellation(
    arguments: &[String],
    memory_project: Option<&platform::MemProjectStore>,
    response_path: &Path,
    diagnostic_path: &Path,
    evidence_root: &Path,
    cancellation: impl Future<Output = ()> + Send,
) -> Result<CallObservation, DevError> {
    require_explicit_paths(arguments, memory_project.is_some())?;
    let (operation, local) = operation(arguments)?;
    let started = Instant::now();
    let single = |result: Result<Vec<u8>, Diagnostic>| result.map_err(|error| vec![error]);
    let backend = if memory_project.is_some() {
        SemanticBackend::Memory
    } else {
        SemanticBackend::Filesystem
    };
    let result = if let Some(project) = memory_project {
        platform::execute_in_memory(project, arguments.to_vec())
    } else {
        match operation {
            "new" => single(platform::execute_new(&local[1..])),
            "capabilities" => single(platform::execute_capabilities(&local[1..])),
            "status" => single(platform::execute_status(arguments.to_vec())),
            "inspect" => single(platform::execute_inspect(arguments.to_vec())),
            "query" => single(platform::execute_query(arguments.to_vec())),
            "change" => platform::execute_change(arguments.to_vec()),
            "check" => single(platform::execute_check(arguments.to_vec())),
            "build" => single(platform::execute_build(arguments.to_vec())),
            "package" => single(platform::execute_package_builtin(arguments.to_vec())),
            "data" => single(platform::execute_data(&local[1..])),
            "run"
                if local.iter().any(|value| {
                    matches!(value.as_str(), "--batch-input-file" | "--batch-result-file")
                }) =>
            {
                match platform::cli::parse_run_batch(arguments) {
                    Ok(options) => single(
                        runtime()?
                            .block_on(platform::cli::execute_run_batch(options, cancellation)),
                    ),
                    Err(error) => Err(vec![error]),
                }
            }
            "run" if local.iter().any(|value| value == "--deployment") => {
                match platform::parse_foreground_run(arguments) {
                    Ok(options) => single(
                        runtime()?
                            .block_on(platform::execute_foreground_run(options, cancellation)),
                    ),
                    Err(error) => Err(vec![error]),
                }
            }
            "run" => single(platform::execute_run(arguments.to_vec())),
            _ => {
                return Err(DevError::usage(format!(
                    "linked semantic owner does not expose {operation}"
                )));
            }
        }
    };
    let (returned_success, protocol_code, response) = match result {
        Ok(bytes) => (true, 0, bytes),
        Err(errors) => {
            let first = errors
                .first()
                .ok_or_else(|| DevError::corrupt("public operation returned no diagnostic"))?;
            (
                false,
                if matches!(
                    first.code.as_str(),
                    "change_stale_base" | "change_authored_stale_base"
                ) {
                    7
                } else {
                    exit_status_for(first.class)
                },
                failure_bytes(operation, &errors)?,
            )
        }
    };
    publish_call(
        backend,
        operation,
        arguments,
        returned_success,
        protocol_code,
        started,
        &response,
        response_path,
        diagnostic_path,
        evidence_root,
    )
}

pub(crate) fn observe_json(
    operation: &str,
    arguments: &[String],
    response_path: &Path,
    diagnostic_path: &Path,
    evidence_root: &Path,
    run: impl FnOnce() -> Result<serde_json::Value, Diagnostic>,
) -> Result<CallObservation, DevError> {
    let started = Instant::now();
    let (success, code, bytes) = match run() {
        Ok(value) => {
            // Probe commands emit compact JSON. Preserve every value without
            // spending their response budget on evidence-file indentation.
            let mut bytes = serde_json::to_vec(&value)?;
            bytes.push(b'\n');
            (true, 0, bytes)
        }
        Err(error) => (
            false,
            exit_status_for(error.class),
            failure_bytes(operation, &[error])?,
        ),
    };
    publish_call(
        SemanticBackend::Filesystem,
        operation,
        arguments,
        success,
        code,
        started,
        &bytes,
        response_path,
        diagnostic_path,
        evidence_root,
    )
}

pub(crate) fn runtime() -> Result<&'static tokio::runtime::Runtime, DevError> {
    static RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| DevError::infrastructure(error.clone()))
}

pub(crate) fn environment_names() -> Result<Vec<String>, DevError> {
    std::env::vars_os()
        .map(|(name, _)| {
            name.into_string()
                .map_err(|_| DevError::corrupt("linked host has a non-UTF-8 environment name"))
        })
        .collect::<Result<std::collections::BTreeSet<_>, _>>()
        .map(|names| names.into_iter().collect())
}

fn operation(arguments: &[String]) -> Result<(&str, &[String]), DevError> {
    let local = if arguments.first().is_some_and(|value| value == "--project") {
        arguments
            .get(2..)
            .ok_or_else(|| DevError::usage("linked operation has no project value"))?
    } else {
        arguments
    };
    let operation = local
        .first()
        .ok_or_else(|| DevError::usage("linked operation is empty"))?;
    Ok((operation, local))
}

fn require_explicit_paths(arguments: &[String], memory_project: bool) -> Result<(), DevError> {
    let (operation, local) = operation(arguments)?;
    require(
        operation != "new"
            || local
                .get(1)
                .is_some_and(|value| Path::new(value).is_absolute()),
        "linked new requires an absolute owned path",
    )?;
    for pair in arguments.windows(2) {
        if matches!(
            pair[0].as_str(),
            "--project"
                | "--input-file"
                | "--output"
                | "--deployment"
                | "--arguments-file"
                | "--result-file"
                | "--root"
                | "--batch-input-file"
                | "--batch-result-file"
        ) {
            require(
                Path::new(&pair[1]).is_absolute(),
                "linked operation requires explicit absolute paths",
            )?;
        }
    }
    let independent = matches!(operation, "new" | "capabilities" | "data")
        || local.starts_with(&["package".to_owned(), "builtin".to_owned()])
        || (operation == "run" && local.iter().any(|value| value == "--deployment"));
    require(
        memory_project
            || independent
            || arguments.first().is_some_and(|value| value == "--project"),
        "linked operation cannot use implicit project discovery",
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "one returned call binds its complete evidence destinations"
)]
fn publish_call(
    backend: SemanticBackend,
    operation: &str,
    arguments: &[String],
    returned_success: bool,
    protocol_code: u8,
    started: Instant,
    response: &[u8],
    response_path: &Path,
    diagnostic_path: &Path,
    evidence_root: &Path,
) -> Result<CallObservation, DevError> {
    require(
        response_path.parent() == Some(evidence_root)
            && diagnostic_path.parent() == Some(evidence_root),
        "linked evidence files must belong to the owned evidence root",
    )?;
    std::fs::write(response_path, response)?;
    std::fs::write(diagnostic_path, [])?;
    let response_name = response_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| DevError::corrupt("linked response evidence filename"))?;
    let diagnostic_name = diagnostic_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| DevError::corrupt("linked diagnostic evidence filename"))?;
    Ok(CallObservation::Semantic(SemanticObservation {
        schema: "lkjscript-linked-semantic-call-2".into(),
        backend,
        operation: operation.into(),
        arguments: arguments.to_vec(),
        returned_success,
        protocol_code,
        elapsed_nanoseconds: u64::try_from(started.elapsed().as_nanos())
            .map_err(|_| DevError::infrastructure("linked call elapsed time overflow"))?,
        response: evidence::proof(response_path, response_name.to_owned())?,
        diagnostics: evidence::proof(diagnostic_path, diagnostic_name.to_owned())?,
    }))
}

fn failure_bytes(command: &str, diagnostics: &[Diagnostic]) -> Result<Vec<u8>, DevError> {
    let mut output = CompactResponseWriter::new(CompactResponseLimits::default())
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    output
        .append_record("result", &[("status", "failure"), ("command", command)])
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    for error in diagnostics {
        let mut fields = vec![
            ("class", diagnostic_class_name(error.class).to_owned()),
            ("code", error.code.clone()),
            ("message", error.message.clone()),
        ];
        if let Some(location) = &error.location {
            fields.extend([
                ("path", location.path.clone()),
                ("line", location.line.to_string()),
                ("column", location.column.to_string()),
            ]);
        }
        if !error.notes.is_empty() {
            fields.push(("notes", serde_json::to_string(&error.notes)?));
        }
        output
            .append_record(
                "diagnostic",
                &fields
                    .iter()
                    .map(|(key, value)| (*key, value.as_str()))
                    .collect::<Vec<_>>(),
            )
            .map_err(|error| DevError::corrupt(error.to_string()))?;
    }
    Ok(output.finish())
}

fn require(condition: bool, message: &str) -> Result<(), DevError> {
    if condition {
        Ok(())
    } else {
        Err(DevError::corrupt(message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform::control::{CompactRecord, parse_records};

    #[test]
    #[allow(
        clippy::result_large_err,
        reason = "probe callbacks preserve the public typed diagnostic boundary"
    )]
    fn json_probe_keeps_complete_values_beyond_the_former_response_bound() {
        let root = tempfile::tempdir().unwrap();
        let former_default_bytes = 4 * 1024 * 1024;
        let value = serde_json::Value::String("x".repeat(former_default_bytes + 1));
        let mut expected = serde_json::to_vec(&value).unwrap();
        expected.push(b'\n');
        assert!(expected.len() > former_default_bytes);
        let response = root.path().join("probe.stdout");
        let diagnostic = root.path().join("probe.stderr");
        let observed = observe_json(
            "contributor.probe",
            &[],
            &response,
            &diagnostic,
            root.path(),
            || Ok(value),
        )
        .unwrap();
        observed.require_outcome(true).unwrap();
        assert_eq!(std::fs::read(&response).unwrap(), expected);
        assert!(std::fs::read(&diagnostic).unwrap().is_empty());
    }

    #[test]
    fn failure_projection_keeps_every_diagnostic_beyond_former_record_defaults() {
        let diagnostics = (0..10_001)
            .map(|index| {
                Diagnostic::new(
                    platform::DiagnosticClass::Semantic,
                    "complete_diagnostic",
                    index.to_string(),
                )
            })
            .collect::<Vec<_>>();
        let bytes = failure_bytes("change", &diagnostics).unwrap();
        let records = parse_records("response", &bytes).unwrap();
        assert_eq!(records.len(), diagnostics.len() + 1);
        for (record, diagnostic) in records[1..].iter().zip(&diagnostics) {
            assert_eq!(record.operation, "diagnostic");
            assert_eq!(
                record
                    .fields
                    .iter()
                    .find(|field| field.name == "message")
                    .unwrap()
                    .value,
                diagnostic.message
            );
        }
    }

    fn field(records: &[CompactRecord], name: &str) -> String {
        records
            .iter()
            .find(|record| record.operation == "diagnostic")
            .unwrap()
            .fields
            .iter()
            .find(|field| field.name == name)
            .unwrap()
            .value
            .clone()
    }

    #[test]
    fn result_only_batch_reaches_public_batch_preflight_without_effects() {
        let root = tempfile::tempdir().unwrap();
        let result = root.path().join("never-published.jsonl");
        let arguments = vec![
            "run".into(),
            "--deployment".into(),
            root.path()
                .join("missing.deployment.json")
                .display()
                .to_string(),
            "--batch-result-file".into(),
            result.display().to_string(),
        ];
        let expected = platform::cli::parse_run_batch(&arguments).err().unwrap();
        let call = execute(
            &arguments,
            &root.path().join("call.stdout"),
            &root.path().join("call.stderr"),
            root.path(),
        )
        .unwrap();
        let bytes = std::fs::read(root.path().join("call.stdout")).unwrap();
        let records = parse_records("batch-preflight", &bytes).unwrap();
        assert_eq!(field(&records, "code"), expected.code);
        call.require_outcome(false).unwrap();
        call.require_arguments(&arguments).unwrap();
        call.require_public_response(&records).unwrap();
        assert!(!result.exists());
        let json = serde_json::to_value(&call).unwrap();
        assert_eq!(json["engine"], "semantic");
        assert!(json["observation"].get("status").is_none());
        assert!(json["observation"].get("exit_code").is_none());
        assert!(json["observation"].get("cwd").is_none());
    }

    #[test]
    fn memory_authority_rejects_a_filesystem_project_selector() {
        let root = tempfile::tempdir().unwrap();
        let project = platform::MemProjectStore::new(None).unwrap();
        let forbidden = root.path().join("unselected-project");
        let arguments = vec![
            "--project".into(),
            forbidden.display().to_string(),
            "status".into(),
        ];
        let call = execute_with_cancellation(
            &arguments,
            Some(&project),
            &root.path().join("response"),
            &root.path().join("diagnostics"),
            root.path(),
            std::future::pending(),
        )
        .unwrap();
        call.require_outcome(false).unwrap();
        assert!(!forbidden.exists());
        let records = parse_records(
            "memory rejection",
            &std::fs::read(root.path().join("response")).unwrap(),
        )
        .unwrap();
        call.require_public_response(&records).unwrap();
        assert!(call.require_arguments(&arguments).is_err());
        assert_eq!(
            serde_json::to_value(&call).unwrap()["observation"]["backend"],
            "memory"
        );
    }

    #[test]
    fn reviewed_current_request_retains_stale_base_protocol_seven() {
        let root = tempfile::tempdir().unwrap();
        let project = platform::MemProjectStore::new(None).unwrap();
        platform::create_memory_project(&project, "current-review", "minimal").unwrap();
        let sibling = project.fork().unwrap();
        let mut index = 0;
        let mut invoke = |project: &platform::MemProjectStore, args: Vec<String>| {
            let call = execute_with_cancellation(
                &args,
                Some(project),
                &root.path().join(format!("call-{index}.stdout")),
                &root.path().join(format!("call-{index}.stderr")),
                root.path(),
                std::future::pending(),
            )
            .unwrap();
            let records = parse_records(
                "linked-current-review",
                &std::fs::read(root.path().join(format!("call-{index}.stdout"))).unwrap(),
            )
            .unwrap();
            index += 1;
            call.require_public_response(&records).unwrap();
            let observed = call.as_semantic().unwrap();
            assert_eq!(observed.backend, SemanticBackend::Memory);
            assert_eq!(observed.arguments, args);
            assert!(call.require_arguments(&args).is_err());
            (call, records)
        };
        let (_, created) = invoke(&project, vec!["status".into()]);
        let base = created
            .iter()
            .find(|record| record.operation == "revision")
            .unwrap()
            .fields
            .iter()
            .find(|field| field.name == "id")
            .unwrap()
            .value
            .clone();
        let request =
            format!("request base={base}\ncreate.module as=$module name=current-review\n");
        let prefix = vec!["change".into()];
        let (_, planned) = invoke(
            &project,
            prefix
                .iter()
                .cloned()
                .chain(["plan".into(), "--input".into(), request.clone()])
                .collect(),
        );
        let token = planned
            .iter()
            .find(|record| record.operation == "plan")
            .unwrap()
            .fields
            .iter()
            .find(|field| field.name == "token")
            .unwrap()
            .value
            .clone();
        let (applied, _) = invoke(
            &project,
            prefix
                .iter()
                .cloned()
                .chain([
                    "apply".into(),
                    "--input".into(),
                    request.clone(),
                    "--plan".into(),
                    token,
                ])
                .collect(),
        );
        applied.require_outcome(true).unwrap();
        let (stale, records) = invoke(
            &project,
            prefix
                .iter()
                .cloned()
                .chain(["plan".into(), "--input".into(), request.clone()])
                .collect(),
        );
        assert_eq!(field(&records, "code"), "change_authored_stale_base");
        assert_eq!(stale.reported_code(), Some(7));
        stale.require_outcome(false).unwrap();
        let (unchanged, _) = invoke(
            &sibling,
            prefix
                .into_iter()
                .chain(["plan".into(), "--input".into(), request])
                .collect(),
        );
        unchanged.require_outcome(true).unwrap();
        assert!(std::fs::read_dir(root.path()).unwrap().all(|entry| {
            let entry = entry.unwrap();
            entry.file_type().unwrap().is_file()
                && entry.file_name().to_string_lossy().starts_with("call-")
        }));
    }
}
