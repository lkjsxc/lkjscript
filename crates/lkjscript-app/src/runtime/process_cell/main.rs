use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;

use lkjscript_core::{CapabilityKind, Limits, ResourceProfileName};
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
    write_response(
        &mut output,
        &ProcessResponse::Ready {
            process: std::process::id(),
        },
    )
    .map_err(|error| error.to_string())?;
    loop {
        match read_request(&mut input).map_err(|error| error.to_string())? {
            ProcessRequest::Invoke { cell, arguments } => {
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
                write_response(
                    &mut output,
                    &ProcessResponse::Outcome {
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
    if bootstrap.execution.max_output_bytes > MAX_APPLICATION_OUTPUT_BYTES {
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
    let (_, manifest) = lkjscript_compiler::package::verify(entry).map_err(|e| e.to_string())?;
    let profile = manifest
        .resource_profile
        .as_deref()
        .map_or(
            Ok(ResourceProfileName::Default),
            ResourceProfileName::from_str,
        )
        .map_err(|error| error.to_string())?;
    let profile = lkjscript_compiler::ResourceProfile::new(profile);
    let program = lkjscript_compiler::compile_path_with_profile(entry, &Limits::default(), profile)
        .map_err(|error| error.to_string())?;
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
        if package
            .binary_search_by_key(&capability.as_str(), String::as_str)
            .is_err()
        {
            return Err(format!("package lacks {} grant", capability.as_str()));
        }
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

fn bounded(message: &str) -> String {
    let mut length = message.len().min(MAX_DIAGNOSTIC_BYTES);
    while !message.is_char_boundary(length) {
        length -= 1;
    }
    message[..length].to_owned()
}
