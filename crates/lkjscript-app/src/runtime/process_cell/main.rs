use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use lkjscript_core::CapabilityKind;

mod provenance;
use lkjscript_runtime::process_cell_protocol::{
    read_bootstrap, read_request, runtime_control_digest, write_response, ProcessBootstrap,
    ProcessRequest, ProcessResponse, MAX_APPLICATION_OUTPUT_BYTES, MAX_DIAGNOSTIC_BYTES,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr(), "isolated process cell: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let bootstrap = read_bootstrap(&mut input).map_err(|error| error.to_string())?;
    let program = match prepare(&bootstrap) {
        Ok(program) => program,
        Err(error) => {
            write_response(
                &mut output,
                &ProcessResponse::ReadyFailure {
                    diagnostic: bounded(&error),
                },
            )
            .map_err(|write| write.to_string())?;
            return Err(error);
        }
    };
    let provenance = match provenance::authenticated(&bootstrap, &program) {
        Ok(provenance) => provenance,
        Err(error) => {
            write_response(
                &mut output,
                &ProcessResponse::ReadyFailure {
                    diagnostic: bounded(&error),
                },
            )
            .map_err(|write| write.to_string())?;
            return Err(error);
        }
    };
    write_response(
        &mut output,
        &ProcessResponse::Ready {
            process: u64::from(std::process::id()),
            provenance: provenance.clone(),
        },
    )
    .map_err(|error| error.to_string())?;
    loop {
        match read_request(&mut input).map_err(|error| error.to_string())? {
            ProcessRequest::Invoke { cell, arguments } => {
                let malformed_provenance = test_worker()
                    && arguments.len() == 1
                    && arguments.first().map(String::as_str) == Some("malformed-provenance");
                let stdio = lkjscript_host::BufferedStdio::default();
                let host = lkjscript_host::HostEnvironment {
                    stdio: Some(Arc::new(stdio.clone())),
                    clock: Some(Arc::new(lkjscript_host::PortableClock::new())),
                    logger: Some(Arc::new(lkjscript_host::PortableLogger)),
                    cancellation: Some(Arc::new(lkjscript_host::CancellationToken::new())),
                    directory: None,
                    database: None,
                };
                let inputs = lkjscript_vm::ExecutionInputs {
                    arguments,
                    capabilities: bootstrap.capabilities.clone(),
                    host,
                };
                let outcome =
                    lkjscript_vm::run_chunk(program.bytecode(), &inputs, &bootstrap.execution);
                let application_output = stdio.output().map_err(|error| error.to_string())?;
                let flushes = stdio.flushes().map_err(|error| error.to_string())?;
                let mut outcome_provenance = provenance.clone();
                if malformed_provenance {
                    outcome_provenance.root_witness_member = [0x5a; 32];
                }
                write_response(
                    &mut output,
                    &ProcessResponse::Outcome {
                        provenance: outcome_provenance,
                        cell,
                        outcome,
                        output: application_output,
                        flushes,
                    },
                )
                .map_err(|error| error.to_string())?;
            }
            ProcessRequest::Stop => {
                write_response(&mut output, &ProcessResponse::Stopped)
                    .map_err(|error| error.to_string())?;
                return Ok(());
            }
        }
    }
}

fn prepare(bootstrap: &ProcessBootstrap) -> Result<lkjscript_compiler::ExecutableProgram, String> {
    if bootstrap.platform_revision != lkjscript_contracts::PLATFORM_REVISION {
        return Err("worker platform revision mismatch".into());
    }
    if bootstrap.contract != runtime_control_digest().map_err(|error| error.to_string())? {
        return Err("worker runtime-control contract mismatch".into());
    }
    let execution = bootstrap
        .execution
        .limited_policy()
        .ok_or_else(|| "worker requires an explicit limited execution policy".to_string())?;
    if execution.max_output_bytes > MAX_APPLICATION_OUTPUT_BYTES {
        return Err("worker output limit exceeds process-cell bound".into());
    }
    let entry = Path::new(&bootstrap.entry);
    if !entry.is_absolute() {
        return Err("worker package entry must be absolute".into());
    }
    let canonical = entry
        .canonicalize()
        .map_err(|error| format!("canonicalize worker entry: {error}"))?;
    if canonical != entry {
        return Err("worker package entry must be canonical".into());
    }
    let (_, manifest, package) =
        lkjscript_compiler::package::verify_content(entry).map_err(|e| e.to_string())?;
    if package.as_bytes() != bootstrap.package {
        return Err("worker package content identity mismatch".into());
    }
    let program = lkjscript_compiler::compile_path(entry).map_err(|error| error.to_string())?;
    validate_grants(
        program.bytecode().required_capabilities(),
        &bootstrap.capabilities,
        &manifest.capabilities,
    )?;
    Ok(program)
}

fn validate_grants(
    required: &[CapabilityKind],
    granted: &[CapabilityKind],
    package: &[String],
) -> Result<(), String> {
    for capability in required {
        if granted.binary_search(capability).is_err() {
            return Err(format!("worker lacks {} grant", capability.as_str()));
        }
    }
    if granted.iter().any(|capability| {
        package
            .binary_search_by_key(&capability.as_str(), String::as_str)
            .is_err()
    }) {
        return Err("package lacks an isolated worker grant".into());
    }
    if granted.iter().any(|capability| {
        !matches!(
            capability,
            CapabilityKind::Arguments | CapabilityKind::Stdio | CapabilityKind::Clock
        )
    }) {
        return Err("worker received an unsupported capability grant".into());
    }
    Ok(())
}

fn test_worker() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.file_stem().map(|name| name.to_owned()))
        .and_then(|name| name.to_str().map(str::to_owned))
        .is_some_and(|name| name.starts_with("lkjscript-cell-test-worker"))
}

fn bounded(message: &str) -> String {
    let mut length = message.len().min(MAX_DIAGNOSTIC_BYTES);
    while !message.is_char_boundary(length) {
        length -= 1;
    }
    message[..length].to_owned()
}
