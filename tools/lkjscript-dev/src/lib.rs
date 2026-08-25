#![cfg_attr(test, allow(clippy::expect_used, clippy::panic, clippy::unwrap_used))]
#![forbid(unsafe_code)]

mod check;
mod error;
mod evidence;
mod process;
mod scale;

use error::DevError;
use std::ffi::OsString;
use std::io::{self, Write};
use std::process::ExitCode;

pub fn entry(arguments: impl IntoIterator<Item = OsString>) -> ExitCode {
    match run(arguments) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            let _ = writeln!(io::stderr(), "lkjscript-dev: {error}");
            ExitCode::from(2)
        }
    }
}

fn run(arguments: impl IntoIterator<Item = OsString>) -> Result<u8, DevError> {
    let mut arguments = arguments.into_iter();
    let _program = arguments.next();
    let command = next_utf8(&mut arguments, "command")?;
    match command.as_deref() {
        Some("check") => check::command(arguments),
        Some("scale") => scale::command(arguments),
        Some("__fixture") => check::fixture(arguments),
        Some("help") | Some("--help") | Some("-h") | None => {
            println!(
                "usage: lkjscript-dev check <focused|changed|product|service|full|self-test> ... | lkjscript-dev scale <independent-modules|small-functions|wide-module|deep-chain|wide-fanout> ..."
            );
            Ok(0)
        }
        Some(value) => Err(DevError::usage(format!("unknown command '{value}'"))),
    }
}

pub(crate) fn next_utf8(
    arguments: &mut impl Iterator<Item = OsString>,
    label: &str,
) -> Result<Option<String>, DevError> {
    arguments
        .next()
        .map(|value| {
            value
                .into_string()
                .map_err(|_| DevError::usage(format!("{label} must be valid UTF-8")))
        })
        .transpose()
}
