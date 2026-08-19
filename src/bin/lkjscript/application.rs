use super::{
    BoundaryErrorKind, CliOutcome, EXIT_APPLICATION_INPUT, EXIT_ARTIFACT, EXIT_OUTPUT,
    EXIT_PROGRAM, EXIT_RESOURCE, EXIT_TRANSPORT, EXIT_USAGE_OR_JSON, failure, success, usage,
    write_outcome,
};
use lkjscript::application::{
    APPLICATION_CONTRACT_VERSION, ApplicationInvocation, ApplicationTestReport,
};
use lkjscript::error::{ErrorCode, LkError};
use lkjscript::machine::{MAX_JSON_INPUT_BYTES, MAX_JSON_OUTPUT_BYTES};
use lkjscript::runtime::{RuntimeKernel, RuntimePolicy};
use lkjscript::schema::MAXIMUM_BYTE_STRING_BYTES;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HELP: &str = "usage: lkjscript app COMMAND [OPTIONS]

Commands:
  validate --artifact FILE [--pretty]
  inspect --artifact FILE [--pretty]
  test --artifact FILE [--pretty]
  run --artifact FILE [--pretty]       # strict ApplicationInvocation JSON on stdin
  stream --artifact FILE               # raw bytes on stdin and stdout

Application CLI JSON contract version 5 is required on inputs and reported on outputs. These
commands consume immutable application distribution authority. Semantic projects create
applications through `lkjscript target build`; the removed command-local build predecessor is
rejected. Artifact paths must be absolute.";

pub(super) enum ApplicationCommand {
    Invalid(String),
    Help,
    Validate { artifact: PathBuf, pretty: bool },
    Inspect { artifact: PathBuf, pretty: bool },
    Test { artifact: PathBuf, pretty: bool },
    Run { artifact: PathBuf, pretty: bool },
    Stream { artifact: PathBuf },
}

pub(super) fn parse(arguments: impl Iterator<Item = String>) -> Result<ApplicationCommand, String> {
    let arguments = arguments.collect::<Vec<_>>();
    let Some(command) = arguments.first().map(String::as_str) else {
        return Ok(ApplicationCommand::Help);
    };
    let rest = &arguments[1..];
    match command {
        "help" | "--help" if rest.is_empty() => Ok(ApplicationCommand::Help),
        "validate" => parse_artifact_action(rest, ArtifactAction::Validate),
        "inspect" => parse_artifact_action(rest, ArtifactAction::Inspect),
        "test" => parse_artifact_action(rest, ArtifactAction::Test),
        "run" => parse_artifact_action(rest, ArtifactAction::Run),
        "stream" => parse_artifact_action(rest, ArtifactAction::Stream),
        _ => Err(app_usage(
            "unknown application command or unexpected argument",
        )),
    }
}

pub(super) fn run(command: ApplicationCommand) -> ExitCode {
    match command {
        ApplicationCommand::Stream { artifact } => run_stream(&artifact),
        command => write_outcome(run_json(command)),
    }
}

fn run_json(command: ApplicationCommand) -> CliOutcome {
    match command {
        ApplicationCommand::Invalid(message) => application_error_with_exit(
            LkError::new(ErrorCode::ProtocolMalformed, message),
            false,
            EXIT_USAGE_OR_JSON,
        ),
        ApplicationCommand::Help => success(HELP.as_bytes().to_vec()),
        ApplicationCommand::Validate { artifact, pretty }
        | ApplicationCommand::Inspect { artifact, pretty } => {
            let mut kernel = match RuntimeKernel::new(RuntimePolicy::default()) {
                Ok(kernel) => kernel,
                Err(error) => return application_error(error, pretty),
            };
            match kernel.inspect_application_path(&artifact) {
                Ok(inspection) => encode_json(&inspection, pretty),
                Err(error) => application_error(error, pretty),
            }
        }
        ApplicationCommand::Test { artifact, pretty } => run_tests(&artifact, pretty),
        ApplicationCommand::Run { artifact, pretty } => run_typed(&artifact, pretty),
        ApplicationCommand::Stream { .. } => failure(
            EXIT_USAGE_OR_JSON,
            BoundaryErrorKind::Usage,
            app_usage("stream must own the raw process boundary"),
            None,
        ),
    }
}

#[derive(Clone, Copy)]
enum ArtifactAction {
    Validate,
    Inspect,
    Test,
    Run,
    Stream,
}

