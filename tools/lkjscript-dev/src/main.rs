#![forbid(unsafe_code)]

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    match lkjscript::platform::stack::run("lkjscript-dev", || {
        lkjscript_dev::entry(std::env::args_os())
    }) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "lkjscript-dev: semantic stack: {error}");
            ExitCode::from(2)
        }
    }
}
