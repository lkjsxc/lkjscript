//! Read-only contributor command for the implementation-disjoint function-extraction oracle.

use crate::error::DevError;
use lkjscript::platform::contributor::function_extraction_oracle;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Default)]
struct Options {
    project: Option<PathBuf>,
    function: Option<String>,
    expression: Option<String>,
    output: Option<PathBuf>,
}

pub(crate) fn command(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let options = parse_options(arguments)?;
    let project = options
        .project
        .ok_or_else(|| DevError::usage("function-extraction-oracle requires --project PATH"))?;
    let function = options
        .function
        .ok_or_else(|| DevError::usage("function-extraction-oracle requires --function DECL"))?;
    let expression = options
        .expression
        .ok_or_else(|| DevError::usage("function-extraction-oracle requires --expression EXPR"))?;
    let observation =
        function_extraction_oracle(&project, &function, &expression).map_err(|diagnostic| {
            DevError::corrupt(format!(
                "{} [{:?}]: {}",
                diagnostic.code, diagnostic.class, diagnostic.message
            ))
        })?;
    match options.output {
        Some(path) => {
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&path)
                .map_err(|error| {
                    DevError::infrastructure(format!(
                        "create extraction oracle output '{}': {error}",
                        path.display()
                    ))
                })?;
            serde_json::to_writer_pretty(&mut output, &observation)?;
            output.write_all(b"\n")?;
            output.sync_all()?;
        }
        None => {
            let stdout = io::stdout();
            let mut output = stdout.lock();
            serde_json::to_writer_pretty(&mut output, &observation)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
    Ok(0)
}

fn parse_options(arguments: impl Iterator<Item = OsString>) -> Result<Options, DevError> {
    let values = arguments
        .map(|value| {
            value
                .into_string()
                .map_err(|_| DevError::usage("oracle arguments must be valid UTF-8"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut options = Options::default();
    let mut index = 0_usize;
    while index < values.len() {
        let option = &values[index];
        let value = values.get(index.saturating_add(1)).ok_or_else(|| {
            DevError::usage(format!(
                "function-extraction-oracle option '{option}' needs a value"
            ))
        })?;
        match option.as_str() {
            "--project" if options.project.is_none() => {
                options.project = Some(PathBuf::from(value));
            }
            "--function" if options.function.is_none() => {
                options.function = Some(value.clone());
            }
            "--expression" if options.expression.is_none() => {
                options.expression = Some(value.clone());
            }
            "--output" if options.output.is_none() => {
                options.output = Some(PathBuf::from(value));
            }
            "--project" | "--function" | "--expression" | "--output" => {
                return Err(DevError::usage(format!(
                    "function-extraction-oracle option '{option}' is duplicated"
                )));
            }
            _ => {
                return Err(DevError::usage(format!(
                    "unknown function-extraction-oracle option '{option}'"
                )));
            }
        }
        index = index.saturating_add(2);
    }
    Ok(options)
}
