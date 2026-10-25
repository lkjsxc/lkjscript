//! Finite pure command batches retain the ordinary per-row foreground lifecycle.

use super::*;
use crate::platform::contract::exit_status_for;
use crate::platform::deployment::pure_batch::{PreparedPureRunBatch, close_uninvoked_row};
use crate::platform::json::{JsonIntegerPolicy, JsonLimits, decode_strict_with_integer_policy};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub const MAXIMUM_BATCH_ROWS: usize = 4_096;
pub const MAXIMUM_BATCH_BYTES: usize = 64 * 1024 * 1024;
pub const MAXIMUM_BATCH_ROW_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    pub(super) id: String,
    pub(super) arguments_json: String,
    #[serde(default)]
    pub(super) target: Option<String>,
    #[serde(default)]
    pub(super) execution: Option<super::super::execution::RunPolicy>,
    #[serde(default)]
    pub(super) deadline_milliseconds: Option<u64>,
}

#[derive(Serialize)]
pub(super) struct Response {
    pub(super) id: String,
    pub(super) exit_code: u8,
    pub(super) stdout: String,
    pub(super) result_json: Option<String>,
}

pub struct PureRunBatchOptions {
    descriptor: PathBuf,
    requests: Vec<Request>,
    output: PathBuf,
}

pub fn parse_pure_run_batch(arguments: &[String]) -> Result<PureRunBatchOptions, Diagnostic> {
    if arguments.first().map(String::as_str) != Some("run") {
        return Err(usage_error("pure batches require the run operation"));
    }
    ensure_options(
        &arguments[1..],
        &["--deployment", "--batch-input-file", "--batch-result-file"],
        &[],
    )?;
    let descriptor = bounded_path(required_option(&arguments[1..], "--deployment")?)?;
    let input = bounded_path(required_option(&arguments[1..], "--batch-input-file")?)?;
    let output = bounded_path(required_option(&arguments[1..], "--batch-result-file")?)?;
    let output = super::super::owned_output::inspect_create_new(&output)?;
    let bytes = read_bounded(&input, MAXIMUM_BATCH_BYTES, "pure batch input")?;
    let requests = decode_requests(&bytes)?;
    Ok(PureRunBatchOptions {
        descriptor,
        requests,
        output,
    })
}

pub(super) fn bounded_path(value: String) -> Result<PathBuf, Diagnostic> {
    if value.is_empty() || value.len() > 4_096 || value.contains('\0') {
        return Err(usage_error(
            "batch paths must contain 1 to 4096 UTF-8 bytes without NUL",
        ));
    }
    Ok(value.into())
}

pub(super) fn decode_requests(bytes: &[u8]) -> Result<Vec<Request>, Diagnostic> {
    if bytes.len() > MAXIMUM_BATCH_BYTES {
        return Err(batch_error(
            DiagnosticClass::Resource,
            "run_batch_input_limit",
            "batch input exceeds 64 MiB",
        ));
    }
    let mut requests = Vec::new();
    let mut ids = BTreeSet::new();
    let mut lines = bytes.split(|byte| *byte == b'\n').peekable();
    while let Some(line) = lines.next() {
        if line.is_empty() && lines.peek().is_none() {
            break;
        }
        if requests.len() == MAXIMUM_BATCH_ROWS || line.len() > MAXIMUM_BATCH_ROW_BYTES {
            return Err(batch_error(
                DiagnosticClass::Resource,
                "run_batch_input_limit",
                "batch exceeds its row count or encoded row byte bound",
            ));
        }
        // Numeric policy applies only to framing and controls; application bytes remain exact
        // inside arguments_json and cross ordinary typed ingress independently for each row.
        let value = decode_strict_with_integer_policy(
            line,
            JsonLimits {
                maximum_bytes: MAXIMUM_BATCH_ROW_BYTES,
                maximum_string_bytes: MAXIMUM_BATCH_ROW_BYTES,
                ..JsonLimits::default()
            },
            JsonIntegerPolicy::SignedOrUnsigned64,
        )
        .map_err(|error| {
            batch_error(
                DiagnosticClass::Source,
                "run_batch_request",
                format!(
                    "batch row {} is not strict JSON: {}",
                    requests.len(),
                    error.code
                ),
            )
        })?;
        let request: Request = serde_json::from_value(value).map_err(|error| {
            batch_error(
                DiagnosticClass::Source,
                "run_batch_request",
                format!("batch row {} has invalid fields: {error}", requests.len()),
            )
        })?;
        if request.id.is_empty() || request.id.len() > 128 || request.id.contains('\0') {
            return Err(batch_error(
                DiagnosticClass::Source,
                "run_batch_id",
                "batch row IDs require 1 to 128 UTF-8 bytes without NUL",
            ));
        }
        if !ids.insert(request.id.clone()) {
            return Err(batch_error(
                DiagnosticClass::Source,
                "run_batch_id",
                "batch row IDs must be unique",
            ));
        }
        requests.push(request);
    }
    if requests.is_empty() {
        return Err(batch_error(
            DiagnosticClass::Source,
            "run_batch_request",
            "a pure batch must contain at least one row",
        ));
    }
    Ok(requests)
}

