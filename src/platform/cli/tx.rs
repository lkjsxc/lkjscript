//! Single-call symbolic authoring with differential verification before atomic publication.

use super::*;
use crate::platform::normalized_lifecycle::PreparedPreview;

#[cfg(test)]
mod tests;

pub fn execute_tx(arguments: Vec<String>) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let project = memory::current();
    std::thread::Builder::new()
        .name("symbolic-transaction".into())
        .stack_size(64 * 1_048_576)
        .spawn(move || memory::with_project(project, || execute(arguments)))
        .map_err(|error| {
            single_diagnostic(internal_error(format!(
                "cannot start transaction worker: {error}"
            )))
        })?
        .join()
        .map_err(|_| single_diagnostic(internal_error("transaction worker panicked")))?
}

fn execute(arguments: Vec<String>) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let (arguments, project) = extract_global_project(arguments).map_err(single_diagnostic)?;
    if arguments.first().map(String::as_str) != Some("tx") {
        return Err(single_diagnostic(usage_error(
            "transaction dispatch requires tx",
        )));
    }
    let options = &arguments[1..];
    ensure_options(
        options,
        &["--input", "--input-file", "--run"],
        &["--compact"],
    )
    .map_err(single_diagnostic)?;
    let inline = option_value(options, "--input").map_err(single_diagnostic)?;
    let file = option_value(options, "--input-file").map_err(single_diagnostic)?;
    let (source, bytes) = match (inline, file) {
        (Some(value), None) => ("<transaction-input>".into(), value.into_bytes()),
        (None, Some(path)) => {
            let bytes = read_with_limit(
                Path::new(&path),
                None,
                "transaction input",
            )
            .map_err(single_diagnostic)?;
            (path, bytes)
        }
        _ => {
            return Err(single_diagnostic(usage_error(
                "tx requires exactly one of --input or --input-file",
            )));
        }
    };
    let target = option_value(options, "--run")
        .map_err(single_diagnostic)?
        .map(|name| Name::new(&name))
        .transpose()
        .map_err(single_diagnostic)?;
    let repository = open_normalized_repository(project).map_err(single_diagnostic)?;
    let control = ExecutionControl::uncancelled();
    let view = repository.view_current().map_err(single_diagnostic)?;
    let base = view.current().head;
    let normalized =
        crate::platform::control::decode_transaction(&source, &bytes, &view, &control)?;
    let mut prepared = match view
        .prepare_authored_change(&normalized.semantic, normalized.options.clone())
    {
        Ok(prepared) => prepared,
        Err(errors) if errors.len() == 1 && errors[0].code == "publication_semantic_no_change" => {
            view.require_current_validation()
                .map_err(single_diagnostic)?;
            let run = if let Some(target) = target {
                Some(
                    PreparedPreview::current(view, &control)
                        .map_err(single_diagnostic)?
                        .run(&target, &control)
                        .map_err(single_diagnostic)?,
                )
            } else {
                None
            };
            if repository.current().map_err(single_diagnostic)?.head != base {
                return Err(single_diagnostic(Diagnostic::new(
                    DiagnosticClass::Semantic,
                    "transaction_stale",
                    "HEAD changed while verifying an unchanged transaction",
                )));
            }
            return encode_result(serde_json::json!({
                "operation": "tx", "status": "unchanged", "base": base.revision.to_string(),
                "revision": base.revision.to_string(), "created": 0, "updated": 0,
                "tests_selected": 0, "tests_passed": 0, "differential": "equal",
                "run": run_value(run)?,
            }));
        }
        Err(errors) => return Err(errors),
    };
    finish_authored_review(&mut prepared, &view, &normalized)?;
    let preview = PreparedPreview::prepare(view, &prepared.publication, &control)
        .map_err(single_diagnostic)?;
    let tests = preview
        .check(&prepared.logical_plan.tests, &control)
        .map_err(single_diagnostic)?;
    let run = target
        .as_ref()
        .map(|target| preview.run(target, &control))
        .transpose()
        .map_err(single_diagnostic)?;
    prepared
        .record_verified_tests(&tests, normalized.semantic.budget)
        .map_err(single_diagnostic)?;
    let tests = tests.receipt();
    // Encode and bound the complete result before any durable mutation.
    let result = serde_json::json!({
        "operation": "tx", "status": "accepted", "base": base.revision.to_string(),
        "revision": prepared.publication.head.revision.to_string(),
        "transaction": prepared.publication.transaction_digest.to_string(),
        "created": prepared.publication.receipt.counts.owners_created,
        "updated": prepared.publication.receipt.counts.owners_updated,
        "tests_selected": prepared.logical_plan.tests.len(),
        "tests_passed": tests.passed,
        "differential": tests.differential,
        "run": run_value(run)?,
    });
    let bytes = encode_result(result)?;
    match repository
        .publish(&prepared.publication)
        .map_err(single_diagnostic)?
    {
        GraphPublicationOutcome::Accepted { .. }
        | GraphPublicationOutcome::AlreadyAccepted { .. } => Ok(bytes),
        GraphPublicationOutcome::Stale { .. } => Err(single_diagnostic(Diagnostic::new(
            DiagnosticClass::Semantic,
            "transaction_stale",
            "HEAD changed before publication; submit a new transaction against the new HEAD",
        ))),
    }
}

fn run_value(
    run: Option<crate::platform::execution::normalized::NormalizedCommandReceipt>,
) -> Result<Option<serde_json::Value>, Vec<Diagnostic>> {
    run.map(|receipt| serde_json::from_slice(&receipt.result_json))
        .transpose()
        .map_err(|error| {
            single_diagnostic(internal_error(format!("invalid command result: {error}")))
        })
}

fn encode_result(result: serde_json::Value) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let mut bytes = serde_json::to_vec(&result)
        .map_err(|error| single_diagnostic(internal_error(error.to_string())))?;
    bytes.push(b'\n');
    if bytes.len() > crate::platform::control::MAXIMUM_TRANSACTION_RESULT_BYTES {
        return Err(single_diagnostic(Diagnostic::new(
            DiagnosticClass::Resource,
            "transaction_result_capacity",
            "transaction result exceeds 65536 bytes",
        )));
    }
    Ok(bytes)
}
