use super::{CliOutcome, EXIT_OUTPUT, EXIT_TRANSPORT, EXIT_USAGE_OR_JSON, failure, success, usage};
use lkjscript::engine::Engine;
use lkjscript::error::{ErrorCode, LkError};
use lkjscript::ids::{NodeId, RequestId, Revision, WorkspaceId};
use lkjscript::machine::{BoundaryErrorKind, MAX_JSON_OUTPUT_BYTES, active_machine_schema_digest};
use lkjscript::protocol::{PROTOCOL_VERSION, Request, Response};
use lkjscript::transaction::TransactionMode;
use lkjscript::workbench::{
    ContextBuildRequest, ContextPacket, ContextPacketDigest, ContextPurpose, DocumentError,
    MAX_CONTEXT_PACKET_BYTES, MAX_WORKBENCH_INPUT_BYTES, WORKBENCH_VERSION, authoring_help_cards,
    build_context_packet, decode_context_packet, encode_context_packet, parse_edit_document,
    parse_run_document, render_context_packet, render_function_document, render_semantic_diff,
};
use serde::Serialize;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

const ORIENTATION: &str = "A coding agent edits a typed, versioned program model through a directly opened local engine. The engine validates proposed changes, saves immutable revisions, and can seal an exact tested semantic closure as a standalone application artifact.\n\
\n\
Preferred workflow:\n\
  agent create   create an empty authoritative workspace\n\
  agent context  derive an exact revision-bound task packet\n\
  agent view     render a deterministic, non-authoritative semantic review\n\
  agent document render one packet-bound editable function document\n\
  agent validate parse and validate a bounded editable semantic document without publishing\n\
  agent apply    parse and atomically commit a bounded editable semantic document\n\
  agent diff     render exact semantic changes carried by a review packet\n\
  agent run      run an exact revision from a compact run document\n\
  app build      test and atomically seal an exact revision and entry\n\
  app inspect    inspect a validated artifact without source state\n\
  app test       rerun the artifact's immutable invocation cases\n\
  app run        invoke exact typed values from standalone artifact bytes\n\
  app stream     invoke a compatible pure bytes -> bytes process profile\n\
\n\
Documents are revision-, schema-, and scope-bound proposals. They normalize into the same typed transaction used by raw JSON and never become authority. Packet aliases use @n1 spelling, require the packet digest in the document, and never become persistent identity.\n\
\n\
Run `lkjscript agent help` for authoring details and `lkjscript app help` for the standalone lifecycle.";

const HELP_PREFIX: &str = "usage: lkjscript agent COMMAND [OPTIONS]\n\
\n\
Commands:\n\
  orient | help\n\
  create --state DIR [--pretty]\n\
  context --state DIR --workspace ID --revision N --purpose PURPOSE\n\
          [--target NODE ...] [--from-revision N] [--max-nodes N]\n\
          [--known-digest DIGEST] [--pretty]\n\
  view --packet FILE [--ids]\n\
  diff --packet FILE [--ids]\n\
  document --packet FILE\n\
  validate --state DIR [--packet FILE] [--pretty]    # editable document on stdin\n\
  apply --state DIR [--packet FILE] [--pretty]       # editable document on stdin\n\
  run --state DIR [--packet FILE] [--pretty]         # run document on stdin\n\
\n\
Standalone lifecycle:\n\
  lkjscript app build|validate|inspect|test|run|stream ...\n\
  Run `lkjscript app help` for exact artifact commands and path rules.\n\
\n\
Purposes: orient, create, repair, refactor, debug, extend, delete, review.\n\
Repair, refactor, debug, extend, and delete require targets. A review packet may use --from-revision to carry an exact semantic diff.\n\
\n\
Editable document grammar:\n\
  document { version 1 schema DIGEST workspace VALUE base_revision N\n\
             scope (workspace) | (function NODE) packet DIGEST edits [ ... ]\n\
         return_symbols [ ... ] idempotency_key VALUE }\n\
  run  { workspace VALUE revision N packet DIGEST entry VALUE\n\
         arguments [ ... ] policy { ... } }\n\
\n\
Objects are `{ field value ... }`, lists are `[ value ... ]`, and tagged variants are `(kind payload)` or `(kind)`. Strings use JSON quoting. Bare identifiers are strings. Booleans, null, and signed integers are literals. Packet aliases are unquoted @n1 values. Commas, semicolons, equals signs, comments, unknown fields, duplicate fields, trailing input, and implicit current-head lookup are rejected. Function scope accepts exactly one matching replace_function_body edit. Omit both packet and aliases for a packet-free document.";

