//! CLI entry for the lkjscript language runtime.

mod describe;
mod disasm;
mod engine;
mod errors;
mod execution_command;
mod help;
mod memory;
mod metrics;
mod metrics_json;
mod output;
mod package;

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => code,
        Err(error) => errors::report(error),
    }
}

fn real_main() -> Result<ExitCode, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version" | "-V") if args.len() == 1 => {
            println!("lkjscript {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
        None | Some("--help" | "-h") => {
            help::print();
            Ok(ExitCode::SUCCESS)
        }
        Some("describe") => describe::command(&args),
        Some("run") => execution_command::command(&args),
        Some("disasm") => disasm::command(&args),
        Some("package") => package::command(&args),
        Some("memory") => memory::command(&args),
        Some(other) => Err(format!("unknown command: {other}")),
    }
}
