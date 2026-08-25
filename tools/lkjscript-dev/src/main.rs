#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    lkjscript_dev::entry(std::env::args_os())
}
