#![cfg_attr(test, allow(clippy::expect_used, clippy::panic, clippy::unwrap_used))]
#![forbid(unsafe_code)]

mod authority;
mod check;
mod data_oracle;
mod distributed_http;
mod error;
mod evidence;
mod extraction_oracle;
mod http_probe;
mod measure;
mod outbound_http;
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
        Some("data-oracle") => data_oracle::command(arguments),
        Some("distributed-http") => distributed_http::command(arguments),
        Some("function-extraction-oracle") => extraction_oracle::command(arguments),
        Some("policy") => check::policy_command(arguments),
        Some("scale") => scale::command(arguments),
        Some("measure") => measure::command(arguments),
        Some("outbound-http") => outbound_http::command(arguments),
        Some("release") => release::command(arguments),
        Some("service") => service::command(arguments),
        Some("stateful-http") => stateful_http::command(arguments),
        Some("__fixture") => check::fixture(arguments),
        Some("help") | Some("--help") | Some("-h") | None => {
            println!(
                "usage: lkjscript-dev check <focused|changed|product|service|full|self-test> ... | lkjscript-dev data-oracle --binary PATH --bbs-receipt PATH --service-receipt PATH [--machine] | lkjscript-dev distributed-http [--binary PATH] [--evidence-root ABSENT_ABSOLUTE_PATH] [--machine] | lkjscript-dev function-extraction-oracle --project PATH --function DECL --expression EXPR [--output ABSENT_PATH] | lkjscript-dev outbound-http [--binary PATH] [--evidence-root ABSENT_ABSOLUTE_PATH] [--machine] | lkjscript-dev stateful-http [--binary PATH] [--evidence-root ABSENT_ABSOLUTE_PATH] [--machine] | lkjscript-dev policy <no-python|product-surface> [--binary PATH] [--machine] | lkjscript-dev measure --cwd PATH --output DIR -- COMMAND [ARG ...] | lkjscript-dev release <target|build|admit|verifier|prepare|verify> ... | lkjscript-dev scale <independent-modules|small-functions|wide-module|deep-chain|wide-fanout> ... | lkjscript-dev service [--binary PATH] [--machine]"
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
