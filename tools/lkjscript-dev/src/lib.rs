#![cfg_attr(test, allow(clippy::expect_used, clippy::panic, clippy::unwrap_used))]
#![forbid(unsafe_code)]

mod authority;
mod check;
mod distributed_http;
mod error;
mod evidence;
mod http_probe;
mod measure;
mod postgres;
mod process;
mod release;
mod scale;
mod service;
mod stateful_http;
mod stateful_http_program;

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
        Some("distributed-http") => distributed_http::command(arguments),
        Some("policy") => check::policy_command(arguments),
        Some("scale") => scale::command(arguments),
        Some("measure") => measure::command(arguments),
        Some("release") => release::command(arguments),
        Some("service") => service::command(arguments),
        Some("stateful-http") => stateful_http::command(arguments),
        Some("__fixture") => check::fixture(arguments),
        Some("help") | Some("--help") | Some("-h") | None => {
            println!(
                "usage: lkjscript-dev check <focused|changed|product|service|full|self-test> ... | lkjscript-dev distributed-http [--binary PATH] [--evidence-root ABSENT_ABSOLUTE_PATH] [--machine] | lkjscript-dev stateful-http [--binary PATH] [--postgres-root PATH] [--machine] | lkjscript-dev policy no-python [--machine] | lkjscript-dev measure --cwd PATH --output DIR -- COMMAND [ARG ...] | lkjscript-dev release <prepare|verify> ... | lkjscript-dev scale <independent-modules|small-functions|wide-module|deep-chain|wide-fanout> ... | lkjscript-dev service [--binary PATH] [--postgres-root PATH] [--machine]"
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
