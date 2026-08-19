use super::{
    BoundaryErrorKind, CliOutcome, EXIT_APPLICATION_INPUT, EXIT_ARTIFACT, EXIT_OUTPUT,
    EXIT_PROGRAM, EXIT_RESOURCE, EXIT_TRANSPORT, EXIT_USAGE_OR_JSON, failure, success, usage,
    write_outcome,
};
use lkjscript::error::{ErrorCode, LkError};
use lkjscript::machine::MAX_JSON_OUTPUT_BYTES;
use lkjscript::release::{self, RELEASE_CONTRACT_VERSION, ReleaseTestReport};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HELP: &str = "usage: lkjscript release COMMAND [OPTIONS]

Commands:
  validate --artifact FILE [--pretty]
  inspect --artifact FILE [--pretty]
  test --artifact FILE [--dependency FILE ...] [--pretty]

Reusable-release CLI JSON contract version 2 is required. These commands consume immutable release
distribution authority. Semantic projects create releases through `lkjscript target build`; the
removed command-local build predecessor is rejected. Artifact paths must be absolute.";

pub(super) enum ReleaseCommand {
    Invalid(String),
    Help,
    Validate {
        artifact: PathBuf,
        pretty: bool,
    },
    Inspect {
        artifact: PathBuf,
        pretty: bool,
    },
    Test {
        artifact: PathBuf,
        dependencies: Vec<PathBuf>,
        pretty: bool,
    },
}

pub(super) fn parse(arguments: impl Iterator<Item = String>) -> Result<ReleaseCommand, String> {
    let arguments = arguments.collect::<Vec<_>>();
    let Some(command) = arguments.first().map(String::as_str) else {
        return Ok(ReleaseCommand::Help);
    };
    let rest = &arguments[1..];
    match command {
        "help" | "--help" if rest.is_empty() => Ok(ReleaseCommand::Help),
        "validate" => parse_artifact(rest, ArtifactAction::Validate),
        "inspect" => parse_artifact(rest, ArtifactAction::Inspect),
        "test" => parse_artifact(rest, ArtifactAction::Test),
        _ => Err(release_usage(
            "unknown release command or unexpected argument",
        )),
    }
}

pub(super) fn run(command: ReleaseCommand) -> ExitCode {
    write_outcome(run_json(command))
}

fn run_json(command: ReleaseCommand) -> CliOutcome {
    match command {
        ReleaseCommand::Invalid(message) => release_error_with_exit(
            LkError::new(ErrorCode::ProtocolMalformed, message),
            false,
            EXIT_USAGE_OR_JSON,
        ),
        ReleaseCommand::Help => success(HELP.as_bytes().to_vec()),
        ReleaseCommand::Validate { artifact, pretty }
        | ReleaseCommand::Inspect { artifact, pretty } => {
            let bytes = match release::read_file(&artifact) {
                Ok(bytes) => bytes,
                Err(error) => return release_error(error, pretty),
            };
            match release::inspect(&bytes) {
                Ok(inspection) => encode_json(&inspection, pretty),
                Err(error) => release_error(error, pretty),
            }
        }
        ReleaseCommand::Test {
            artifact,
            dependencies,
            pretty,
        } => run_tests(&artifact, &dependencies, pretty),
    }
}

#[derive(Clone, Copy)]
enum ArtifactAction {
    Validate,
    Inspect,
    Test,
}

fn parse_artifact(arguments: &[String], action: ArtifactAction) -> Result<ReleaseCommand, String> {
    let mut artifact = None;
    let mut dependencies = Vec::new();
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
            "--dependency" if matches!(action, ArtifactAction::Test) => {
                dependencies.push(PathBuf::from(value_after(
                    arguments,
                    &mut index,
                    "--dependency",
                )?));
            }
            "--pretty" if !pretty => pretty = true,
            _ => return Err(release_usage("invalid or duplicate artifact option")),
        }
        index += 1;
    }
    let artifact = artifact.ok_or_else(|| release_usage("release command requires --artifact"))?;
    Ok(match action {
        ArtifactAction::Validate => ReleaseCommand::Validate { artifact, pretty },
        ArtifactAction::Inspect => ReleaseCommand::Inspect { artifact, pretty },
        ArtifactAction::Test => ReleaseCommand::Test {
            artifact,
            dependencies,
            pretty,
        },
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
        .ok_or_else(|| release_usage(&format!("{flag} requires a value")))
}

fn run_tests(artifact: &Path, dependencies: &[PathBuf], pretty: bool) -> CliOutcome {
    let bytes = match release::read_file(artifact) {
        Ok(bytes) => bytes,
        Err(error) => return release_error(error, pretty),
    };
    let dependency_bytes = match read_dependencies(dependencies) {
        Ok(bytes) => bytes,
        Err(error) => return release_error(error, pretty),
    };
    let inspection = match release::inspect(&bytes) {
        Ok(inspection) => inspection,
        Err(error) => return release_error(error, pretty),
    };
    let report = match release::test(&bytes, &dependency_bytes) {
        Ok(report) => report,
        Err(error) => return release_error(error, pretty),
    };
    #[derive(Serialize)]
    #[serde(deny_unknown_fields)]
    struct TestEnvelope {
        contract_version: u16,
        release: lkjscript::ReleaseId,
        report: ReleaseTestReport,
    }
    let failed = !report.all_passed();
    let mut outcome = encode_json(
        &TestEnvelope {
            contract_version: RELEASE_CONTRACT_VERSION,
            release: inspection.release,
            report,
        },
        pretty,
    );
    if failed && outcome.exit == 0 {
        outcome.exit = EXIT_PROGRAM;
        outcome.diagnostic = Some("one or more reusable release tests did not pass".into());
    }
    outcome
}

#[allow(clippy::result_large_err)]
fn read_dependencies(paths: &[PathBuf]) -> Result<Vec<Vec<u8>>, LkError> {
    paths.iter().map(|path| release::read_file(path)).collect()
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
            "release response exceeds JSON output policy".into(),
            None,
        ),
        Err(error) => failure(
            EXIT_OUTPUT,
            BoundaryErrorKind::Output,
            format!("cannot encode release response: {error}"),
            None,
        ),
    }
}

fn release_error(error: LkError, pretty: bool) -> CliOutcome {
    let exit = error_exit(&error);
    release_error_with_exit(error, pretty, exit)
}

fn release_error_with_exit(error: LkError, pretty: bool, exit: u8) -> CliOutcome {
    #[derive(Serialize)]
    #[serde(deny_unknown_fields)]
    struct ErrorEnvelope<'a> {
        contract_version: u16,
        error: &'a LkError,
    }
    let diagnostic = error.to_string();
    let mut outcome = encode_json(
        &ErrorEnvelope {
            contract_version: RELEASE_CONTRACT_VERSION,
            error: &error,
        },
        pretty,
    );
    outcome.exit = exit;
    outcome.diagnostic = Some(diagnostic);
    outcome
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

fn release_usage(reason: &str) -> String {
    usage(&format!(
        "{reason}; use `lkjscript release help` for reusable release details"
    ))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn release_help_is_default_and_options_are_closed() {
        assert!(matches!(
            parse(Vec::<String>::new().into_iter()).expect("default"),
            ReleaseCommand::Help
        ));
        assert!(parse(["build".to_owned()].into_iter()).is_err());
        assert!(
            parse(
                [
                    "validate".to_owned(),
                    "--artifact".to_owned(),
                    "/tmp/r".to_owned(),
                    "--dependency".to_owned(),
                    "/tmp/d".to_owned(),
                ]
                .into_iter()
            )
            .is_err()
        );
    }
}