fn parse_artifact_action(
    arguments: &[String],
    action: ArtifactAction,
) -> Result<ApplicationCommand, String> {
    let mut artifact = None;
    let mut pretty = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--artifact" if artifact.is_none() => {
                artifact = Some(PathBuf::from(value_after(
                    arguments,
                    &mut index,
                    "--artifact",
                )?));
            }
            "--pretty" if !pretty && !matches!(action, ArtifactAction::Stream) => pretty = true,
            _ => return Err(app_usage("invalid or duplicate artifact option")),
        }
        index += 1;
    }
    let artifact = artifact.ok_or_else(|| app_usage("application command requires --artifact"))?;
    Ok(match action {
        ArtifactAction::Validate => ApplicationCommand::Validate { artifact, pretty },
        ArtifactAction::Inspect => ApplicationCommand::Inspect { artifact, pretty },
        ArtifactAction::Test => ApplicationCommand::Test { artifact, pretty },
        ArtifactAction::Run => ApplicationCommand::Run { artifact, pretty },
        ArtifactAction::Stream => ApplicationCommand::Stream { artifact },
    })
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
        .ok_or_else(|| app_usage(&format!("{flag} requires a value")))
}

fn run_tests(path: &Path, pretty: bool) -> CliOutcome {
    let mut kernel = match RuntimeKernel::new(RuntimePolicy::default()) {
        Ok(kernel) => kernel,
        Err(error) => return application_error(error, pretty),
    };
    let inspection = match kernel.inspect_application_path(path) {
        Ok(inspection) => inspection,
        Err(error) => return application_error(error, pretty),
    };
    let report = match kernel.test_application_path(path) {
        Ok(report) => report,
        Err(error) => return application_error(error, pretty),
    };
    #[derive(Serialize)]
    #[serde(deny_unknown_fields)]
    struct TestEnvelope {
        contract_version: u16,
        digest: lkjscript::ApplicationDigest,
        report: ApplicationTestReport,
    }
    let failed = !report.all_passed();
    let mut outcome = encode_json(
        &TestEnvelope {
            contract_version: APPLICATION_CONTRACT_VERSION,
            digest: inspection.digest,
            report,
        },
        pretty,
    );
    if failed && outcome.exit == 0 {
        outcome.exit = EXIT_PROGRAM;
        outcome.diagnostic = Some("one or more application tests did not pass".into());
    }
    outcome
}

fn run_typed(path: &Path, pretty: bool) -> CliOutcome {
    let input = match read_stdin(MAX_JSON_INPUT_BYTES, "application invocation") {
        Ok(input) => input,
        Err(error) => return application_error(error, pretty),
    };
    let invocation = match decode_json::<ApplicationInvocation>(&input, "application invocation") {
        Ok(invocation) => invocation,
        Err(error) => return application_error(error, pretty),
    };
    let mut kernel = match RuntimeKernel::new(RuntimePolicy::default()) {
        Ok(kernel) => kernel,
        Err(error) => return application_error(error, pretty),
    };
    match kernel.run_application_path(path, &invocation) {
        Ok(receipt) => encode_json(&receipt, pretty),
        Err(error) => application_error(error, pretty),
    }
}

fn run_stream(path: &Path) -> ExitCode {
    let input = match read_stdin(MAXIMUM_BYTE_STRING_BYTES, "application stream input") {
        Ok(input) => input,
        Err(error) => return stream_error(error),
    };
    let mut kernel = match RuntimeKernel::new(RuntimePolicy::default()) {
        Ok(kernel) => kernel,
        Err(error) => return stream_error(error),
    };
    let output = match kernel.run_stream_application_path(path, &input) {
        Ok(output) => output,
        Err(error) => return stream_error(error),
    };
    if let Err(error) = std::io::stdout().lock().write_all(&output) {
        eprintln!("cannot write application stream output: {error}");
        return ExitCode::from(EXIT_OUTPUT);
    }
    ExitCode::SUCCESS
}

fn stream_error(error: LkError) -> ExitCode {
    let encoded = serde_json::to_vec(&ApplicationErrorEnvelope {
        contract_version: APPLICATION_CONTRACT_VERSION,
        error: &error,
    })
    .unwrap_or_else(|_| {
        b"{\"contract_version\":4,\"error\":{\"code\":\"io\",\"related\":[],\"retryable\":false,\"message\":\"cannot encode application error\"}}".to_vec()
    });
    let mut stderr = std::io::stderr().lock();
    let _ = stderr.write_all(&encoded);
    let _ = stderr.write_all(b"\n");
    ExitCode::from(error_exit(&error))
}

#[allow(clippy::result_large_err)]
fn decode_json<T: DeserializeOwned>(bytes: &[u8], label: &str) -> Result<T, LkError> {
    serde_json::from_slice(bytes).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot decode {label} JSON: {error}"),
        )
    })
}

