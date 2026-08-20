// The public typed error deliberately carries bounded related authority facts.
#![allow(clippy::result_large_err)]

use lkjscript::application::{
    APPLICATION_CONTRACT_VERSION, InteractiveEvent, InteractiveOpenEvent,
    MAXIMUM_APPLICATION_ARTIFACT_BYTES,
};
use lkjscript::error::{ErrorCode, LkError, Result};
use lkjscript::interactive_runner::{decode_headless_replay, run_headless_replay};
use lkjscript::terminal::run_terminal_with_actions_and_initial_events;
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
const EMBEDDED_APPLICATION: &[u8] = include_bytes!("../../applications/lkjedit/lkjedit.lkja");

const HELP: &str = "lkjedit — Vim-like tiled semantic text editor

Usage:
  lkjedit [PATH]
  lkjedit --root ROOT [PATH]
  lkjedit --project PROJECT [PATH]
  lkjedit headless [--artifact FILE] [--pretty]  # deterministic replay JSON on stdin
  lkjedit version [--artifact FILE] [--pretty]
  lkjedit help

A directory PATH becomes the selected root and opens an explorer tab. A file PATH selects its
parent as root and opens or creates one editor buffer without publishing before :w. --root keeps
ROOT as the exact selected-filesystem authority and interprets PATH below it. --project adds one
explicit semantic-project grant; without PATH it opens semantic orientation. Normal launch uses the
checked embedded application. --artifact is an internal conformance override for headless/version
and is not part of ordinary launch. Native code owns deployment selection, terminal mechanics, and
typed host adaptation only; lkjscript application meaning owns editor and layout policy.";

#[derive(Clone, Debug)]
enum Command {
    Run {
        project: Option<PathBuf>,
        root: Option<PathBuf>,
        path: Option<PathBuf>,
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
                write_diagnostic(format_args!("lkjedit output failed"));
                ExitCode::from(EXIT_INFRASTRUCTURE)
            } else {
                ExitCode::SUCCESS
            }
        }
        Command::Run {
            project,
            root,
            path,
        } => {
            let application = match load_application(None) {
                Ok(application) => application,
                Err(error) => return write_error(&error, false),
            };
            let selection =
                match resolve_launch(path.as_deref(), root.as_deref(), project.is_some()) {
                    Ok(selection) => selection,
                    Err(error) => return write_error(&error, false),
                };
            let mut host =
                match WorkbenchHost::open(project.as_deref(), selection.filesystem_root.as_deref())
                {
                    Ok(host) => host,
                    Err(error) => return write_error(&error, false),
                };
            match run_terminal_with_actions_and_initial_events(
                &application,
                vec![InteractiveEvent::Open(selection.open)],
                move |action| host.handle(action),
            ) {
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
            project: None,
            root: None,
            path: None,
        });
    }
    if arguments.as_slice() == ["help"] || arguments.as_slice() == ["--help"] {
        return Ok(Command::Help);
    }
    let (name, options) = match arguments[0].as_str() {
        "run" | "headless" | "version" => (arguments[0].as_str(), &arguments[1..]),
        _ => ("run", arguments.as_slice()),
    };
    let mut artifact = None;
    let mut project = None;
    let mut root = None;
    let mut path = None;
    let mut pretty = false;
    let mut offset = 0_usize;
    while offset < options.len() {
        match options[offset].as_str() {
            "--artifact" if name != "run" && artifact.is_none() && offset + 1 < options.len() => {
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
            value if name == "run" && !value.starts_with('-') && path.is_none() => {
                path = Some(PathBuf::from(value));
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
            project,
            root,
            path,
        },
        "headless" => Command::Headless { artifact, pretty },
        "version" => Command::Version { artifact, pretty },
        _ => return Err(format!("unknown command\n\n{HELP}")),
    })
}

struct LaunchSelection {
    filesystem_root: Option<PathBuf>,
    open: InteractiveOpenEvent,
}

fn resolve_launch(
    path: Option<&Path>,
    explicit_root: Option<&Path>,
    project_selected: bool,
) -> Result<LaunchSelection> {
    if let Some(root) = explicit_root {
        require_directory(root, "selected root")?;
        let (relative, directory) = match path {
            Some(path) => {
                let relative = relative_below_root(root, path)?;
                let candidate = root.join(&relative);
                (
                    relative_path_text(&relative)?,
                    path_is_directory_or_absent(&candidate)?,
                )
            }
            None => (String::new(), true),
        };
        return Ok(LaunchSelection {
            filesystem_root: Some(root.to_owned()),
            open: InteractiveOpenEvent {
                path: relative,
                directory,
                project: false,
            },
        });
    }

    let Some(path) = path else {
        if project_selected {
            return Ok(LaunchSelection {
                filesystem_root: None,
                open: InteractiveOpenEvent {
                    path: String::new(),
                    directory: false,
                    project: true,
                },
            });
        }
        let current = std::env::current_dir().map_err(|error| {
            LkError::new(
                ErrorCode::Io,
                format!("cannot inspect current directory: {error}"),
            )
        })?;
        require_directory(&current, "current directory")?;
        return Ok(LaunchSelection {
            filesystem_root: Some(current),
            open: InteractiveOpenEvent {
                path: String::new(),
                directory: true,
                project: false,
            },
        });
    };

    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "startup path must not be a symbolic link",
        )),
        Ok(metadata) if metadata.is_dir() => Ok(LaunchSelection {
            filesystem_root: Some(path.to_owned()),
            open: InteractiveOpenEvent {
                path: String::new(),
                directory: true,
                project: false,
            },
        }),
        Ok(metadata) if metadata.is_file() => file_launch(path),
        Ok(_) => Err(LkError::new(
            ErrorCode::FilesystemWrongType,
            "startup path is neither a directory nor a regular file",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => file_launch(path),
        Err(error) => Err(LkError::new(
            ErrorCode::Io,
            format!("cannot inspect startup path {}: {error}", path.display()),
        )),
    }
}