fn help_text() -> Result<String, Box<LkError>> {
    authoring_help_cards()
        .map(|cards| format!("{HELP_PREFIX}\n\n{cards}"))
        .map_err(Box::new)
}

pub(super) enum AgentCommand {
    Orient,
    Help,
    Create {
        state: PathBuf,
        pretty: bool,
    },
    Context {
        state: PathBuf,
        request: ContextBuildRequest,
        known_digest: Option<ContextPacketDigest>,
        pretty: bool,
    },
    View {
        packet: PathBuf,
        full_ids: bool,
    },
    Diff {
        packet: PathBuf,
        full_ids: bool,
    },
    FunctionDocument {
        packet: PathBuf,
    },
    Edit {
        state: PathBuf,
        packet: Option<PathBuf>,
        mode: TransactionMode,
        pretty: bool,
    },
    Run {
        state: PathBuf,
        packet: Option<PathBuf>,
        pretty: bool,
    },
}

pub(super) fn parse(arguments: impl Iterator<Item = String>) -> Result<AgentCommand, String> {
    let arguments: Vec<_> = arguments.collect();
    let Some(command) = arguments.first().map(String::as_str) else {
        return Ok(AgentCommand::Orient);
    };
    let rest = &arguments[1..];
    match command {
        "orient" if rest.is_empty() => Ok(AgentCommand::Orient),
        "help" | "--help" if rest.is_empty() => Ok(AgentCommand::Help),
        "create" => parse_create(rest),
        "context" => parse_context(rest),
        "view" => parse_packet_view(rest, false),
        "diff" => parse_packet_view(rest, true),
        "document" => parse_function_document(rest),
        "validate" => parse_edit(rest, TransactionMode::ValidateOnly),
        "apply" => parse_edit(rest, TransactionMode::Commit),
        "run" => parse_run(rest),
        _ => Err(agent_usage("unknown agent command or unexpected argument")),
    }
}

pub(super) fn run(command: AgentCommand) -> CliOutcome {
    match command {
        AgentCommand::Orient => run_orientation(),
        AgentCommand::Help => match help_text() {
            Ok(help) => success(help.into_bytes()),
            Err(error) => agent_error(*error, false),
        },
        AgentCommand::Create { state, pretty } => request(
            &state,
            Request::CreateWorkspace,
            ExpectedResponse::Workspace,
            pretty,
        ),
        AgentCommand::Context {
            state,
            request,
            known_digest,
            pretty,
        } => run_context(&state, &request, known_digest, pretty),
        AgentCommand::View { packet, full_ids } => run_view(&packet, full_ids, false),
        AgentCommand::Diff { packet, full_ids } => run_view(&packet, full_ids, true),
        AgentCommand::FunctionDocument { packet } => run_function_document(&packet),
        AgentCommand::Edit {
            state,
            packet,
            mode,
            pretty,
        } => run_edit(&state, packet.as_deref(), mode, pretty),
        AgentCommand::Run {
            state,
            packet,
            pretty,
        } => run_document(&state, packet.as_deref(), pretty),
    }
}

fn parse_function_document(arguments: &[String]) -> Result<AgentCommand, String> {
    if arguments.len() != 2 || arguments[0] != "--packet" {
        return Err(agent_usage("document requires exactly --packet FILE"));
    }
    Ok(AgentCommand::FunctionDocument {
        packet: PathBuf::from(&arguments[1]),
    })
}

fn run_orientation() -> CliOutcome {
    match active_machine_schema_digest() {
        Ok(digest) => success(
            format!(
                "lkjscript semantic workbench v{WORKBENCH_VERSION}\n{ORIENTATION}\nactive protocol {PROTOCOL_VERSION} machine_schema {digest}"
            )
            .into_bytes(),
        ),
        Err(error) => agent_error(error, false),
    }
}