#[allow(clippy::result_large_err)]
fn read_stdin(maximum: usize, label: &str) -> Result<Vec<u8>, LkError> {
    let stdin = std::io::stdin().lock();
    let limit = u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::new();
    stdin.take(limit).read_to_end(&mut bytes).map_err(|error| {
        LkError::new(
            ErrorCode::Io,
            format!("cannot read {label} from stdin: {error}"),
        )
    })?;
    if bytes.len() > maximum {
        return Err(LkError::new(
            if label == "application stream input" {
                ErrorCode::RuntimeByteInputTooLarge
            } else {
                ErrorCode::PolicyExceeded
            },
            format!("{label} exceeds its byte policy"),
        ));
    }
    Ok(bytes)
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
            "application response exceeds JSON output policy".into(),
            None,
        ),
        Err(error) => failure(
            EXIT_OUTPUT,
            BoundaryErrorKind::Output,
            format!("cannot encode application response: {error}"),
            None,
        ),
    }
}

fn application_error(error: LkError, pretty: bool) -> CliOutcome {
    let exit = error_exit(&error);
    application_error_with_exit(error, pretty, exit)
}

fn application_error_with_exit(error: LkError, pretty: bool, exit: u8) -> CliOutcome {
    let diagnostic = error.to_string();
    let mut outcome = encode_json(
        &ApplicationErrorEnvelope {
            contract_version: APPLICATION_CONTRACT_VERSION,
            error: &error,
        },
        pretty,
    );
    outcome.exit = exit;
    outcome.diagnostic = Some(diagnostic);
    outcome
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ApplicationErrorEnvelope<'a> {
    contract_version: u16,
    error: &'a LkError,
}

fn error_exit(error: &LkError) -> u8 {
    match error.code {
        ErrorCode::ArtifactCorrupt | ErrorCode::ArtifactPublicationOutcomeUnknown => EXIT_ARTIFACT,
        ErrorCode::ApplicationTestFailed
        | ErrorCode::RuntimeTrap
        | ErrorCode::ByteIndexOutOfBounds
        | ErrorCode::ByteSliceOutOfBounds => EXIT_PROGRAM,
        ErrorCode::ExecutionFuelExhausted
        | ErrorCode::ExecutionFrameExhausted
        | ErrorCode::RuntimeByteInputTooLarge
        | ErrorCode::ManagedObjectPolicyExceeded
        | ErrorCode::ManagedVisibleBytePolicyExceeded
        | ErrorCode::RetainedBytePolicyExceeded
        | ErrorCode::ResultBytePolicyExceeded
        | ErrorCode::ByteValueTooLarge
        | ErrorCode::ExecutionMemoryExhausted => EXIT_RESOURCE,
        ErrorCode::Io | ErrorCode::AuthorityBusy | ErrorCode::CommitOutcomeUnknown => {
            EXIT_TRANSPORT
        }
        _ => EXIT_APPLICATION_INPUT,
    }
}

fn app_usage(reason: &str) -> String {
    usage(&format!(
        "{reason}; use `lkjscript app help` for application lifecycle details"
    ))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn application_help_is_default_and_options_are_closed() {
        assert!(matches!(
            parse(Vec::<String>::new().into_iter()).expect("default"),
            ApplicationCommand::Help
        ));
        assert!(parse(["build".to_owned()].into_iter()).is_err());
        assert!(
            parse(
                [
                    "stream".to_owned(),
                    "--artifact".to_owned(),
                    "/tmp/a".to_owned(),
                    "--pretty".to_owned(),
                ]
                .into_iter()
            )
            .is_err()
        );
    }

    #[test]
    fn test_status_vocabulary_carries_required_non_pass_outcomes() {
        let statuses = [
            lkjscript::ApplicationTestStatus::Passed,
            lkjscript::ApplicationTestStatus::UnexpectedValue,
            lkjscript::ApplicationTestStatus::UnexpectedTrap,
            lkjscript::ApplicationTestStatus::MissingTrap,
            lkjscript::ApplicationTestStatus::WrongTrap,
            lkjscript::ApplicationTestStatus::InvalidCase,
            lkjscript::ApplicationTestStatus::Incomplete,
            lkjscript::ApplicationTestStatus::ResourceFailure,
            lkjscript::ApplicationTestStatus::EngineFailure,
        ];
        let encoded = serde_json::to_string(&statuses).expect("statuses");
        for name in [
            "passed",
            "unexpected_value",
            "unexpected_trap",
            "missing_trap",
            "wrong_trap",
            "invalid_case",
            "incomplete",
            "resource_failure",
            "engine_failure",
        ] {
            assert!(encoded.contains(name));
        }
    }
}
