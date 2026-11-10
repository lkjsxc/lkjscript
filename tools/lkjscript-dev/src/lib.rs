#![cfg_attr(test, allow(clippy::expect_used, clippy::panic, clippy::unwrap_used))]
#![forbid(unsafe_code)]

mod authority;
mod compact_request;
mod data_oracle;
mod distributed_http;
mod error;
mod evidence;
mod extraction_oracle;
mod http_probe;
mod measure;
mod offline_packages;
mod outbound_http;
mod postgres;
mod process;
mod raw_websocket;
mod scale;
mod semantic_cli;
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
        Some("data-oracle") => data_oracle::command(arguments),
        Some("distributed-http") => distributed_http::command(arguments),
        Some("function-extraction-oracle") => extraction_oracle::command(arguments),
        Some("scale") => scale::command(arguments),
        Some("measure") => measure::command(arguments),
        Some("outbound-http") => outbound_http::command(arguments),
        Some("offline-packages") => offline_packages::command(arguments),
        Some("recursive-probe") => offline_packages::recursive_probe_command(arguments),
        Some("effect-probe") => offline_packages::effect_probe_command(arguments),
        Some("effect-transaction-probe") => {
            offline_packages::effect_transaction_probe_command(arguments)
        }
        Some("recursive-transaction-probe") => {
            offline_packages::recursive_transaction_probe_command(arguments)
        }
        Some("nominal-session-state-probe") => service::nominal::state_probe_command(arguments),
        Some("service") => service::command(arguments),
        Some("stateful-http") => stateful_http::command(arguments),
        Some("help") | Some("--help") | Some("-h") | None => {
            println!(
                "usage: lkjscript-dev <data-oracle|distributed-http|function-extraction-oracle|scale|measure|outbound-http|offline-packages|service|stateful-http> ..."
            );
            println!("development checks: bash tools/check.sh [focused FILTER|slow FILTER]");
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
