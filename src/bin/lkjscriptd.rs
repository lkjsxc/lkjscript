use lkjscript::daemon;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match arguments() {
        Ok(state) => match daemon::run_foreground(&state) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn arguments() -> Result<PathBuf, String> {
    let mut arguments = std::env::args_os();
    let _program = arguments.next();
    let state_flag = arguments.next().ok_or_else(|| usage("missing --state"))?;
    if state_flag != "--state" {
        return Err(usage("expected --state"));
    }
    let state = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| usage("missing state directory"))?;
    let foreground = arguments
        .next()
        .ok_or_else(|| usage("missing --foreground"))?;
    if foreground != "--foreground" || arguments.next().is_some() {
        return Err(usage(
            "expected only --foreground after the state directory",
        ));
    }
    Ok(state)
}

fn usage(reason: &str) -> String {
    format!("{reason}\nusage: lkjscriptd --state DIRECTORY --foreground")
}
