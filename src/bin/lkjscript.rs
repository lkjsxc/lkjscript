//! Public graph-native development command and generic resident runner.

#![allow(
    clippy::result_large_err,
    reason = "the process boundary preserves the complete structured diagnostic"
)]

use lkjscript::platform::{CLI_CONTRACT_VERSION, Diagnostic, PreparedDeployment, execute_cli};
use serde::Serialize;
use serde_json::json;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
use tokio::net::TcpListener;

const EXIT_USAGE_OR_SOURCE: u8 = 2;
const EXIT_CAPABILITY: u8 = 3;
const EXIT_RESOURCE: u8 = 4;
const EXIT_CORRUPT: u8 = 5;
const EXIT_INFRASTRUCTURE: u8 = 6;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if !matches!(
        arguments.first().map(String::as_str),
        Some("serve" | "worker")
    ) {
        return cli(arguments);
    }
    let outcome = match arguments.first().map(String::as_str) {
        Some("serve") => serve(&arguments[1..]).await,
        Some("worker") => worker(&arguments[1..]).await,
        _ => unreachable!("standalone commands returned above"),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => write_failure(&error, 1),
    }
}

fn cli(arguments: Vec<String>) -> ExitCode {
    match execute_cli(arguments) {
        Ok(receipt) => {
            let exit = receipt.process_exit_code();
            if write_json(&receipt).is_err() {
                return ExitCode::from(EXIT_INFRASTRUCTURE);
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
        ExitCode::from(EXIT_INFRASTRUCTURE)
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
    std::io::stdout().lock().write_all(&bytes).map_err(|error| {
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
    use lkjscript::platform::DiagnosticClass;
    match error.class {
        DiagnosticClass::Source | DiagnosticClass::Semantic => EXIT_USAGE_OR_SOURCE,
        DiagnosticClass::Capability | DiagnosticClass::Cancelled => EXIT_CAPABILITY,
        DiagnosticClass::Resource => EXIT_RESOURCE,
        DiagnosticClass::Corrupt => EXIT_CORRUPT,
        DiagnosticClass::Infrastructure => EXIT_INFRASTRUCTURE,
    }
}
