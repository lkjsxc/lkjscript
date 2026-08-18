#![allow(clippy::result_large_err)]

use super::{
    BoundaryErrorKind, CliOutcome, EXIT_APPLICATION_INPUT, EXIT_ARTIFACT, EXIT_OUTPUT,
    EXIT_RESOURCE, EXIT_TRANSPORT, EXIT_USAGE_OR_JSON, failure, read_stdin_bounded, success, usage,
    write_outcome,
};
use lkjscript::application;
use lkjscript::error::{ErrorCode, LkError};
use lkjscript::instance::{
    INSTANCE_CONTRACT_VERSION, InstanceCreateRequest, InstanceDeleteRequest, InstanceEventRequest,
    InstanceFakeHostRequest, InstanceHostRequest, InstanceId, InstanceResumeRequest, InstanceStore,
    strict_json,
};
use lkjscript::machine::MAX_JSON_OUTPUT_BYTES;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HELP: &str = "usage: lkjscript instance COMMAND [OPTIONS]

Commands:
  create --store DIRECTORY --application FILE [--pretty]
          # strict InstanceCreateRequest JSON on stdin
  validate-event --store DIRECTORY [--pretty]
          # strict InstanceEventRequest JSON on stdin; mode must be validate_only
  apply-event --store DIRECTORY [--pretty]
          # strict InstanceEventRequest JSON on stdin; mode must be commit
  validate-application --store DIRECTORY [--pretty]
          # strict InstanceHostRequest JSON on stdin
  execute-activation --store DIRECTORY [--pretty]
          # strict InstanceHostRequest JSON on stdin
  reconcile-activation --store DIRECTORY [--pretty]
          # strict InstanceHostRequest JSON on stdin
  fake-outcome --store DIRECTORY [--pretty]
          # strict InstanceFakeHostRequest JSON on stdin; fake-bound instances only
  validate-resume --store DIRECTORY [--pretty]
          # strict InstanceResumeRequest JSON on stdin; mode must be validate_only
  resume --store DIRECTORY [--pretty]
          # strict InstanceResumeRequest JSON on stdin; mode must be commit
  inspect --store DIRECTORY --instance HEX [--pretty]
  history --store DIRECTORY --instance HEX [--start REVISION] [--limit COUNT] [--pretty]
  delete --store DIRECTORY [--pretty]
          # strict InstanceDeleteRequest JSON on stdin

Instance CLI JSON contract version 1 is required. Store, application, activation-source, and slot
paths are bounded canonical absolute paths. A committed event or resume requires an instance-scoped
event key. Host execution records an exact typed outcome but never mutates semantic state; resume
is the only path that lets a host outcome enter the next deterministic transition. A possibly
visible activation is never repeated automatically.";

pub(super) enum InstanceCommand {
    Invalid(String),
    Help,
    Create {
        store: PathBuf,
        application: PathBuf,
        pretty: bool,
    },
    ValidateEvent {
        store: PathBuf,
        pretty: bool,
    },
    ApplyEvent {
        store: PathBuf,
        pretty: bool,
    },
    ValidateApplication {
        store: PathBuf,
        pretty: bool,
    },
    ExecuteActivation {
        store: PathBuf,
        pretty: bool,
    },
    ReconcileActivation {
        store: PathBuf,
        pretty: bool,
    },
    FakeOutcome {
        store: PathBuf,
        pretty: bool,
    },
    ValidateResume {
        store: PathBuf,
        pretty: bool,
    },
    Resume {
        store: PathBuf,
        pretty: bool,
    },
    Inspect {
        store: PathBuf,
        instance: InstanceId,
        pretty: bool,
    },
    History {
        store: PathBuf,
        instance: InstanceId,
        start: u64,
        limit: usize,
        pretty: bool,
    },
    Delete {
        store: PathBuf,
        pretty: bool,
    },
}