fn file_launch(path: &Path) -> Result<LaunchSelection> {
    let parent = path.parent().filter(|value| !value.as_os_str().is_empty());
    let parent = parent.unwrap_or_else(|| Path::new("."));
    require_directory(parent, "startup file parent")?;
    let name = path.file_name().ok_or_else(|| {
        LkError::new(
            ErrorCode::FilesystemDenied,
            "startup file path has no final component",
        )
    })?;
    let name = name.to_str().ok_or_else(|| {
        LkError::new(
            ErrorCode::FilesystemDenied,
            "startup file path is not valid UTF-8",
        )
    })?;
    if name.is_empty() || name == "." || name == ".." || name.contains('/') {
        return Err(LkError::new(
            ErrorCode::FilesystemDenied,
            "startup file name is not one safe relative component",
        ));
    }
    Ok(LaunchSelection {
        filesystem_root: Some(parent.to_owned()),
        open: InteractiveOpenEvent {
            path: name.to_owned(),
            directory: false,
            project: false,
        },
    })
}

fn require_directory(path: &Path, label: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        LkError::new(
            ErrorCode::Io,
            format!("cannot inspect {label} {}: {error}", path.display()),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LkError::new(
            ErrorCode::CapabilityDenied,
            format!("{label} must be a regular non-symlink directory"),
        ));
    }
    Ok(())
}

fn relative_below_root(root: &Path, path: &Path) -> Result<PathBuf> {
    let relative = if path.is_absolute() {
        let root = root.canonicalize().map_err(|error| {
            LkError::new(
                ErrorCode::Io,
                format!("cannot resolve selected root {}: {error}", root.display()),
            )
        })?;
        path.strip_prefix(&root).map_err(|_| {
            LkError::new(
                ErrorCode::FilesystemDenied,
                "absolute startup path is outside the selected root",
            )
        })?
    } else {
        path
    };
    for component in relative.components() {
        match component {
            std::path::Component::Normal(_) => {}
            std::path::Component::CurDir => {}
            _ => {
                return Err(LkError::new(
                    ErrorCode::FilesystemDenied,
                    "startup path below --root must not traverse or replace authority",
                ));
            }
        }
    }
    Ok(relative.to_owned())
}

fn relative_path_text(path: &Path) -> Result<String> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(value) => {
                components.push(value.to_str().ok_or_else(|| {
                    LkError::new(
                        ErrorCode::FilesystemDenied,
                        "startup path below --root is not valid UTF-8",
                    )
                })?)
            }
            std::path::Component::CurDir => {}
            _ => {
                return Err(LkError::new(
                    ErrorCode::FilesystemDenied,
                    "startup path below --root must be relative",
                ));
            }
        }
    }
    Ok(components.join("/"))
}

fn path_is_directory_or_absent(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "startup path below --root must not be a symbolic link",
        )),
        Ok(metadata) if metadata.is_dir() => Ok(true),
        Ok(metadata) if metadata.is_file() => Ok(false),
        Ok(_) => Err(LkError::new(
            ErrorCode::FilesystemWrongType,
            "startup path below --root is neither a directory nor a regular file",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(LkError::new(
            ErrorCode::Io,
            format!("cannot inspect startup path {}: {error}", path.display()),
        )),
    }
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
        write_diagnostic(format_args!("lkjedit output encoding failed"));
        return ExitCode::from(EXIT_INFRASTRUCTURE);
    };
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    if writer.write_all(&bytes).is_err()
        || writer.write_all(b"\n").is_err()
        || writer.flush().is_err()
    {
        write_diagnostic(format_args!("lkjedit output failed"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_launch_rejects_the_diagnostic_artifact_override() {
        assert!(parse(vec!["--artifact".into(), "alternate.lkja".into()]).is_err());
        assert!(
            parse(vec![
                "run".into(),
                "--artifact".into(),
                "alternate.lkja".into(),
            ])
            .is_err()
        );
        assert!(matches!(
            parse(vec![
                "headless".into(),
                "--artifact".into(),
                "alternate.lkja".into(),
            ]),
            Ok(Command::Headless {
                artifact: Some(_),
                pretty: false
            })
        ));
    }

    #[test]
    fn ordinary_launch_grammar_is_closed() {
        assert!(matches!(
            parse(vec!["--root".into(), "root".into(), "note.txt".into()]),
            Ok(Command::Run {
                root: Some(_),
                path: Some(_),
                ..
            })
        ));
        assert!(parse(vec!["--unknown".into()]).is_err());
        assert!(parse(vec!["one".into(), "two".into()]).is_err());
    }
}
