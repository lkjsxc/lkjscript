//! Public graph-native development command and generic resident runner.

#![allow(
    clippy::result_large_err,
    reason = "the process boundary preserves the complete structured diagnostic"
)]

use lkjscript::platform::contract::{
    MAXIMUM_CLI_RESPONSE_BYTES, MAXIMUM_CLI_RESPONSE_RECORDS, exit_status_for,
};
use lkjscript::platform::control::{CompactResponseLimits, CompactResponseWriter};
use lkjscript::platform::{
    Diagnostic, PreparedDeployment, PublicOperation, ShutdownReceipt, execute_build,
    execute_capabilities, execute_change, execute_check, execute_data, execute_inspect,
    execute_new, execute_package_builtin, execute_query, execute_run, execute_status,
};
use serde::Serialize;
use serde_json::json;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
use tokio::net::TcpListener;

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.as_slice() == ["--version"] {
        return product_version();
    }
    if let Some(capability_arguments) = compact_capability_arguments(&arguments) {
        return compact_capabilities(&capability_arguments);
    }
    if let Some(new_arguments) = compact_new_arguments(&arguments) {
        return compact_new(&new_arguments);
    }
    if arguments.first().map(String::as_str) == Some("data") {
        return compact_data(&arguments[1..]);
    }
    if compact_status_arguments(&arguments) {
        return compact_status(arguments);
    }
    if compact_inspect_arguments(&arguments) {
        return compact_inspect(arguments);
    }
    if compact_query_arguments(&arguments) {
        return compact_query(arguments);
    }
    if compact_change_arguments(&arguments) {
        return compact_change(arguments);
    }
    if compact_project_command(&arguments, "check") {
        return compact_check(arguments);
    }
    if compact_project_command(&arguments, "build") {
        return compact_build(arguments);
    }
    if compact_project_command(&arguments, "run") {
        return compact_run(arguments);
    }
    if compact_project_command(&arguments, "package") {
        return compact_package_builtin(arguments);
    }
    let operation = arguments
        .first()
        .and_then(|value| PublicOperation::parse(value));
    if !matches!(
        operation,
        Some(PublicOperation::Serve | PublicOperation::Worker)
    ) {
        return unknown_operation(&arguments);
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            return write_failure(&Diagnostic::new(
                lkjscript::platform::DiagnosticClass::Infrastructure,
                "runtime_initialize",
                format!("resident runtime could not initialize: {error}"),
            ));
        }
    };
    let outcome = match operation {
        Some(PublicOperation::Serve) => runtime.block_on(serve(&arguments[1..])),
        Some(PublicOperation::Worker) => runtime.block_on(worker(&arguments[1..])),
        _ => return unknown_operation(&arguments),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => write_failure(&error),
    }
}

fn compact_data(arguments: &[String]) -> ExitCode {
    match execute_data(arguments) {
        Ok(bytes) => match write_bytes(&bytes) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(exit_status_for(
                lkjscript::platform::DiagnosticClass::Infrastructure,
            )),
        },
        Err(error) => write_compact_failure("data", &error),
    }
}

fn product_version() -> ExitCode {
    let line = format!("lkjscript {}\n", lkjscript::PRODUCT_VERSION);
    match write_bytes(line.as_bytes()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(exit_status_for(
            lkjscript::platform::DiagnosticClass::Infrastructure,
        )),
    }
}

fn compact_capability_arguments(arguments: &[String]) -> Option<Vec<String>> {
    let mut filtered = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == "--project" {
            arguments.get(index + 1)?;
            index += 2;
        } else {
            filtered.push(arguments[index].clone());
            index += 1;
        }
    }
    if filtered.is_empty() {
        Some(Vec::new())
    } else if filtered.first().map(String::as_str) == Some("capabilities") {
        Some(filtered.into_iter().skip(1).collect())
    } else {
        None
    }
}

fn compact_capabilities(arguments: &[String]) -> ExitCode {
    match execute_capabilities(arguments) {
        Ok(bytes) => match write_bytes(&bytes) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(exit_status_for(
                lkjscript::platform::DiagnosticClass::Infrastructure,
            )),
        },
        Err(error) => write_compact_failure("capabilities", &error),
    }
}

fn compact_new_arguments(arguments: &[String]) -> Option<Vec<String>> {
    let mut filtered = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == "--project" {
            arguments.get(index + 1)?;
            index += 2;
        } else {
            filtered.push(arguments[index].clone());
            index += 1;
        }
    }
    (filtered.first().map(String::as_str) == Some("new"))
        .then(|| filtered.into_iter().skip(1).collect())
}

