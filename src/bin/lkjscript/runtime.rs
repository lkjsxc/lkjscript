#![allow(clippy::result_large_err)]

use super::{
    BoundaryErrorKind, CliOutcome, EXIT_APPLICATION_INPUT, EXIT_OUTPUT, EXIT_RESOURCE,
    EXIT_TRANSPORT, EXIT_USAGE_OR_JSON, SessionLine, failure, read_session_line, success, usage,
    write_outcome,
};
use lkjscript::error::{ErrorCode, LkError};
use lkjscript::instance::strict_json;
use lkjscript::machine::{MAX_JSON_INPUT_BYTES, MAX_JSON_OUTPUT_BYTES};
use lkjscript::runtime::{RUNTIME_CONTRACT_VERSION, RuntimeKernel, RuntimePolicy};
use lkjscript::runtime_protocol::{
    RuntimeErrorEnvelope, RuntimeRequestEnvelope, success as response,
};
use serde::Serialize;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

const HELP: &str = "usage: lkjscript runtime COMMAND [OPTIONS]

Commands:
  orientation [--pretty]
  inspect --store DIRECTORY [--pretty]
  session --store DIRECTORY

Runtime command and foreground-session contract version 2 is required. A session retains one
topology-neutral kernel and one exact store lock for its caller-owned lifetime. Every request names
its exact application, instance, command, and grant; no current application or instance exists.
The stream is canonical one-line JSON, remains synchronized after a malformed bounded line, and
stops on an acknowledged shutdown request or EOF.";

pub(super) enum RuntimeCommand {
    Invalid(String),
    Help,
    Orientation { pretty: bool },
    Inspect { store: PathBuf, pretty: bool },
    Session { store: PathBuf },
}

pub(super) fn parse(arguments: impl Iterator<Item = String>) -> Result<RuntimeCommand, String> {
    let arguments = arguments.collect::<Vec<_>>();
    let Some(command) = arguments.first().map(String::as_str) else {
        return Ok(RuntimeCommand::Help);
    };
    match command {
        "help" | "--help" if arguments.len() == 1 => Ok(RuntimeCommand::Help),
        "orientation" => parse_orientation(&arguments[1..]),
        "inspect" => parse_store(&arguments[1..], false),
        "session" => parse_store(&arguments[1..], true),
        _ => Err(runtime_usage(
            "unknown runtime command or unexpected argument",
        )),
    }
}

pub(super) fn run(command: RuntimeCommand) -> ExitCode {
    match command {
        RuntimeCommand::Session { store } => run_session(store),
        command => write_outcome(run_json(command)),
    }
}

fn run_json(command: RuntimeCommand) -> CliOutcome {
    match command {
        RuntimeCommand::Invalid(message) => runtime_error(
            LkError::new(ErrorCode::ProtocolMalformed, message),
            false,
            EXIT_USAGE_OR_JSON,
        ),
        RuntimeCommand::Help => success(HELP.as_bytes().to_vec()),
        RuntimeCommand::Orientation { pretty } => {
            match RuntimeKernel::new(RuntimePolicy::default()) {
                Ok(kernel) => encode_json(&kernel.orientation(), pretty),
                Err(error) => runtime_error(error, pretty, EXIT_RESOURCE),
            }
        }
        RuntimeCommand::Inspect { store, pretty } => {
            match RuntimeKernel::open_instance_store(&store, RuntimePolicy::default()) {
                Ok(kernel) => encode_json(&kernel.inspection(), pretty),
                Err(error) => runtime_error_auto(error, pretty),
            }
        }
        RuntimeCommand::Session { .. } => failure(
            EXIT_USAGE_OR_JSON,
            BoundaryErrorKind::Usage,
            runtime_usage("session must own process input and output"),
            None,
        ),
    }
}

fn run_session(store: PathBuf) -> ExitCode {
    let mut kernel = match RuntimeKernel::open_instance_store(&store, RuntimePolicy::default()) {
        Ok(kernel) => kernel,
        Err(error) => return write_outcome(runtime_error_auto(error, false)),
    };
    let mut reader = BufReader::new(std::io::stdin().lock());
    let mut stdout = std::io::stdout().lock();
    loop {
        let line = match read_session_line(&mut reader, MAX_JSON_INPUT_BYTES) {
            Ok(SessionLine::End) => return ExitCode::SUCCESS,
            Ok(SessionLine::Data(line)) => line,
            Ok(SessionLine::TooLarge) => {
                let error = LkError::new(
                    ErrorCode::PolicyExceeded,
                    "runtime session JSON line exceeds input byte policy",
                );
                if write_session_error(&mut stdout, &mut kernel, None, &error).is_err() {
                    return ExitCode::from(EXIT_OUTPUT);
                }
                continue;
            }
            Err(error) => {
                eprintln!("cannot read runtime session JSON line: {error}");
                return ExitCode::from(EXIT_USAGE_OR_JSON);
            }
        };
        let decode_started = Instant::now();
        let envelope = match strict_json::<RuntimeRequestEnvelope>(&line, "runtime request") {
            Ok(envelope) => envelope,
            Err(error) => {
                kernel.observe_request_decode(decode_started.elapsed(), line.len());
                if write_session_error(&mut stdout, &mut kernel, None, &error).is_err() {
                    return ExitCode::from(EXIT_OUTPUT);
                }
                continue;
            }
        };
        kernel.observe_request_decode(decode_started.elapsed(), line.len());
        let request_id = envelope.request_id;
        if let Err(error) = envelope.validate() {
            if write_session_error(&mut stdout, &mut kernel, Some(request_id), &error).is_err() {
                return ExitCode::from(EXIT_OUTPUT);
            }
            continue;
        }
        let shutdown = envelope.request.requests_shutdown();
        match envelope.request.execute(&mut kernel) {
            Ok(value) => {
                let response = response(request_id, value);
                if write_session_value(&mut stdout, &mut kernel, &response).is_err() {
                    return ExitCode::from(EXIT_OUTPUT);
                }
            }
            Err(error) => {
                if write_session_error(&mut stdout, &mut kernel, Some(request_id), &error).is_err()
                {
                    return ExitCode::from(EXIT_OUTPUT);
                }
            }
        }
        if shutdown {
            return ExitCode::SUCCESS;
        }
    }
}