pub(super) fn parse(arguments: impl Iterator<Item = String>) -> Result<InstanceCommand, String> {
    let arguments = arguments.collect::<Vec<_>>();
    let Some(command) = arguments.first().map(String::as_str) else {
        return Ok(InstanceCommand::Help);
    };
    let rest = &arguments[1..];
    match command {
        "help" | "--help" if rest.is_empty() => Ok(InstanceCommand::Help),
        "create" => parse_create(rest),
        "validate-event" => parse_store_action(rest, StoreAction::ValidateEvent),
        "apply-event" => parse_store_action(rest, StoreAction::ApplyEvent),
        "validate-application" => parse_store_action(rest, StoreAction::ValidateApplication),
        "execute-activation" => parse_store_action(rest, StoreAction::ExecuteActivation),
        "reconcile-activation" => parse_store_action(rest, StoreAction::ReconcileActivation),
        "fake-outcome" => parse_store_action(rest, StoreAction::FakeOutcome),
        "validate-resume" => parse_store_action(rest, StoreAction::ValidateResume),
        "resume" => parse_store_action(rest, StoreAction::Resume),
        "inspect" => parse_query(rest, false),
        "history" => parse_query(rest, true),
        "delete" => parse_store_action(rest, StoreAction::Delete),
        _ => Err(instance_usage(
            "unknown instance command or unexpected argument",
        )),
    }
}

pub(super) fn run(command: InstanceCommand) -> ExitCode {
    write_outcome(run_json(command))
}

fn run_json(command: InstanceCommand) -> CliOutcome {
    match command {
        InstanceCommand::Invalid(message) => instance_error_with_exit(
            LkError::new(ErrorCode::ProtocolMalformed, message),
            false,
            EXIT_USAGE_OR_JSON,
        ),
        InstanceCommand::Help => success(HELP.as_bytes().to_vec()),
        InstanceCommand::Create {
            store,
            application: artifact,
            pretty,
        } => {
            let request = match read_request::<InstanceCreateRequest>("instance create request") {
                Ok(request) => request,
                Err(error) => return instance_error(error, pretty),
            };
            let bytes = match application::read_file(&artifact) {
                Ok(bytes) => bytes,
                Err(error) => return instance_error(error, pretty),
            };
            with_store(&store, pretty, |store| store.create(&request, &bytes))
        }
        InstanceCommand::ValidateEvent { store, pretty } => {
            let request = match read_request::<InstanceEventRequest>("instance event request") {
                Ok(request) => request,
                Err(error) => return instance_error(error, pretty),
            };
            with_store(&store, pretty, |store| store.validate_event(&request))
        }
        InstanceCommand::ApplyEvent { store, pretty } => {
            let request = match read_request::<InstanceEventRequest>("instance event request") {
                Ok(request) => request,
                Err(error) => return instance_error(error, pretty),
            };
            with_store(&store, pretty, |store| store.apply_event(&request))
        }
        InstanceCommand::ValidateApplication { store, pretty } => {
            let request = match read_request::<InstanceHostRequest>("instance host request") {
                Ok(request) => request,
                Err(error) => return instance_error(error, pretty),
            };
            with_store(&store, pretty, |store| store.validate_application(&request))
        }
        InstanceCommand::ExecuteActivation { store, pretty } => {
            let request = match read_request::<InstanceHostRequest>("instance host request") {
                Ok(request) => request,
                Err(error) => return instance_error(error, pretty),
            };
            with_store(&store, pretty, |store| store.execute_activation(&request))
        }
        InstanceCommand::ReconcileActivation { store, pretty } => {
            let request = match read_request::<InstanceHostRequest>("instance host request") {
                Ok(request) => request,
                Err(error) => return instance_error(error, pretty),
            };
            with_store(&store, pretty, |store| store.reconcile_activation(&request))
        }
        InstanceCommand::FakeOutcome { store, pretty } => {
            let request =
                match read_request::<InstanceFakeHostRequest>("instance fake host request") {
                    Ok(request) => request,
                    Err(error) => return instance_error(error, pretty),
                };
            with_store(&store, pretty, |store| store.record_fake_outcome(&request))
        }
        InstanceCommand::ValidateResume { store, pretty } => {
            let request = match read_request::<InstanceResumeRequest>("instance resume request") {
                Ok(request) => request,
                Err(error) => return instance_error(error, pretty),
            };
            with_store(&store, pretty, |store| store.validate_resume(&request))
        }
        InstanceCommand::Resume { store, pretty } => {
            let request = match read_request::<InstanceResumeRequest>("instance resume request") {
                Ok(request) => request,
                Err(error) => return instance_error(error, pretty),
            };
            with_store(&store, pretty, |store| store.resume(&request))
        }
        InstanceCommand::Inspect {
            store,
            instance,
            pretty,
        } => with_store(&store, pretty, |store| store.inspect(instance)),
        InstanceCommand::History {
            store,
            instance,
            start,
            limit,
            pretty,
        } => with_store(&store, pretty, |store| {
            store.history(instance, start, limit)
        }),
        InstanceCommand::Delete { store, pretty } => {
            let request = match read_request::<InstanceDeleteRequest>("instance delete request") {
                Ok(request) => request,
                Err(error) => return instance_error(error, pretty),
            };
            with_store(&store, pretty, |store| store.delete(request))
        }
    }
}