fn compact_new(arguments: &[String]) -> ExitCode {
    match execute_new(arguments) {
        Ok(bytes) => match write_bytes(&bytes) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(exit_status_for(
                lkjscript::platform::DiagnosticClass::Infrastructure,
            )),
        },
        Err(error) => write_compact_failure("new", &error),
    }
}

fn compact_status_arguments(arguments: &[String]) -> bool {
    compact_exact_project_operation(arguments, &["status"])
}

fn compact_exact_project_operation(arguments: &[String], expected: &[&str]) -> bool {
    let mut filtered = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == "--project" {
            if arguments.get(index + 1).is_none() {
                return filtered == expected;
            }
            index += 2;
        } else {
            filtered.push(arguments[index].as_str());
            index += 1;
        }
    }
    filtered == expected
}

fn compact_status(arguments: Vec<String>) -> ExitCode {
    match execute_status(arguments) {
        Ok(bytes) => match write_bytes(&bytes) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(exit_status_for(
                lkjscript::platform::DiagnosticClass::Infrastructure,
            )),
        },
        Err(error) => write_compact_failure("status", &error),
    }
}

fn compact_inspect_arguments(arguments: &[String]) -> bool {
    let mut filtered = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == "--project" {
            index = index.saturating_add(2);
        } else {
            filtered.push(arguments[index].as_str());
            index += 1;
        }
    }
    filtered.first().copied() == Some("inspect")
}

fn compact_inspect(arguments: Vec<String>) -> ExitCode {
    match execute_inspect(arguments) {
        Ok(bytes) => match write_bytes(&bytes) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(exit_status_for(
                lkjscript::platform::DiagnosticClass::Infrastructure,
            )),
        },
        Err(error) => write_compact_failure("inspect", &error),
    }
}

fn compact_query_arguments(arguments: &[String]) -> bool {
    let mut filtered = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == "--project" {
            index = index.saturating_add(2);
        } else {
            filtered.push(arguments[index].as_str());
            index += 1;
        }
    }
    filtered.first().copied() == Some("query")
}

fn compact_query(arguments: Vec<String>) -> ExitCode {
    match execute_query(arguments) {
        Ok(bytes) => match write_bytes(&bytes) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(exit_status_for(
                lkjscript::platform::DiagnosticClass::Infrastructure,
            )),
        },
        Err(error) => write_compact_failure("query", &error),
    }
}

fn compact_change_arguments(arguments: &[String]) -> bool {
    arguments.first().map(String::as_str) == Some("change")
        || (arguments.first().map(String::as_str) == Some("--project")
            && arguments.get(2).map(String::as_str) == Some("change"))
}

fn compact_change(arguments: Vec<String>) -> ExitCode {
    match execute_change(arguments) {
        Ok(bytes) => match write_bytes(&bytes) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(exit_status_for(
                lkjscript::platform::DiagnosticClass::Infrastructure,
            )),
        },
        Err(diagnostics) => write_compact_failures("change", &diagnostics),
    }
}

fn compact_project_command(arguments: &[String], command: &str) -> bool {
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == "--project" {
            index = index.saturating_add(2);
        } else {
            return arguments[index] == command;
        }
    }
    false
}

fn compact_check(arguments: Vec<String>) -> ExitCode {
    compact_finite("check", execute_check(arguments))
}

fn compact_build(arguments: Vec<String>) -> ExitCode {
    compact_finite("build", execute_build(arguments))
}

fn compact_run(arguments: Vec<String>) -> ExitCode {
    compact_finite("run", execute_run(arguments))
}

fn compact_package_builtin(arguments: Vec<String>) -> ExitCode {
    compact_finite("package", execute_package_builtin(arguments))
}

fn compact_finite(command: &str, result: Result<Vec<u8>, Diagnostic>) -> ExitCode {
    match result {
        Ok(bytes) => match write_bytes(&bytes) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(exit_status_for(
                lkjscript::platform::DiagnosticClass::Infrastructure,
            )),
        },
        Err(error) => write_compact_failure(command, &error),
    }
}

fn unknown_operation(arguments: &[String]) -> ExitCode {
    let mut index = 0_usize;
    let mut operation = None;
    while index < arguments.len() {
        if arguments[index] == "--project" {
            index = index.saturating_add(2);
        } else {
            operation = Some(arguments[index].as_str());
            break;
        }
    }
    let message = operation.map_or_else(
        || "missing operation; use 'capabilities'".to_owned(),
        |value| format!("unknown operation '{value}'; use 'capabilities'"),
    );
    write_compact_failure(
        "cli",
        &Diagnostic::new(
            lkjscript::platform::DiagnosticClass::Source,
            "cli_usage",
            message,
        ),
    )
}