fn parse_create(arguments: &[String]) -> Result<AgentCommand, String> {
    let (state, packet, pretty) = parse_action_options(arguments)?;
    if packet.is_some() {
        return Err(agent_usage("create does not accept --packet"));
    }
    Ok(AgentCommand::Create { state, pretty })
}

fn parse_context(arguments: &[String]) -> Result<AgentCommand, String> {
    let mut state = None;
    let mut workspace = None;
    let mut revision = None;
    let mut purpose = None;
    let mut targets = Vec::new();
    let mut from_revision = None;
    let mut maximum_nodes = None;
    let mut known_digest = None;
    let mut pretty = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--state" if state.is_none() => {
                state = Some(PathBuf::from(value_after(
                    arguments, &mut index, "--state",
                )?));
            }
            "--workspace" if workspace.is_none() => {
                let value = value_after(arguments, &mut index, "--workspace")?;
                workspace = Some(
                    value
                        .parse::<WorkspaceId>()
                        .map_err(|error| agent_usage(&format!("invalid workspace ID: {error}")))?,
                );
            }
            "--revision" if revision.is_none() => {
                revision = Some(parse_revision(value_after(
                    arguments,
                    &mut index,
                    "--revision",
                )?)?);
            }
            "--purpose" if purpose.is_none() => {
                let value = value_after(arguments, &mut index, "--purpose")?;
                purpose = Some(value.parse::<ContextPurpose>().map_err(agent_usage)?);
            }
            "--target" => {
                let value = value_after(arguments, &mut index, "--target")?;
                targets.push(
                    value.parse::<NodeId>().map_err(|error| {
                        agent_usage(&format!("invalid target Node ID: {error}"))
                    })?,
                );
            }
            "--from-revision" if from_revision.is_none() => {
                from_revision = Some(parse_revision(value_after(
                    arguments,
                    &mut index,
                    "--from-revision",
                )?)?);
            }
            "--max-nodes" if maximum_nodes.is_none() => {
                let value = value_after(arguments, &mut index, "--max-nodes")?;
                maximum_nodes = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| agent_usage("--max-nodes must be a canonical u32"))?,
                );
            }
            "--known-digest" if known_digest.is_none() => {
                let value = value_after(arguments, &mut index, "--known-digest")?;
                known_digest = Some(value.parse::<ContextPacketDigest>().map_err(|error| {
                    agent_usage(&format!("invalid context packet digest: {error}"))
                })?);
            }
            "--pretty" if !pretty => pretty = true,
            _ => return Err(agent_usage("invalid or duplicate context option")),
        }
        index += 1;
    }
    let state = state.ok_or_else(|| agent_usage("context requires --state"))?;
    let workspace = workspace.ok_or_else(|| agent_usage("context requires --workspace"))?;
    let revision = revision.ok_or_else(|| agent_usage("context requires --revision"))?;
    let purpose = purpose.ok_or_else(|| agent_usage("context requires --purpose"))?;
    let mut request = ContextBuildRequest::new(workspace, revision, purpose);
    request.targets = targets;
    request.from_revision = from_revision;
    if let Some(maximum_nodes) = maximum_nodes {
        request.maximum_nodes = maximum_nodes;
    }
    Ok(AgentCommand::Context {
        state,
        request,
        known_digest,
        pretty,
    })
}

fn parse_packet_view(arguments: &[String], diff: bool) -> Result<AgentCommand, String> {
    let mut packet = None;
    let mut full_ids = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--packet" if packet.is_none() => {
                packet = Some(PathBuf::from(value_after(
                    arguments, &mut index, "--packet",
                )?));
            }
            "--ids" if !full_ids => full_ids = true,
            _ => return Err(agent_usage("invalid or duplicate view option")),
        }
        index += 1;
    }
    let packet = packet.ok_or_else(|| agent_usage("view and diff require --packet"))?;
    if diff {
        Ok(AgentCommand::Diff { packet, full_ids })
    } else {
        Ok(AgentCommand::View { packet, full_ids })
    }
}

