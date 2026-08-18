use lkjscript::engine::Engine;
use lkjscript::machine::{
    BoundaryErrorKind, DescribeSchemaRequest, MAX_JSON_INPUT_BYTES, MachineSchemaDigest,
    SchemaProjection, SchemaRoot, decode_request, encode_boundary_error, encode_response,
    encode_schema, request_id_hint,
};
use lkjscript::protocol::Request;
use std::io::{BufRead, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

#[path = "lkjscript/agent.rs"]
mod agent;
#[path = "lkjscript/application.rs"]
mod application;
#[path = "lkjscript/instance.rs"]
mod instance;
#[path = "lkjscript/release.rs"]
mod reusable_release;
#[path = "lkjscript/runtime.rs"]
mod runtime_cli;

const EXIT_USAGE_OR_JSON: u8 = 2;
const EXIT_TRANSPORT: u8 = 3;
const EXIT_OUTPUT: u8 = 4;
const EXIT_ARTIFACT: u8 = 5;
const EXIT_APPLICATION_INPUT: u8 = 6;
const EXIT_PROGRAM: u8 = 7;
const EXIT_RESOURCE: u8 = 8;

fn main() -> ExitCode {
    let outcome = match parse_command(std::env::args().skip(1)) {
        Ok(Command::Session { state }) => return run_session(state),
        Ok(Command::Application(command)) => return application::run(command),
        Ok(Command::Instance(command)) => return instance::run(command),
        Ok(Command::Runtime(command)) => return runtime_cli::run(command),
        Ok(Command::Release(command)) => return reusable_release::run(command),
        Ok(command) => run_command(command),
        Err(message) => failure(EXIT_USAGE_OR_JSON, BoundaryErrorKind::Usage, message, None),
    };
    write_outcome(outcome)
}

fn write_outcome(outcome: CliOutcome) -> ExitCode {
    let mut bytes = outcome.stdout;
    bytes.push(b'\n');
    if let Err(error) = std::io::stdout().lock().write_all(&bytes) {
        eprintln!("cannot write machine response: {error}");
        return ExitCode::from(EXIT_OUTPUT);
    }
    if let Some(diagnostic) = outcome.diagnostic {
        eprintln!("{diagnostic}");
    }
    ExitCode::from(outcome.exit)
}

struct CliOutcome {
    stdout: Vec<u8>,
    diagnostic: Option<String>,
    exit: u8,
}

fn run_command(command: Command) -> CliOutcome {
    match command {
        Command::Schema { request, pretty } => match encode_schema(&request, pretty) {
            Ok(stdout) => success(stdout),
            Err(error) => failure(
                if error.kind == BoundaryErrorKind::Usage {
                    EXIT_USAGE_OR_JSON
                } else {
                    EXIT_OUTPUT
                },
                error.kind,
                error.to_string(),
                None,
            ),
        },
        Command::Rpc { state, pretty } => run_rpc(state, pretty),
        Command::Agent(command) => agent::run(command),
        Command::Application(_) => failure(
            EXIT_USAGE_OR_JSON,
            BoundaryErrorKind::Usage,
            usage("application command must own its input and output boundary"),
            None,
        ),
        Command::Instance(_) => failure(
            EXIT_USAGE_OR_JSON,
            BoundaryErrorKind::Usage,
            usage("instance command must own its input and output boundary"),
            None,
        ),
        Command::Release(_) => failure(
            EXIT_USAGE_OR_JSON,
            BoundaryErrorKind::Usage,
            usage("release command must own its input and output boundary"),
            None,
        ),
        Command::Runtime(_) => failure(
            EXIT_USAGE_OR_JSON,
            BoundaryErrorKind::Usage,
            usage("runtime command must own its input and output boundary"),
            None,
        ),
        Command::Help => success(usage_text().as_bytes().to_vec()),
        Command::Session { .. } => failure(
            EXIT_USAGE_OR_JSON,
            BoundaryErrorKind::Usage,
            usage("session must own the process input and output streams"),
            None,
        ),
    }
}

fn run_rpc(state: PathBuf, pretty: bool) -> CliOutcome {
    let input = match read_stdin_bounded() {
        Ok(input) => input,
        Err(message) => {
            return failure(
                EXIT_USAGE_OR_JSON,
                BoundaryErrorKind::InputTooLarge,
                message,
                None,
            );
        }
    };
    process_rpc(&state, &input, pretty)
}

fn process_rpc(state: &std::path::Path, input: &[u8], pretty: bool) -> CliOutcome {
    let envelope = match decode_request(input) {
        Ok(envelope) => envelope,
        Err(error) => {
            let request_id = request_id_hint(input);
            return failure(
                EXIT_USAGE_OR_JSON,
                error.kind,
                error.to_string(),
                request_id,
            );
        }
    };
    let mut engine = match Engine::open(state) {
        Ok(engine) => engine,
        Err(error) => {
            return failure(
                EXIT_TRANSPORT,
                BoundaryErrorKind::Transport,
                error.to_string(),
                Some(envelope.request_id),
            );
        }
    };
    process_decoded_rpc(&mut engine, envelope, pretty)
}

fn process_decoded_rpc(
    engine: &mut Engine,
    envelope: lkjscript::machine::RequestEnvelope,
    pretty: bool,
) -> CliOutcome {
    let response = match engine.request(envelope.request_id, envelope.request) {
        Ok(response) => response,
        Err(error) => {
            return failure(
                EXIT_TRANSPORT,
                BoundaryErrorKind::Transport,
                error.to_string(),
                Some(envelope.request_id),
            );
        }
    };
    match encode_response(envelope.request_id, &response, pretty) {
        Ok(stdout) => success(stdout),
        Err(error) => failure(
            EXIT_OUTPUT,
            BoundaryErrorKind::Output,
            error.to_string(),
            Some(envelope.request_id),
        ),
    }
}

enum SessionLine {
    End,
    Data(Vec<u8>),
    TooLarge,
}

fn read_session_line(reader: &mut impl BufRead, maximum: usize) -> std::io::Result<SessionLine> {
    let mut bytes = Vec::new();
    let mut too_large = false;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if bytes.is_empty() && !too_large {
                Ok(SessionLine::End)
            } else if too_large {
                Ok(SessionLine::TooLarge)
            } else {
                Ok(SessionLine::Data(bytes))
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let count = newline.unwrap_or(available.len());
        if !too_large {
            if bytes.len().saturating_add(count) > maximum {
                too_large = true;
                bytes.clear();
            } else {
                bytes.extend_from_slice(&available[..count]);
            }
        }
        let consumed = count + usize::from(newline.is_some());
        reader.consume(consumed);
        if newline.is_some() {
            return if too_large {
                Ok(SessionLine::TooLarge)
            } else {
                Ok(SessionLine::Data(bytes))
            };
        }
    }
}

fn run_session(state: PathBuf) -> ExitCode {
    let mut engine = match Engine::open(&state) {
        Ok(engine) => engine,
        Err(error) => {
            let outcome = failure(
                EXIT_TRANSPORT,
                BoundaryErrorKind::Transport,
                error.to_string(),
                None,
            );
            return write_outcome(outcome);
        }
    };
    let mut reader = std::io::BufReader::new(std::io::stdin().lock());
    let mut stdout = std::io::stdout().lock();
    loop {
        let (outcome, shutdown) = match read_session_line(&mut reader, MAX_JSON_INPUT_BYTES) {
            Ok(SessionLine::End) => return ExitCode::SUCCESS,
            Ok(SessionLine::Data(input)) => match decode_request(&input) {
                Ok(envelope) => {
                    let shutdown = matches!(envelope.request, Request::Shutdown);
                    (process_decoded_rpc(&mut engine, envelope, false), shutdown)
                }
                Err(error) => (
                    failure(
                        EXIT_USAGE_OR_JSON,
                        error.kind,
                        error.to_string(),
                        request_id_hint(&input),
                    ),
                    false,
                ),
            },
            Ok(SessionLine::TooLarge) => (
                failure(
                    EXIT_USAGE_OR_JSON,
                    BoundaryErrorKind::InputTooLarge,
                    "session JSON line exceeds input byte policy".to_owned(),
                    None,
                ),
                false,
            ),
            Err(error) => {
                eprintln!("cannot read session JSON line: {error}");
                return ExitCode::from(EXIT_USAGE_OR_JSON);
            }
        };
        let mut output = outcome.stdout;
        output.push(b'\n');
        if let Err(error) = stdout.write_all(&output).and_then(|()| stdout.flush()) {
            eprintln!("cannot write session machine response: {error}");
            return ExitCode::from(EXIT_OUTPUT);
        }
        if shutdown {
            return ExitCode::SUCCESS;
        }
    }
}

fn read_stdin_bounded() -> Result<Vec<u8>, String> {
    let mut stdin = std::io::stdin().lock();
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let count = stdin
            .read(&mut buffer)
            .map_err(|error| format!("cannot read JSON request: {error}"))?;
        if count == 0 {
            return Ok(bytes);
        }
        if bytes.len().saturating_add(count) > MAX_JSON_INPUT_BYTES {
            return Err("JSON request exceeds input byte policy".to_owned());
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
}

fn success(stdout: Vec<u8>) -> CliOutcome {
    CliOutcome {
        stdout,
        diagnostic: None,
        exit: 0,
    }
}

fn failure(
    exit: u8,
    kind: BoundaryErrorKind,
    message: String,
    request_id: Option<lkjscript::RequestId>,
) -> CliOutcome {
    CliOutcome {
        stdout: encode_boundary_error(request_id, kind, &message),
        diagnostic: Some(message),
        exit,
    }
}

enum Command {
    Agent(agent::AgentCommand),
    Application(application::ApplicationCommand),
    Instance(instance::InstanceCommand),
    Runtime(runtime_cli::RuntimeCommand),
    Release(reusable_release::ReleaseCommand),
    Help,
    Rpc {
        state: PathBuf,
        pretty: bool,
    },
    Schema {
        request: DescribeSchemaRequest,
        pretty: bool,
    },
    Session {
        state: PathBuf,
    },
}

fn parse_command(mut arguments: impl Iterator<Item = String>) -> Result<Command, String> {
    let first = arguments.next().ok_or_else(|| usage("missing command"))?;
    if first == "--help" || first == "help" {
        if arguments.next().is_some() {
            return Err(usage("help accepts no arguments"));
        }
        return Ok(Command::Help);
    }
    if first == "agent" {
        return agent::parse(arguments).map(Command::Agent);
    }
    if first == "app" {
        return Ok(Command::Application(
            application::parse(arguments).unwrap_or_else(application::ApplicationCommand::Invalid),
        ));
    }
    if first == "instance" {
        return Ok(Command::Instance(
            instance::parse(arguments).unwrap_or_else(instance::InstanceCommand::Invalid),
        ));
    }
    if first == "release" {
        return Ok(Command::Release(
            reusable_release::parse(arguments)
                .unwrap_or_else(reusable_release::ReleaseCommand::Invalid),
        ));
    }
    if first == "runtime" {
        return Ok(Command::Runtime(
            runtime_cli::parse(arguments).unwrap_or_else(runtime_cli::RuntimeCommand::Invalid),
        ));
    }
    if first == "schema" {
        return parse_schema(arguments);
    }
    if first != "--state" {
        return Err(usage("expected schema or --state"));
    }
    let state = PathBuf::from(
        arguments
            .next()
            .ok_or_else(|| usage("missing state directory"))?,
    );
    match arguments.next().as_deref() {
        Some("rpc") => {
            let pretty = parse_pretty(arguments)?;
            Ok(Command::Rpc { state, pretty })
        }
        Some("session") if arguments.next().is_none() => Ok(Command::Session { state }),
        _ => Err(usage("expected rpc or session")),
    }
}

fn parse_schema(mut arguments: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut pretty = false;
    let mut full = false;
    let mut roots = Vec::new();
    let mut known_digest = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--pretty" if !pretty => pretty = true,
            "--full" if !full => full = true,
            "--root" => {
                let name = arguments
                    .next()
                    .ok_or_else(|| usage("missing schema root name"))?;
                roots.push(parse_schema_root(&name)?);
            }
            "--known-digest" if known_digest.is_none() => {
                let value = arguments
                    .next()
                    .ok_or_else(|| usage("missing known schema digest"))?;
                known_digest =
                    Some(value.parse::<MachineSchemaDigest>().map_err(|error| {
                        usage(&format!("invalid known schema digest: {error}"))
                    })?);
            }
            _ => return Err(usage("invalid or duplicate schema flag")),
        }
    }
    if full && !roots.is_empty() {
        return Err(usage("--full and --root cannot be combined"));
    }
    let projection = if full {
        SchemaProjection::Full
    } else if roots.is_empty() {
        SchemaProjection::Manifest
    } else {
        SchemaProjection::Roots { roots }
    };
    let request = DescribeSchemaRequest {
        projection,
        known_digest,
    };
    request.validate().map_err(usage)?;
    Ok(Command::Schema { request, pretty })
}

