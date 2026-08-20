// The public typed error deliberately carries bounded related authority facts.
#![allow(clippy::result_large_err)]

use lkjscript::application::{APPLICATION_CONTRACT_VERSION, MAXIMUM_APPLICATION_ARTIFACT_BYTES};
use lkjscript::error::{ErrorCode, LkError, Result};
use lkjscript::interactive_runner::{decode_headless_replay, run_headless_replay};
use lkjscript::terminal::run_terminal_with_actions;
use lkjscript::workbench_host::WorkbenchHost;
use serde::Serialize;
use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const PRODUCT_CONTRACT_VERSION: u16 = 1;
const EXIT_USAGE: u8 = 2;
const EXIT_INFRASTRUCTURE: u8 = 3;
const EXIT_DOMAIN: u8 = 5;
const EXIT_RESOURCE: u8 = 8;
const EMBEDDED_APPLICATION: &[u8] = include_bytes!("../../applications/lkjstudio/lkjstudio.lkja");

const HELP: &str = "lkjstudio — semantic application terminal runner

Usage:
  lkjstudio [run] [--artifact FILE] [--project PROJECT] [--root DIRECTORY]
  lkjstudio headless [--artifact FILE] [--pretty]  # replay JSON on stdin
  lkjstudio version [--artifact FILE] [--pretty]
  lkjstudio help

Without --artifact, the checked lkjstudio application is used. PROJECT is discovered from the
current directory when omitted. --root grants bounded selected-file access; no filesystem grant is
installed when it is omitted. The native executable owns deployment selection, terminal mechanics,
and exact host adaptation only; application meaning owns state, commands, key policy, updates, and
frames.";

#[derive(Clone, Debug)]
enum Command {
    Run {
        artifact: Option<PathBuf>,
        project: Option<PathBuf>,
        root: Option<PathBuf>,
    },
    Headless {
        artifact: Option<PathBuf>,
        pretty: bool,
    },
    Version {
        artifact: Option<PathBuf>,
        pretty: bool,
    },
    Help,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct Success<T> {
    version: u16,
    result: T,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct Failure<'a> {
    version: u16,
    error: &'a LkError,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct VersionReceipt {
    product_contract_version: u16,
    application_contract_version: u16,
    application: lkjscript::application::ApplicationInspection,
    embedded: bool,
}

fn main() -> ExitCode {
    let command = match parse(std::env::args().skip(1).collect()) {
        Ok(command) => command,
        Err(message) => {
            write_diagnostic(format_args!("{message}"));
            return ExitCode::from(EXIT_USAGE);
        }
    };
    match command {
        Command::Help => {
            let stdout = std::io::stdout();
            let mut writer = stdout.lock();
            if writer.write_all(HELP.as_bytes()).is_err()
                || writer.write_all(b"\n").is_err()
                || writer.flush().is_err()
            {
                write_diagnostic(format_args!("lkjstudio output failed"));
                ExitCode::from(EXIT_INFRASTRUCTURE)
            } else {
                ExitCode::SUCCESS
            }
        }
        Command::Run {
            artifact,
            project,
            root,
        } => {
            let application = match load_application(artifact.as_deref()) {
                Ok(application) => application,
                Err(error) => return write_error(&error, false),
            };
            let mut host = match WorkbenchHost::open(project.as_deref(), root.as_deref()) {
                Ok(host) => host,
                Err(error) => return write_error(&error, false),
            };
            match run_terminal_with_actions(&application, |action| host.handle(action)) {
                Ok(receipt) => write_success(&receipt, false),
                Err(error) => write_error(&error, false),
            }
        }
        Command::Headless { artifact, pretty } => {
            let result = run_headless(artifact.as_deref());
            match result {
                Ok(receipt) => write_success(&receipt, pretty),
                Err(error) => write_error(&error, pretty),
            }
        }
        Command::Version { artifact, pretty } => {
            let result = version(artifact.as_deref());
            match result {
                Ok(receipt) => write_success(&receipt, pretty),
                Err(error) => write_error(&error, pretty),
            }
        }
    }
}

fn parse(arguments: Vec<String>) -> std::result::Result<Command, String> {
    if arguments.is_empty() {
        return Ok(Command::Run {
            artifact: None,
            project: None,
            root: None,
        });
    }
    if arguments.as_slice() == ["help"] || arguments.as_slice() == ["--help"] {
        return Ok(Command::Help);
    }
    let (name, options) = match arguments[0].as_str() {
        "run" | "headless" | "version" => (arguments[0].as_str(), &arguments[1..]),
        option if option.starts_with('-') => ("run", arguments.as_slice()),
        _ => return Err(format!("unknown command\n\n{HELP}")),
    };
    let mut artifact = None;
    let mut project = None;
    let mut root = None;
    let mut pretty = false;
    let mut offset = 0_usize;
    while offset < options.len() {
        match options[offset].as_str() {
            "--artifact" if artifact.is_none() && offset + 1 < options.len() => {
                artifact = Some(PathBuf::from(&options[offset + 1]));
                offset += 2;
            }
            "--project" if name == "run" && project.is_none() && offset + 1 < options.len() => {
                project = Some(PathBuf::from(&options[offset + 1]));
                offset += 2;
            }
            "--root" if name == "run" && root.is_none() && offset + 1 < options.len() => {
                root = Some(PathBuf::from(&options[offset + 1]));
                offset += 2;
            }
            "--pretty" if name != "run" && !pretty => {
                pretty = true;
                offset += 1;
            }
            _ => {
                return Err(format!(
                    "unknown, duplicate, or incomplete option\n\n{HELP}"
                ));
            }
        }
    }
    Ok(match name {
        "run" => Command::Run {
            artifact,
            project,
            root,
        },
        "headless" => Command::Headless { artifact, pretty },
        "version" => Command::Version { artifact, pretty },
        _ => return Err(format!("unknown command\n\n{HELP}")),
    })
}

fn load_application(path: Option<&Path>) -> Result<Vec<u8>> {
    let Some(path) = path else {
        return Ok(EMBEDDED_APPLICATION.to_vec());
    };
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        LkError::new(
            ErrorCode::Io,
            format!(
                "cannot inspect application artifact {}: {error}",
                path.display()
            ),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "application artifact must be an explicitly selected regular non-symlink file",
        ));
    }
    if metadata.len() > MAXIMUM_APPLICATION_ARTIFACT_BYTES as u64 {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application artifact exceeds runner byte policy",
        ));
    }
    fs::read(path).map_err(|error| {
        LkError::new(
            ErrorCode::Io,
            format!(
                "cannot read application artifact {}: {error}",
                path.display()
            ),
        )
    })
}