fn parse_edit(arguments: &[String], mode: TransactionMode) -> Result<AgentCommand, String> {
    let (state, packet, pretty) = parse_action_options(arguments)?;
    Ok(AgentCommand::Edit {
        state,
        packet,
        mode,
        pretty,
    })
}

fn parse_run(arguments: &[String]) -> Result<AgentCommand, String> {
    let (state, packet, pretty) = parse_action_options(arguments)?;
    Ok(AgentCommand::Run {
        state,
        packet,
        pretty,
    })
}

fn parse_action_options(arguments: &[String]) -> Result<(PathBuf, Option<PathBuf>, bool), String> {
    let mut state = None;
    let mut packet = None;
    let mut pretty = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--state" if state.is_none() => {
                state = Some(PathBuf::from(value_after(
                    arguments, &mut index, "--state",
                )?));
            }
            "--packet" if packet.is_none() => {
                packet = Some(PathBuf::from(value_after(
                    arguments, &mut index, "--packet",
                )?));
            }
            "--pretty" if !pretty => pretty = true,
            _ => return Err(agent_usage("invalid or duplicate action option")),
        }
        index += 1;
    }
    Ok((
        state.ok_or_else(|| agent_usage("action requires --state"))?,
        packet,
        pretty,
    ))
}

fn value_after<'a>(
    arguments: &'a [String],
    index: &mut usize,
    flag: &str,
) -> Result<&'a str, String> {
    *index = index.saturating_add(1);
    arguments
        .get(*index)
        .map(String::as_str)
        .ok_or_else(|| agent_usage(&format!("{flag} requires a value")))
}

fn parse_revision(value: &str) -> Result<Revision, String> {
    let number = value
        .parse::<u64>()
        .map_err(|_| agent_usage("revision must be a canonical u64"))?;
    if number.to_string() != value {
        return Err(agent_usage("revision must use canonical decimal spelling"));
    }
    Ok(Revision::new(number))
}

fn run_context(
    state: &Path,
    request: &ContextBuildRequest,
    known_digest: Option<ContextPacketDigest>,
    pretty: bool,
) -> CliOutcome {
    let mut engine = match Engine::open(state) {
        Ok(engine) => engine,
        Err(error) => return agent_error_with_exit(error, pretty, EXIT_TRANSPORT),
    };
    let packet = match build_context_packet(&mut engine, request) {
        Ok(packet) => packet,
        Err(error) => return agent_error(error, pretty),
    };
    if known_digest == Some(packet.digest) {
        #[derive(Serialize)]
        struct UnchangedContext {
            version: u16,
            digest: ContextPacketDigest,
            unchanged: bool,
        }
        let response = UnchangedContext {
            version: WORKBENCH_VERSION,
            digest: packet.digest,
            unchanged: true,
        };
        let encoded = if pretty {
            serde_json::to_vec_pretty(&response)
        } else {
            serde_json::to_vec(&response)
        };
        return match encoded {
            Ok(output) => success(output),
            Err(error) => agent_error(
                LkError::new(
                    ErrorCode::ProtocolMalformed,
                    format!("cannot encode unchanged context response: {error}"),
                ),
                pretty,
            ),
        };
    }
    match encode_context_packet(&packet, pretty) {
        Ok(output) => success(output),
        Err(error) => agent_error(error, pretty),
    }
}

fn run_view(packet_path: &Path, full_ids: bool, diff: bool) -> CliOutcome {
    let packet = match read_packet(packet_path) {
        Ok(packet) => packet,
        Err(error) => return agent_error(*error, false),
    };
    let rendered = if diff {
        render_semantic_diff(&packet, full_ids)
    } else {
        render_context_packet(&packet, full_ids)
    };
    match rendered {
        Ok(output) => success(output),
        Err(error) => agent_error(error, false),
    }
}

fn run_function_document(packet_path: &Path) -> CliOutcome {
    let packet = match read_packet(packet_path) {
        Ok(packet) => packet,
        Err(error) => return agent_error(*error, false),
    };
    match render_function_document(&packet) {
        Ok(output) => success(output),
        Err(error) => agent_error(error, false),
    }
}

