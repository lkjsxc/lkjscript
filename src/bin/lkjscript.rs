use lkjscript::Client;
use lkjscript::daemon;
use lkjscript::machine::{
    BoundaryErrorKind, DescribeSchemaRequest, MAX_JSON_INPUT_BYTES, MachineSchemaDigest,
    SchemaProjection, SchemaSection, decode_request, encode_boundary_error, encode_response,
    encode_schema, request_id_hint,
};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const EXIT_USAGE_OR_JSON: u8 = 2;
const EXIT_TRANSPORT: u8 = 3;
const EXIT_OUTPUT: u8 = 4;

fn main() -> ExitCode {
    let outcome = run(std::env::args().skip(1));
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

fn run(arguments: impl Iterator<Item = String>) -> CliOutcome {
    match parse_command(arguments) {
        Ok(Command::Schema { request, pretty }) => match encode_schema(&request, pretty) {
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
        Ok(Command::Rpc { state, pretty }) => run_rpc(state, pretty),
        Err(message) => failure(EXIT_USAGE_OR_JSON, BoundaryErrorKind::Usage, message, None),
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
    let envelope = match decode_request(&input) {
        Ok(envelope) => envelope,
        Err(error) => {
            let request_id = request_id_hint(&input);
            return failure(
                EXIT_USAGE_OR_JSON,
                error.kind,
                error.to_string(),
                request_id,
            );
        }
    };
    let response = match Client::new(daemon::endpoint_path(&state))
        .request(envelope.request_id, &envelope.request)
    {
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
    Rpc {
        state: PathBuf,
        pretty: bool,
    },
    Schema {
        request: DescribeSchemaRequest,
        pretty: bool,
    },
}

fn parse_command(mut arguments: impl Iterator<Item = String>) -> Result<Command, String> {
    let first = arguments.next().ok_or_else(|| usage("missing command"))?;
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
    if arguments.next().as_deref() != Some("rpc") {
        return Err(usage("expected rpc"));
    }
    let pretty = parse_pretty(arguments)?;
    Ok(Command::Rpc { state, pretty })
}

fn parse_schema(mut arguments: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut pretty = false;
    let mut full = false;
    let mut sections = Vec::new();
    let mut known_digest = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--pretty" if !pretty => pretty = true,
            "--full" if !full => full = true,
            "--section" => {
                let name = arguments
                    .next()
                    .ok_or_else(|| usage("missing schema section name"))?;
                sections.push(parse_schema_section(&name)?);
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
    if full && !sections.is_empty() {
        return Err(usage("--full and --section cannot be combined"));
    }
    let projection = if full {
        SchemaProjection::Full
    } else if sections.is_empty() {
        SchemaProjection::Manifest
    } else {
        SchemaProjection::Sections { sections }
    };
    let request = DescribeSchemaRequest {
        projection,
        known_digest,
    };
    request.validate().map_err(usage)?;
    Ok(Command::Schema { request, pretty })
}

fn parse_schema_section(name: &str) -> Result<SchemaSection, String> {
    SchemaSection::ALL
        .into_iter()
        .find(|section| section.machine_name() == name)
        .ok_or_else(|| usage("unknown schema section"))
}

fn parse_pretty(mut arguments: impl Iterator<Item = String>) -> Result<bool, String> {
    match (arguments.next(), arguments.next()) {
        (None, None) => Ok(false),
        (Some(flag), None) if flag == "--pretty" => Ok(true),
        _ => Err(usage("unexpected argument")),
    }
}

fn usage(reason: &str) -> String {
    format!(
        "{reason}; usage: lkjscript --state DIRECTORY rpc [--pretty] | lkjscript schema [--section NAME ... | --full] [--known-digest HEX] [--pretty]"
    )
}
