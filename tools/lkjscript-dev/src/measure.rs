use crate::error::DevError;
use crate::evidence;
use crate::process::{self, ProcessSpec, ProcessStatus};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

const MAXIMUM_LOG_BYTES: u64 = 4 * 1_048_576;

pub(crate) fn command(arguments: impl IntoIterator<Item = OsString>) -> Result<u8, DevError> {
    let mut arguments = arguments.into_iter();
    let mut cwd = None;
    let mut output = None;
    let mut child = Vec::new();
    while let Some(argument) = crate::next_utf8(&mut arguments, "measure option")? {
        match argument.as_str() {
            "--cwd" if cwd.is_none() => {
                cwd = Some(PathBuf::from(
                    crate::next_utf8(&mut arguments, "measure cwd")?
                        .ok_or_else(|| DevError::usage("--cwd requires a path"))?,
                ));
            }
            "--output" if output.is_none() => {
                output = Some(PathBuf::from(
                    crate::next_utf8(&mut arguments, "measure output")?
                        .ok_or_else(|| DevError::usage("--output requires a path"))?,
                ));
            }
            "--" => {
                while let Some(value) = crate::next_utf8(&mut arguments, "child argument")? {
                    child.push(value);
                }
                break;
            }
            _ => {
                return Err(DevError::usage(format!(
                    "unknown or duplicate measure option '{argument}'"
                )));
            }
        }
    }
    let cwd = cwd.ok_or_else(|| DevError::usage("measure requires --cwd PATH"))?;
    let output = output.ok_or_else(|| DevError::usage("measure requires --output PATH"))?;
    if child.is_empty() {
        return Err(DevError::usage("measure requires -- COMMAND [ARG ...]"));
    }
    match std::fs::symlink_metadata(&output) {
        Ok(_) => {
            return Err(DevError::usage(format!(
                "measure output '{}' already exists",
                output.display()
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(DevError::infrastructure(format!(
                "inspect measure output '{}': {error}",
                output.display()
            )));
        }
    }
    std::fs::create_dir_all(&output).map_err(|error| {
        DevError::infrastructure(format!(
            "create measure output '{}': {error}",
            output.display()
        ))
    })?;
    let repository = std::env::current_dir()
        .map_err(|error| DevError::infrastructure(format!("read current directory: {error}")))?;
    let specification = ProcessSpec {
        command: child,
        cwd,
        environment: measurement_environment(),
        timeout: Duration::from_secs(300),
        maximum_stdout_bytes: MAXIMUM_LOG_BYTES,
        maximum_stderr_bytes: MAXIMUM_LOG_BYTES,
        stdout_path: output.join("stdout.log"),
        stderr_path: output.join("stderr.log"),
        unavailable_exit_code: None,
    };
    let observation = process::run(&specification, &repository);
    evidence::publish_json(&output.join("observation.json"), &observation)?;
    println!("{}", serde_json::to_string(&observation)?);
    Ok(if observation.status == ProcessStatus::Passed {
        0
    } else {
        1
    })
}

fn measurement_environment() -> BTreeMap<String, String> {
    let mut environment = BTreeMap::new();
    for name in ["HOME", "LANG", "LC_ALL", "PATH", "TMPDIR", "TZ"] {
        if let Ok(value) = std::env::var(name) {
            environment.insert(name.to_owned(), value);
        }
    }
    environment.insert("LANG".to_owned(), "C".to_owned());
    environment
}