fn run_edit(
    state: &Path,
    packet_path: Option<&Path>,
    mode: TransactionMode,
    pretty: bool,
) -> CliOutcome {
    let input = match read_stdin_bounded(MAX_WORKBENCH_INPUT_BYTES, "editable semantic document") {
        Ok(input) => input,
        Err(error) => return agent_error(*error, pretty),
    };
    let packet = match packet_path.map(read_packet).transpose() {
        Ok(packet) => packet,
        Err(error) => return agent_error(*error, pretty),
    };
    let parsed = match parse_edit_document(&input, mode, packet.as_ref()) {
        Ok(parsed) => parsed,
        Err(error) => return document_error(error, pretty),
    };
    request(
        state,
        Request::ApplyTransaction(parsed.request),
        ExpectedResponse::Transaction,
        pretty,
    )
}

fn run_document(state: &Path, packet_path: Option<&Path>, pretty: bool) -> CliOutcome {
    let input = match read_stdin_bounded(MAX_WORKBENCH_INPUT_BYTES, "run document") {
        Ok(input) => input,
        Err(error) => return agent_error(*error, pretty),
    };
    let packet = match packet_path.map(read_packet).transpose() {
        Ok(packet) => packet,
        Err(error) => return agent_error(*error, pretty),
    };
    let parsed = match parse_run_document(&input, packet.as_ref()) {
        Ok(parsed) => parsed,
        Err(error) => return document_error(error, pretty),
    };
    request(
        state,
        Request::Run {
            workspace: parsed.workspace,
            revision: parsed.revision,
            entry: parsed.entry,
            arguments: parsed.arguments,
            policy: parsed.policy,
        },
        ExpectedResponse::Run,
        pretty,
    )
}

enum ExpectedResponse {
    Workspace,
    Transaction,
    Run,
}

fn request(state: &Path, request: Request, expected: ExpectedResponse, pretty: bool) -> CliOutcome {
    let mut engine = match Engine::open(state) {
        Ok(engine) => engine,
        Err(error) => return agent_error_with_exit(error, pretty, EXIT_TRANSPORT),
    };
    let response = match engine.request(RequestId::new(1), request) {
        Ok(response) => response,
        Err(error) => return agent_error_with_exit(error, pretty, EXIT_TRANSPORT),
    };
    let right_family = matches!(
        (&expected, &response),
        (ExpectedResponse::Workspace, Response::WorkspaceCreated(_))
            | (
                ExpectedResponse::Transaction,
                Response::TransactionReceipt(_)
            )
            | (ExpectedResponse::Run, Response::Run(_))
            | (_, Response::Error(_))
    );
    if !right_family {
        return agent_error_with_exit(
            LkError::new(
                ErrorCode::ProtocolMalformed,
                "engine returned the wrong response family for the workbench action",
            ),
            pretty,
            EXIT_TRANSPORT,
        );
    }
    encode_logical_response(&response, pretty)
}

fn encode_logical_response(response: &Response, pretty: bool) -> CliOutcome {
    let encoded = if pretty {
        serde_json::to_vec_pretty(response)
    } else {
        serde_json::to_vec(response)
    };
    match encoded {
        Ok(output) if output.len() <= MAX_JSON_OUTPUT_BYTES => success(output),
        Ok(_) => failure(
            EXIT_OUTPUT,
            BoundaryErrorKind::Output,
            "workbench response exceeds the output byte policy".to_owned(),
            None,
        ),
        Err(error) => failure(
            EXIT_OUTPUT,
            BoundaryErrorKind::Output,
            format!("cannot encode workbench response: {error}"),
            None,
        ),
    }
}

fn read_packet(path: &Path) -> Result<ContextPacket, Box<LkError>> {
    let bytes = read_file_bounded(path, MAX_CONTEXT_PACKET_BYTES, "context packet")?;
    decode_context_packet(&bytes).map_err(Box::new)
}