fn write_session_error(
    stdout: &mut impl Write,
    kernel: &mut RuntimeKernel,
    request_id: Option<u64>,
    error: &LkError,
) -> std::io::Result<()> {
    write_session_value(
        stdout,
        kernel,
        &RuntimeErrorEnvelope {
            version: RUNTIME_CONTRACT_VERSION,
            request_id,
            error,
        },
    )
}

fn write_session_value(
    stdout: &mut impl Write,
    kernel: &mut RuntimeKernel,
    value: &impl Serialize,
) -> std::io::Result<()> {
    let started = Instant::now();
    let mut bytes = serde_json::to_vec(value).map_err(std::io::Error::other)?;
    if bytes.len() > MAX_JSON_OUTPUT_BYTES {
        return Err(std::io::Error::other(
            "runtime session response exceeds output policy",
        ));
    }
    kernel
        .observe_response_encoding(started.elapsed(), bytes.len())
        .map_err(std::io::Error::other)?;
    bytes.push(b'\n');
    stdout.write_all(&bytes)?;
    stdout.flush()
}

fn encode_json<T: Serialize>(value: &T, pretty: bool) -> CliOutcome {
    let encoded = if pretty {
        serde_json::to_vec_pretty(value)
    } else {
        serde_json::to_vec(value)
    };
    match encoded {
        Ok(bytes) if bytes.len() <= MAX_JSON_OUTPUT_BYTES => success(bytes),
        Ok(_) => failure(
            EXIT_OUTPUT,
            BoundaryErrorKind::Output,
            "runtime response exceeds JSON output policy".into(),
            None,
        ),
        Err(error) => failure(
            EXIT_OUTPUT,
            BoundaryErrorKind::Output,
            format!("cannot encode runtime response: {error}"),
            None,
        ),
    }
}

fn runtime_error_auto(error: LkError, pretty: bool) -> CliOutcome {
    let exit = match error.code {
        ErrorCode::PolicyExceeded
        | ErrorCode::ExecutionFuelExhausted
        | ErrorCode::ExecutionFrameExhausted
        | ErrorCode::ExecutionMemoryExhausted => EXIT_RESOURCE,
        ErrorCode::Io | ErrorCode::AuthorityBusy | ErrorCode::CommitOutcomeUnknown => {
            EXIT_TRANSPORT
        }
        _ => EXIT_APPLICATION_INPUT,
    };
    runtime_error(error, pretty, exit)
}

fn runtime_error(error: LkError, pretty: bool, exit: u8) -> CliOutcome {
    let diagnostic = error.to_string();
    let mut outcome = encode_json(
        &RuntimeErrorEnvelope {
            version: RUNTIME_CONTRACT_VERSION,
            request_id: None,
            error: &error,
        },
        pretty,
    );
    outcome.exit = exit;
    outcome.diagnostic = Some(diagnostic);
    outcome
}

fn parse_orientation(arguments: &[String]) -> Result<RuntimeCommand, String> {
    match arguments {
        [] => Ok(RuntimeCommand::Orientation { pretty: false }),
        [flag] if flag == "--pretty" => Ok(RuntimeCommand::Orientation { pretty: true }),
        _ => Err(runtime_usage("orientation accepts only --pretty")),
    }
}

fn parse_store(arguments: &[String], session: bool) -> Result<RuntimeCommand, String> {
    let mut store = None;
    let mut pretty = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--store" if store.is_none() => {
                index += 1;
                store = arguments.get(index).map(PathBuf::from);
                if store.is_none() {
                    return Err(runtime_usage("--store requires a directory"));
                }
            }
            "--pretty" if !session && !pretty => pretty = true,
            _ => return Err(runtime_usage("invalid or duplicate runtime option")),
        }
        index += 1;
    }
    let store = store.ok_or_else(|| runtime_usage("runtime command requires --store"))?;
    if session {
        Ok(RuntimeCommand::Session { store })
    } else {
        Ok(RuntimeCommand::Inspect { store, pretty })
    }
}

fn runtime_usage(reason: &str) -> String {
    usage(&format!(
        "{reason}; use `lkjscript runtime help` for kernel and session details"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_options_are_closed() {
        assert!(matches!(
            parse(Vec::<String>::new().into_iter()),
            Ok(RuntimeCommand::Help)
        ));
        assert!(parse(["session".into()].into_iter()).is_err());
        assert!(
            parse(
                [
                    "session".into(),
                    "--store".into(),
                    "/tmp/store".into(),
                    "--pretty".into()
                ]
                .into_iter()
            )
            .is_err()
        );
    }
}