pub async fn execute_pure_run_batch(
    options: PureRunBatchOptions,
    cancellation: impl std::future::Future<Output = ()>,
) -> Result<Vec<u8>, Diagnostic> {
    let control = ExecutionControl::uncancelled();
    let preparation_control = control.clone();
    let descriptor = options.descriptor;
    let mut preparing = tokio::task::spawn_blocking(move || {
        PreparedPureRunBatch::load(&descriptor, &preparation_control)
    });
    tokio::pin!(cancellation);
    let prepared = Arc::new(tokio::select! {
        biased;
        result = &mut preparing => result.map_err(foreground_join_error)??,
        () = cancellation.as_mut() => {
            control.cancel();
            let _ = preparing.await.map_err(foreground_join_error)?;
            return Err(batch_error(DiagnosticClass::Cancelled, "execution_cancelled", "pure batch preparation was cancelled"));
        }
    });
    let mut encoded = Vec::new();
    let mut passed = 0_usize;
    let mut failed = 0_usize;
    let rows = options.requests.len();
    for request in options.requests {
        batch_cancellation_checkpoint(cancellation.as_mut()).await?;
        let started = std::time::Instant::now();
        let deadline_milliseconds = request
            .deadline_milliseconds
            .or_else(|| prepared.deadline_milliseconds());
        let control = row_control(deadline_milliseconds);
        let Request {
            id,
            target,
            arguments_json,
            execution,
            ..
        } = request;
        let row = match control.as_ref() {
            Ok(control) => {
                let owner = Arc::clone(&prepared);
                let preparation_control = control.clone();
                let runtime = tokio::runtime::Handle::current();
                let mut preparing = tokio::task::spawn_blocking(move || {
                    owner.prepare_row(
                        target.as_deref(),
                        arguments_json.as_bytes(),
                        execution,
                        runtime,
                        &preparation_control,
                    )
                });
                tokio::select! {
                    biased;
                    result = &mut preparing => result.map_err(foreground_join_error)?,
                    () = cancellation.as_mut() => {
                        control.cancel();
                        let joined = preparing.await.map_err(foreground_join_error)?;
                        let mut error = batch_error(DiagnosticClass::Cancelled, "execution_cancelled", "pure batch row preparation was cancelled");
                        match joined {
                            Ok((row, _)) => error = close_uninvoked_row(row, error),
                            Err(failure) => error.notes.extend(failure.notes),
                        }
                        return Err(error);
                    }
                }
            }
            Err(error) => Err(error.clone()),
        };
        let preparation_nanoseconds = started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
        let externally_cancelled = AtomicBool::new(false);
        let deadline_expired = AtomicBool::new(false);
        let row = row.and_then(|(row, invocation)| {
            match control.as_ref().map_err(Clone::clone)?.check() {
                Ok(()) => Ok((row, invocation)),
                Err(error) => Err(close_uninvoked_row(
                    row,
                    crate::platform::execution::normalized::execution_diagnostic(error),
                )),
            }
        });
        let outcome = match row {
            Ok((row, invocation)) => {
                let row_cancellation = async {
                    let deadline = async {
                        if let Some(deadline) =
                            control.as_ref().ok().and_then(ExecutionControl::deadline)
                        {
                            tokio::time::sleep_until(deadline.into()).await;
                        } else {
                            std::future::pending::<()>().await;
                        }
                    };
                    tokio::select! {
                        biased;
                        () = cancellation.as_mut() => externally_cancelled.store(true, Ordering::Relaxed),
                        () = deadline => deadline_expired.store(true, Ordering::Relaxed),
                    }
                };
                row.run_foreground(invocation, row_cancellation).await
            }
            Err(error) => Err(error),
        };
        if externally_cancelled.load(Ordering::Relaxed) {
            return Err(outcome.err().unwrap_or_else(|| {
                batch_error(
                    DiagnosticClass::Cancelled,
                    "execution_cancelled",
                    "pure batch was cancelled after its current row joined",
                )
            }));
        }
        let (exit_code, stdout, result_json) = match outcome {
            Ok(mut receipt) => {
                let result =
                    String::from_utf8(std::mem::take(&mut receipt.result_json)).map_err(|_| {
                        batch_error(
                            DiagnosticClass::Corrupt,
                            "run_result_utf8",
                            "typed batch result is not UTF-8 JSON",
                        )
                    })?;
                match foreground_receipt_output(receipt, None, preparation_nanoseconds, true) {
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
            Err(mut error) => {
                if deadline_expired.load(Ordering::Relaxed) && error.code == "execution_cancelled" {
                    error.code = "execution_deadline".to_owned();
                    error.message = "execution exceeded its operational deadline".to_owned();
                }
                failed += 1;
                (exit_status_for(error.class), failure_bytes(&error)?, None)
            }
        };
        let stdout = String::from_utf8(stdout).map_err(|_| {
            batch_error(
                DiagnosticClass::Infrastructure,
                "run_batch_response",
                "batch row projection is not UTF-8",
            )
        })?;
        let response = Response {
            id,
            exit_code,
            stdout,
            result_json,
        };
        append_response(&mut encoded, &response)?;
    }
    batch_cancellation_checkpoint(cancellation.as_mut()).await?;
    let mut output = compact_response_writer()?;
    append_compact_record(
        &mut output,
        "result",
        &[
            ("status", "success".to_owned()),
            ("command", "run.batch".to_owned()),
        ],
    )?;
    append_compact_record(
        &mut output,
        "batch",
        &[
            ("artifact-sha256", prepared.artifact_sha256.clone()),
            ("rows", rows.to_string()),
            ("passed", passed.to_string()),
            ("failed", failed.to_string()),
            ("artifact-preparations", "1".to_owned()),
            ("result-file", options.output.display().to_string()),
        ],
    )?;
    let publication = publish_create_new(
        &options.output,
        &encoded,
        Some(MAXIMUM_BATCH_BYTES),
        "pure batch result",
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

pub(super) fn row_control(
    deadline_milliseconds: Option<u64>,
) -> Result<ExecutionControl, Diagnostic> {
    match deadline_milliseconds {
        Some(milliseconds) => Ok(ExecutionControl::with_deadline(
            std::time::Instant::now()
                .checked_add(std::time::Duration::from_millis(milliseconds))
                .ok_or_else(|| {
                    batch_error(
                        DiagnosticClass::Resource,
                        "run_batch_deadline",
                        "row deadline is not representable on this platform",
                    )
                })?,
        )),
        None => Ok(ExecutionControl::uncancelled()),
    }
}

pub(super) async fn batch_cancellation_checkpoint(
    cancellation: std::pin::Pin<&mut impl std::future::Future<Output = ()>>,
) -> Result<(), Diagnostic> {
    tokio::select! {
        biased;
        () = cancellation => Err(batch_error(DiagnosticClass::Cancelled, "execution_cancelled",
            "pure batch was cancelled before aggregate publication")),
        () = std::future::ready(()) => Ok(()),
    }
}

pub(super) fn failure_bytes(error: &Diagnostic) -> Result<Vec<u8>, Diagnostic> {
    let mut output = compact_response_writer()?;
    append_compact_record(
        &mut output,
        "result",
        &[
            ("status", "failure".to_owned()),
            ("command", "run".to_owned()),
        ],
    )?;
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
        fields.push((
            "notes",
            serde_json::to_string(&error.notes).map_err(|_| {
                batch_error(
                    DiagnosticClass::Infrastructure,
                    "run_batch_response",
                    "batch diagnostic notes could not be encoded",
                )
            })?,
        ));
    }
    append_compact_record(&mut output, "diagnostic", &fields)?;
    Ok(output.finish())
}

fn batch_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

pub(super) fn append_response(
    encoded: &mut Vec<u8>,
    response: &Response,
) -> Result<(), Diagnostic> {
    let row = serde_json::to_vec(response).map_err(|_| {
        batch_error(
            DiagnosticClass::Infrastructure,
            "run_batch_response",
            "batch row could not be encoded",
        )
    })?;
    let next = encoded
        .len()
        .checked_add(row.len())
        .and_then(|bytes| bytes.checked_add(1));
    if row.len() > MAXIMUM_BATCH_ROW_BYTES || next.is_none_or(|bytes| bytes > MAXIMUM_BATCH_BYTES) {
        return Err(batch_error(
            DiagnosticClass::Resource,
            "run_batch_output_limit",
            "batch output exceeds its encoded row or aggregate byte bound",
        ));
    }
    encoded.try_reserve(row.len() + 1).map_err(|_| {
        batch_error(
            DiagnosticClass::Resource,
            "run_batch_output_limit",
            "batch output storage could not be reserved",
        )
    })?;
    encoded.extend_from_slice(&row);
    encoded.push(b'\n');
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn batch_framing_preserves_raw_application_bytes_and_unsigned_controls() {
        let bytes = serde_json::to_vec(&json!({
            "id":"late-ingress", "arguments_json":"[true] trailing",
            "target":"main", "deadline_milliseconds":u64::MAX,
            "execution":{"instruction_fuel":u64::MAX,"maximum_call_depth":4096,"maximum_value_stack":1000000}
        })).unwrap();
        let requests = decode_requests(&bytes).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].arguments_json, "[true] trailing");
        assert_eq!(requests[0].deadline_milliseconds, Some(u64::MAX));
        assert_eq!(
            requests[0].execution.unwrap().instruction_fuel,
            Some(u64::MAX)
        );
    }

    #[test]
    fn batch_framing_refuses_duplicate_keys_ids_unknown_fields_and_late_corruption() {
        for (bytes, code) in [
            (b"".as_slice(), "run_batch_request"),
            (b"\n".as_slice(), "run_batch_request"),
            (br#"{"id":"a","id":"b","arguments_json":"[]"}"#.as_slice(), "run_batch_request"),
            (br#"{"id":"a","arguments_json":"[]","unknown":true}"#.as_slice(), "run_batch_request"),
            (b"{\"id\":\"a\",\"arguments_json\":\"[]\"}\n{\"id\":\"a\",\"arguments_json\":\"[]\"}".as_slice(), "run_batch_id"),
            (b"{\"id\":\"a\",\"arguments_json\":\"[]\"}\n{".as_slice(), "run_batch_request"),
            (br#"{"id":"a","arguments_json":"[]","execution":{"instruction_fuel":1,"instruction_fuel":2,"maximum_call_depth":4,"maximum_value_stack":4}}"#.as_slice(), "run_batch_request"),
        ] {
            assert_eq!(decode_requests(bytes).unwrap_err().code, code);
        }
        assert_eq!(
            decode_requests(b"{\"id\":\"a\",\"arguments_json\":\"[]\"}\n")
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn batch_framing_enforces_inclusive_row_count_and_encoded_row_bounds() {
        let mut bytes = Vec::new();
        for id in 0..MAXIMUM_BATCH_ROWS {
            serde_json::to_writer(
                &mut bytes,
                &json!({"id":id.to_string(),"arguments_json":"[]"}),
            )
            .unwrap();
            bytes.push(b'\n');
        }
        assert_eq!(decode_requests(&bytes).unwrap().len(), MAXIMUM_BATCH_ROWS);
        bytes.extend_from_slice(b"{\"id\":\"extra\",\"arguments_json\":\"[]\"}\n");
        assert_eq!(
            decode_requests(&bytes).unwrap_err().code,
            "run_batch_input_limit"
        );
        let prefix = br#"{"id":"row","arguments_json":""#;
        let suffix = br#""}"#;
        let mut bytes = prefix.to_vec();
        bytes.resize(MAXIMUM_BATCH_ROW_BYTES - suffix.len(), b' ');
        bytes.extend_from_slice(suffix);
        assert_eq!(bytes.len(), MAXIMUM_BATCH_ROW_BYTES);
        assert_eq!(decode_requests(&bytes).unwrap().len(), 1);
        bytes.push(b' ');
        assert_eq!(
            decode_requests(&bytes).unwrap_err().code,
            "run_batch_input_limit"
        );
    }

    #[tokio::test]
    async fn batch_publication_checkpoint_observes_cancellation_between_rows() {
        let mut cancelled = std::pin::pin!(std::future::ready(()));
        let error = batch_cancellation_checkpoint(cancelled.as_mut())
            .await
            .unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Cancelled);
        assert_eq!(error.code, "execution_cancelled");
        let mut pending = std::pin::pin!(std::future::pending());
        assert!(
            batch_cancellation_checkpoint(pending.as_mut())
                .await
                .is_ok()
        );
    }
}