fn read_file_bounded(path: &Path, maximum: usize, label: &str) -> Result<Vec<u8>, Box<LkError>> {
    let file = File::open(path).map_err(|error| {
        Box::new(LkError::new(
            ErrorCode::Io,
            format!("cannot open {label} {}: {error}", path.display()),
        ))
    })?;
    let limit = u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::new();
    file.take(limit).read_to_end(&mut bytes).map_err(|error| {
        Box::new(LkError::new(
            ErrorCode::Io,
            format!("cannot read {label} {}: {error}", path.display()),
        ))
    })?;
    if bytes.len() > maximum {
        return Err(Box::new(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{label} exceeds its byte policy"),
        )));
    }
    Ok(bytes)
}

fn read_stdin_bounded(maximum: usize, label: &str) -> Result<Vec<u8>, Box<LkError>> {
    let stdin = std::io::stdin().lock();
    let mut bytes = Vec::new();
    let limit = u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1);
    stdin.take(limit).read_to_end(&mut bytes).map_err(|error| {
        Box::new(LkError::new(
            ErrorCode::Io,
            format!("cannot read {label} from stdin: {error}"),
        ))
    })?;
    if bytes.len() > maximum {
        return Err(Box::new(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{label} exceeds its byte policy"),
        )));
    }
    Ok(bytes)
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct DocumentErrorEnvelope<'a> {
    workbench_version: u16,
    kind: &'static str,
    error: &'a DocumentError,
}

fn document_error(error: DocumentError, pretty: bool) -> CliOutcome {
    let envelope = DocumentErrorEnvelope {
        workbench_version: WORKBENCH_VERSION,
        kind: "document_error",
        error: &error,
    };
    let encoded = if pretty {
        serde_json::to_vec_pretty(&envelope)
    } else {
        serde_json::to_vec(&envelope)
    };
    match encoded {
        Ok(output) => CliOutcome {
            stdout: output,
            diagnostic: Some(error.to_string()),
            exit: EXIT_USAGE_OR_JSON,
        },
        Err(encoding) => failure(
            EXIT_OUTPUT,
            BoundaryErrorKind::Output,
            format!("cannot encode document error: {encoding}"),
            None,
        ),
    }
}

fn agent_error(error: LkError, pretty: bool) -> CliOutcome {
    let exit = if error.code == ErrorCode::Io {
        EXIT_TRANSPORT
    } else {
        EXIT_USAGE_OR_JSON
    };
    agent_error_with_exit(error, pretty, exit)
}

fn agent_error_with_exit(error: LkError, pretty: bool, exit: u8) -> CliOutcome {
    let diagnostic = error.to_string();
    let mut outcome = encode_logical_response(&Response::Error(error), pretty);
    outcome.exit = exit;
    outcome.diagnostic = Some(diagnostic);
    outcome
}

fn agent_usage(reason: &str) -> String {
    usage(&format!(
        "{reason}; use `lkjscript agent help` for workbench details"
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn agent_help_is_the_default_and_context_options_are_strict() {
        assert!(matches!(
            parse(Vec::<String>::new().into_iter()).unwrap(),
            AgentCommand::Orient
        ));
        assert!(parse(["context".to_owned()].into_iter()).is_err());
        assert!(parse(["view".to_owned(), "--ids".to_owned()].into_iter()).is_err());
    }

    #[test]
    fn orientation_reports_the_active_schema_digest() {
        let digest = active_machine_schema_digest().unwrap().to_string();
        assert_eq!(digest.len(), 64);
        let output = String::from_utf8(run_orientation().stdout).unwrap();
        assert!(output.contains(&format!("semantic workbench v{WORKBENCH_VERSION}")));
        assert!(output.contains(&digest));
    }

    #[test]
    fn help_distinguishes_draft_targets_aliases_types_and_idempotency() {
        let help = help_text().unwrap();
        assert!(help.contains("(draft SYMBOL) | (existing NODE_OR_@ALIAS)"));
        assert!(help.contains("unit | bool | i64 | bytes"));
        assert!(help.contains("@ spelling is reserved exclusively for aliases"));
        assert!(help.contains("Validate documents must omit idempotency_key"));
        assert!(help.contains("exactly 32 lowercase hexadecimal characters"));
    }
}