fn with_store<T: Serialize>(
    path: &Path,
    pretty: bool,
    operation: impl FnOnce(&InstanceStore) -> lkjscript::Result<T>,
) -> CliOutcome {
    let store = match InstanceStore::open(path) {
        Ok(store) => store,
        Err(error) => return instance_error(error, pretty),
    };
    match operation(&store) {
        Ok(value) => encode_json(&value, pretty),
        Err(error) => instance_error(error, pretty),
    }
}

fn read_request<T: serde::de::DeserializeOwned>(label: &str) -> lkjscript::Result<T> {
    let bytes = read_stdin_bounded().map_err(|message| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            format!("cannot read {label}: {message}"),
        )
    })?;
    strict_json(&bytes, label)
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
            "instance response exceeds JSON output policy".into(),
            None,
        ),
        Err(error) => failure(
            EXIT_OUTPUT,
            BoundaryErrorKind::Output,
            format!("cannot encode instance response: {error}"),
            None,
        ),
    }
}

fn instance_error(error: LkError, pretty: bool) -> CliOutcome {
    let exit = match error.code {
        ErrorCode::ArtifactCorrupt | ErrorCode::ArtifactPublicationOutcomeUnknown => EXIT_ARTIFACT,
        ErrorCode::PolicyExceeded
        | ErrorCode::ExecutionFuelExhausted
        | ErrorCode::ExecutionFrameExhausted
        | ErrorCode::ExecutionMemoryExhausted => EXIT_RESOURCE,
        ErrorCode::Io | ErrorCode::AuthorityBusy | ErrorCode::CommitOutcomeUnknown => {
            EXIT_TRANSPORT
        }
        _ => EXIT_APPLICATION_INPUT,
    };
    instance_error_with_exit(error, pretty, exit)
}

fn instance_error_with_exit(error: LkError, pretty: bool, exit: u8) -> CliOutcome {
    #[derive(Serialize)]
    #[serde(deny_unknown_fields)]
    struct Envelope<'a> {
        contract_version: u16,
        error: &'a LkError,
    }
    let diagnostic = error.to_string();
    let mut outcome = encode_json(
        &Envelope {
            contract_version: INSTANCE_CONTRACT_VERSION,
            error: &error,
        },
        pretty,
    );
    outcome.exit = exit;
    outcome.diagnostic = Some(diagnostic);
    outcome
}

fn parse_create(arguments: &[String]) -> Result<InstanceCommand, String> {
    let mut store = None;
    let mut application = None;
    let mut pretty = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--store" if store.is_none() => {
                store = Some(PathBuf::from(value_after(
                    arguments, &mut index, "--store",
                )?))
            }
            "--application" if application.is_none() => {
                application = Some(PathBuf::from(value_after(
                    arguments,
                    &mut index,
                    "--application",
                )?))
            }
            "--pretty" if !pretty => pretty = true,
            _ => return Err(instance_usage("invalid or duplicate create option")),
        }
        index += 1;
    }
    Ok(InstanceCommand::Create {
        store: store.ok_or_else(|| instance_usage("create requires --store"))?,
        application: application.ok_or_else(|| instance_usage("create requires --application"))?,
        pretty,
    })
}

#[derive(Clone, Copy)]
enum StoreAction {
    ValidateEvent,
    ApplyEvent,
    ValidateApplication,
    ExecuteActivation,
    ReconcileActivation,
    FakeOutcome,
    ValidateResume,
    Resume,
    Delete,
}