fn write_failure(error: &Diagnostic) -> ExitCode {
    let exit = exit_for(error);
    let failure = json!({
        "ok": false,
        "status": "failure",
        "error": error,
    });
    if write_json(&failure).is_err() {
        ExitCode::from(exit_status_for(
            lkjscript::platform::DiagnosticClass::Infrastructure,
        ))
    } else {
        ExitCode::from(exit)
    }
}

fn write_compact_failure(command: &str, error: &Diagnostic) -> ExitCode {
    write_compact_failures(command, std::slice::from_ref(error))
}

fn write_compact_failures(command: &str, diagnostics: &[Diagnostic]) -> ExitCode {
    let Some(first) = diagnostics.first() else {
        return ExitCode::from(exit_status_for(
            lkjscript::platform::DiagnosticClass::Infrastructure,
        ));
    };
    const MAXIMUM_INLINE_DIAGNOSTICS: usize = 64;
    let exit = exit_for(first);
    let result = (|| {
        let mut output = CompactResponseWriter::new(CompactResponseLimits {
            maximum_bytes: MAXIMUM_CLI_RESPONSE_BYTES,
            maximum_records: MAXIMUM_CLI_RESPONSE_RECORDS,
        })?;
        output.append_record("result", &[("status", "failure"), ("command", command)])?;
        for error in diagnostics.iter().take(MAXIMUM_INLINE_DIAGNOSTICS) {
            let class = diagnostic_class_name(error.class);
            let mut fields = vec![
                ("class", class.to_owned()),
                ("code", error.code.clone()),
                ("message", error.message.clone()),
            ];
            if let Some(location) = &error.location {
                fields.push(("path", location.path.clone()));
                fields.push(("line", location.line.to_string()));
                fields.push(("column", location.column.to_string()));
            }
            let borrowed = fields
                .iter()
                .map(|(name, value)| (*name, value.as_str()))
                .collect::<Vec<_>>();
            output.append_record("diagnostic", &borrowed)?;
        }
        if diagnostics.len() > MAXIMUM_INLINE_DIAGNOSTICS {
            let omitted = diagnostics.len() - MAXIMUM_INLINE_DIAGNOSTICS;
            output.append_record(
                "summary",
                &[
                    ("diagnostics", &diagnostics.len().to_string()),
                    ("omitted", &omitted.to_string()),
                ],
            )?;
        }
        write_bytes(&output.finish())
    })();
    if result.is_err() {
        ExitCode::from(exit_status_for(
            lkjscript::platform::DiagnosticClass::Infrastructure,
        ))
    } else {
        ExitCode::from(exit)
    }
}

const fn diagnostic_class_name(class: lkjscript::platform::DiagnosticClass) -> &'static str {
    match class {
        lkjscript::platform::DiagnosticClass::Source => "source",
        lkjscript::platform::DiagnosticClass::Semantic => "semantic",
        lkjscript::platform::DiagnosticClass::Capability => "capability",
        lkjscript::platform::DiagnosticClass::Resource => "resource",
        lkjscript::platform::DiagnosticClass::Cancelled => "cancelled",
        lkjscript::platform::DiagnosticClass::Corrupt => "corrupt",
        lkjscript::platform::DiagnosticClass::Infrastructure => "infrastructure",
    }
}