fn run_headless(
    artifact: Option<&Path>,
) -> Result<lkjscript::interactive_runner::HeadlessReplayReceipt> {
    let application = load_application(artifact)?;
    let mut input = Vec::new();
    std::io::stdin()
        .lock()
        .take((lkjscript::interactive_runner::MAXIMUM_HEADLESS_INPUT_BYTES as u64) + 1)
        .read_to_end(&mut input)
        .map_err(|error| {
            LkError::new(
                ErrorCode::Io,
                format!("cannot read headless replay input: {error}"),
            )
        })?;
    let request = decode_headless_replay(&input)?;
    run_headless_replay(&application, &request)
}

fn version(artifact: Option<&Path>) -> Result<VersionReceipt> {
    let application = load_application(artifact)?;
    Ok(VersionReceipt {
        product_contract_version: PRODUCT_CONTRACT_VERSION,
        application_contract_version: APPLICATION_CONTRACT_VERSION,
        application: lkjscript::application::inspect(&application)?,
        embedded: artifact.is_none(),
    })
}

fn write_success<T: Serialize>(result: &T, pretty: bool) -> ExitCode {
    let envelope = Success {
        version: PRODUCT_CONTRACT_VERSION,
        result,
    };
    write_json(&envelope, pretty, ExitCode::SUCCESS)
}

fn write_error(error: &LkError, pretty: bool) -> ExitCode {
    write_diagnostic(format_args!(
        "{}: {}",
        error.code.machine_name(),
        error.message
    ));
    let exit = match error.code {
        ErrorCode::PolicyExceeded => EXIT_RESOURCE,
        ErrorCode::ProtocolMalformed | ErrorCode::ProtocolVersion => EXIT_USAGE,
        ErrorCode::TerminalUnavailable
        | ErrorCode::TerminalDecode
        | ErrorCode::TerminalOutput
        | ErrorCode::TerminalCleanup
        | ErrorCode::Io => EXIT_INFRASTRUCTURE,
        _ => EXIT_DOMAIN,
    };
    write_json(
        &Failure {
            version: PRODUCT_CONTRACT_VERSION,
            error,
        },
        pretty,
        ExitCode::from(exit),
    )
}

fn write_json(value: &impl Serialize, pretty: bool, exit: ExitCode) -> ExitCode {
    let bytes = if pretty {
        serde_json::to_vec_pretty(value)
    } else {
        serde_json::to_vec(value)
    };
    let Ok(bytes) = bytes else {
        write_diagnostic(format_args!("lkjstudio output encoding failed"));
        return ExitCode::from(EXIT_INFRASTRUCTURE);
    };
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    if writer.write_all(&bytes).is_err()
        || writer.write_all(b"\n").is_err()
        || writer.flush().is_err()
    {
        write_diagnostic(format_args!("lkjstudio output failed"));
        return ExitCode::from(EXIT_INFRASTRUCTURE);
    }
    exit
}

fn write_diagnostic(arguments: fmt::Arguments<'_>) {
    let stderr = std::io::stderr();
    let mut writer = stderr.lock();
    let _ = writer.write_fmt(arguments);
    let _ = writer.write_all(b"\n");
    let _ = writer.flush();
}