fn parse_store_action(
    arguments: &[String],
    action: StoreAction,
) -> Result<InstanceCommand, String> {
    let (store, pretty) = parse_store_pretty(arguments)?;
    Ok(match action {
        StoreAction::ValidateEvent => InstanceCommand::ValidateEvent { store, pretty },
        StoreAction::ApplyEvent => InstanceCommand::ApplyEvent { store, pretty },
        StoreAction::ValidateApplication => InstanceCommand::ValidateApplication { store, pretty },
        StoreAction::ExecuteActivation => InstanceCommand::ExecuteActivation { store, pretty },
        StoreAction::ReconcileActivation => InstanceCommand::ReconcileActivation { store, pretty },
        StoreAction::FakeOutcome => InstanceCommand::FakeOutcome { store, pretty },
        StoreAction::ValidateResume => InstanceCommand::ValidateResume { store, pretty },
        StoreAction::Resume => InstanceCommand::Resume { store, pretty },
        StoreAction::Delete => InstanceCommand::Delete { store, pretty },
    })
}

fn parse_store_pretty(arguments: &[String]) -> Result<(PathBuf, bool), String> {
    let mut store = None;
    let mut pretty = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--store" if store.is_none() => {
                store = Some(PathBuf::from(value_after(
                    arguments, &mut index, "--store",
                )?))
            }
            "--pretty" if !pretty => pretty = true,
            _ => return Err(instance_usage("invalid or duplicate instance option")),
        }
        index += 1;
    }
    Ok((
        store.ok_or_else(|| instance_usage("instance command requires --store"))?,
        pretty,
    ))
}

fn parse_query(arguments: &[String], history: bool) -> Result<InstanceCommand, String> {
    let mut store = None;
    let mut instance = None;
    let mut start = 0_u64;
    let mut limit = 64_usize;
    let mut start_seen = false;
    let mut limit_seen = false;
    let mut pretty = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--store" if store.is_none() => {
                store = Some(PathBuf::from(value_after(
                    arguments, &mut index, "--store",
                )?))
            }
            "--instance" if instance.is_none() => {
                instance = Some(
                    value_after(arguments, &mut index, "--instance")?
                        .parse::<InstanceId>()
                        .map_err(instance_usage)?,
                );
            }
            "--start" if history && !start_seen => {
                start_seen = true;
                start = value_after(arguments, &mut index, "--start")?
                    .parse()
                    .map_err(|_| instance_usage("invalid history start"))?;
            }
            "--limit" if history && !limit_seen => {
                limit_seen = true;
                limit = value_after(arguments, &mut index, "--limit")?
                    .parse()
                    .map_err(|_| instance_usage("invalid history limit"))?;
            }
            "--pretty" if !pretty => pretty = true,
            _ => return Err(instance_usage("invalid or duplicate query option")),
        }
        index += 1;
    }
    let store = store.ok_or_else(|| instance_usage("query requires --store"))?;
    let instance = instance.ok_or_else(|| instance_usage("query requires --instance"))?;
    Ok(if history {
        InstanceCommand::History {
            store,
            instance,
            start,
            limit,
            pretty,
        }
    } else {
        InstanceCommand::Inspect {
            store,
            instance,
            pretty,
        }
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
        .ok_or_else(|| instance_usage(&format!("{flag} requires a value")))
}

fn instance_usage(reason: &str) -> String {
    usage(&format!(
        "{reason}; use `lkjscript instance help` for the durable instance contract"
    ))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn help_is_default_and_query_options_are_closed() {
        assert!(matches!(
            parse(Vec::<String>::new().into_iter()).expect("default"),
            InstanceCommand::Help
        ));
        assert!(
            parse(
                [
                    "inspect".to_owned(),
                    "--store".to_owned(),
                    "/tmp/s".to_owned()
                ]
                .into_iter()
            )
            .is_err()
        );
        assert!(
            parse(
                [
                    "apply-event".to_owned(),
                    "--store".to_owned(),
                    "/tmp/s".to_owned(),
                    "--start".to_owned(),
                    "1".to_owned()
                ]
                .into_iter()
            )
            .is_err()
        );
    }
}