async fn worker(arguments: &[String]) -> Result<(), Diagnostic> {
    if arguments.len() != 2 || arguments[0] != "--deployment" {
        return Err(cli_error(
            "worker requires exactly --deployment <descriptor.json>",
        ));
    }
    let prepared =
        PreparedDeployment::load(Path::new(&arguments[1]), tokio::runtime::Handle::current())?;
    let application = prepared.worker_application()?;
    if let Err(mut error) = write_json(&json!({
        "ok": true,
        "event": "ready",
        "deployment": prepared.observe_redacted(),
    })) {
        append_shutdown_evidence(&mut error, &application.shutdown().await);
        return Err(error);
    }
    let receipt = application
        .run(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    write_json(&json!({
        "ok": true,
        "event": "stopped",
        "receipt": receipt,
    }))
}

async fn serve(arguments: &[String]) -> Result<(), Diagnostic> {
    if arguments.len() != 2 || arguments[0] != "--deployment" {
        return Err(cli_error(
            "serve requires exactly --deployment <descriptor.json>",
        ));
    }
    let prepared =
        PreparedDeployment::load(Path::new(&arguments[1]), tokio::runtime::Handle::current())?;
    let address = prepared
        .listen()
        .ok_or_else(|| cli_error("service deployment requires a concrete listen address"))?
        .to_owned();
    match prepared.observe_redacted().runner.as_str() {
        "http" => serve_http(prepared, &address).await,
        "interactive" => serve_interactive(prepared, &address).await,
        _ => Err(cli_error(
            "serve requires an http or interactive resident target",
        )),
    }
}

async fn serve_http(prepared: PreparedDeployment, address: &str) -> Result<(), Diagnostic> {
    let application = prepared.http_application()?;
    let listener = match TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(source) => {
            let mut error = Diagnostic::new(
                lkjscript::platform::DiagnosticClass::Infrastructure,
                "serve_bind",
                format!("listener could not bind: {source}"),
            );
            append_shutdown_evidence(&mut error, &application.shutdown().await);
            return Err(error);
        }
    };
    let local_address = match listener.local_addr() {
        Ok(address) => address,
        Err(source) => {
            let mut error = Diagnostic::new(
                lkjscript::platform::DiagnosticClass::Infrastructure,
                "serve_address",
                format!("listener address is unavailable: {source}"),
            );
            append_shutdown_evidence(&mut error, &application.shutdown().await);
            return Err(error);
        }
    };
    if let Err(mut error) = write_json(&json!({
        "ok": true,
        "event": "ready",
        "local_address": local_address.to_string(),
        "deployment": prepared.observe_redacted(),
    })) {
        append_shutdown_evidence(&mut error, &application.shutdown().await);
        return Err(error);
    }
    let receipt = application
        .serve(listener, async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    write_json(&json!({
        "ok": true,
        "event": "stopped",
        "receipt": receipt,
    }))
}

async fn serve_interactive(prepared: PreparedDeployment, address: &str) -> Result<(), Diagnostic> {
    let application = prepared.session_application()?;
    let listener = match TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(source) => {
            let mut error = Diagnostic::new(
                lkjscript::platform::DiagnosticClass::Infrastructure,
                "serve_bind",
                format!("listener could not bind: {source}"),
            );
            append_shutdown_evidence(&mut error, &application.shutdown().await);
            return Err(error);
        }
    };
    let local_address = match listener.local_addr() {
        Ok(address) => address,
        Err(source) => {
            let mut error = Diagnostic::new(
                lkjscript::platform::DiagnosticClass::Infrastructure,
                "serve_address",
                format!("listener address is unavailable: {source}"),
            );
            append_shutdown_evidence(&mut error, &application.shutdown().await);
            return Err(error);
        }
    };
    if let Err(mut error) = write_json(&json!({
        "ok": true,
        "event": "ready",
        "local_address": local_address.to_string(),
        "deployment": prepared.observe_redacted(),
    })) {
        append_shutdown_evidence(&mut error, &application.shutdown().await);
        return Err(error);
    }
    let receipt = application
        .serve(listener, async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    write_json(&json!({
        "ok": true,
        "event": "stopped",
        "receipt": receipt,
    }))
}

fn append_shutdown_evidence(error: &mut Diagnostic, shutdown: &ShutdownReceipt) {
    if shutdown.remaining_tasks != 0 {
        error.notes.push(format!(
            "{} resident tasks remained after failed command cleanup",
            shutdown.remaining_tasks
        ));
    }
    error.notes.extend(
        shutdown
            .cleanup_failures
            .iter()
            .map(|failure| format!("adapter cleanup failed with safe code '{}'", failure.code)),
    );
}

fn write_json(value: &impl Serialize) -> Result<(), Diagnostic> {
    let mut bytes = serde_json::to_vec(value).map_err(|error| {
        Diagnostic::new(
            lkjscript::platform::DiagnosticClass::Infrastructure,
            "cli_json_encode",
            format!("machine response could not be encoded: {error}"),
        )
    })?;
    bytes.push(b'\n');
    write_bytes(&bytes)
}

fn write_bytes(bytes: &[u8]) -> Result<(), Diagnostic> {
    std::io::stdout().lock().write_all(bytes).map_err(|error| {
        Diagnostic::new(
            lkjscript::platform::DiagnosticClass::Infrastructure,
            "cli_output",
            format!("machine response could not be written: {error}"),
        )
    })
}

fn cli_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        lkjscript::platform::DiagnosticClass::Source,
        "cli_usage",
        message,
    )
}

fn exit_for(error: &Diagnostic) -> u8 {
    match error.code.as_str() {
        "change_stale_base" | "change_authored_stale_base" => 7,
        _ => exit_status_for(error.class),
    }
}