fn parse_schema_root(name: &str) -> Result<SchemaRoot, String> {
    SchemaRoot::ALL
        .into_iter()
        .find(|root| root.machine_name() == name)
        .ok_or_else(|| usage("unknown schema root"))
}

fn parse_pretty(mut arguments: impl Iterator<Item = String>) -> Result<bool, String> {
    match (arguments.next(), arguments.next()) {
        (None, None) => Ok(false),
        (Some(flag), None) if flag == "--pretty" => Ok(true),
        _ => Err(usage("unexpected argument")),
    }
}

fn usage(reason: &str) -> String {
    format!("{reason}; {}", usage_text().replace('\n', " | "))
}

fn usage_text() -> &'static str {
    "usage: lkjscript agent [COMMAND] [OPTIONS]\n       lkjscript release COMMAND [OPTIONS]\n       lkjscript app COMMAND [OPTIONS]\n       lkjscript instance COMMAND [OPTIONS]\n       lkjscript runtime COMMAND [OPTIONS]\n       lkjscript --state DIRECTORY (rpc [--pretty] | session)\n       lkjscript schema [--root NAME ... | --full] [--known-digest HEX] [--pretty]\n\nRun `lkjscript agent` for semantic authoring, `lkjscript release help` for immutable reuse, `lkjscript app help` for exact offline applications, `lkjscript instance help` for durable operation, and `lkjscript runtime help` for topology-neutral operation and its foreground session. Raw RPC and schema commands are exact low-level diagnostic surfaces."
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn session_line_reader_bounds_and_recovers_without_unbounded_buffering() {
        let mut reader = std::io::BufReader::new(Cursor::new(b"one\n\n12345\ntwo"));
        assert!(matches!(
            read_session_line(&mut reader, 4).unwrap(),
            SessionLine::Data(value) if value == b"one"
        ));
        assert!(matches!(
            read_session_line(&mut reader, 4).unwrap(),
            SessionLine::Data(value) if value.is_empty()
        ));
        assert!(matches!(
            read_session_line(&mut reader, 4).unwrap(),
            SessionLine::TooLarge
        ));
        assert!(matches!(
            read_session_line(&mut reader, 4).unwrap(),
            SessionLine::Data(value) if value == b"two"
        ));
        assert!(matches!(
            read_session_line(&mut reader, 4).unwrap(),
            SessionLine::End
        ));
    }
}
