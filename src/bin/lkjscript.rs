//! Public graph-native development command and generic resident runner.

#![allow(
    clippy::result_large_err,
    reason = "the process boundary preserves the complete structured diagnostic"
)]

use lkjscript::platform::contract::exit_status_for;
use lkjscript::platform::control::render_record;
use lkjscript::platform::{
    CLI_CONTRACT_VERSION, Diagnostic, PreparedDeployment, PublicOperation, execute_capabilities,
    execute_cli, execute_new,
};
use serde::Serialize;
use serde_json::json;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
use tokio::net::TcpListener;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if let Some(capability_arguments) = compact_capability_arguments(&arguments) {
        return compact_capabilities(&capability_arguments);
    }
    if let Some(new_arguments) = compact_new_arguments(&arguments) {
        return compact_new(&new_arguments);
    }
    let operation = arguments
        .first()
        .and_then(|value| PublicOperation::parse(value));
    if !matches!(
        operation,
        Some(PublicOperation::Serve | PublicOperation::Worker)
    ) {
        return cli(arguments);
    }
    let outcome = match operation {
        Some(PublicOperation::Serve) => serve(&arguments[1..]).await,
        Some(PublicOperation::Worker) => worker(&arguments[1..]).await,
        _ => return cli(arguments),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => write_failure(&error, 1),
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

fn cli(arguments: Vec<String>) -> ExitCode {
    match execute_cli(arguments) {
        Ok(receipt) => {
            let exit = receipt.process_exit_code();
            if write_json(&receipt).is_err() {
                return ExitCode::from(exit_status_for(
                    lkjscript::platform::DiagnosticClass::Infrastructure,
                ));
            }
            ExitCode::from(exit)
        }
        Err(error) => write_failure(&error, CLI_CONTRACT_VERSION),
    }
}

fn write_failure(error: &Diagnostic, contract_version: u16) -> ExitCode {
    let exit = exit_for(error);
    let failure = json!({
        "contract_version": contract_version,
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
    let exit = exit_for(error);
    let class = match error.class {
        lkjscript::platform::DiagnosticClass::Source => "source",
        lkjscript::platform::DiagnosticClass::Semantic => "semantic",
        lkjscript::platform::DiagnosticClass::Capability => "capability",
        lkjscript::platform::DiagnosticClass::Resource => "resource",
        lkjscript::platform::DiagnosticClass::Cancelled => "cancelled",
        lkjscript::platform::DiagnosticClass::Corrupt => "corrupt",
        lkjscript::platform::DiagnosticClass::Infrastructure => "infrastructure",
    };
    let result = render_record("result", &[("status", "failure"), ("command", command)])
        .and_then(|mut output| {
            output.push_str(&render_record(
                "diagnostic",
                &[
                    ("class", class),
                    ("code", &error.code),
                    ("message", &error.message),
                ],
            )?);
            Ok(output)
        })
        .and_then(|output| write_bytes(output.as_bytes()));
    if result.is_err() {
        ExitCode::from(exit_status_for(
            lkjscript::platform::DiagnosticClass::Infrastructure,
        ))
    } else {
        ExitCode::from(exit)
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
    write_json(&json!({
        "contract_version": 1,
        "ok": true,
        "event": "ready",
        "deployment": prepared.observe_redacted(),
    }))?;
    let application = prepared.worker_application()?;
    let receipt = application
        .run(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    write_json(&json!({
        "contract_version": 1,
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
        .ok_or_else(|| cli_error("service deployment requires a concrete listen address"))?;
    let listener = TcpListener::bind(address).await.map_err(|error| {
        Diagnostic::new(
            lkjscript::platform::DiagnosticClass::Infrastructure,
            "serve_bind",
            format!("listener could not bind: {error}"),
        )
    })?;
    let local_address = listener.local_addr().map_err(|error| {
        Diagnostic::new(
            lkjscript::platform::DiagnosticClass::Infrastructure,
            "serve_address",
            format!("listener address is unavailable: {error}"),
        )
    })?;
    write_json(&json!({
        "contract_version": 1,
        "ok": true,
        "event": "ready",
        "local_address": local_address.to_string(),
        "deployment": prepared.observe_redacted(),
    }))?;
    let application = prepared.http_application()?;
    let receipt = application
        .serve(listener, async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    write_json(&json!({
        "contract_version": 1,
        "ok": true,
        "event": "stopped",
        "receipt": receipt,
    }))
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
    exit_status_for(error.class)
}
