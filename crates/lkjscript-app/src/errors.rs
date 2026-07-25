use std::process::ExitCode;

pub fn report(error: String) -> ExitCode {
    eprintln!("lkjscript: {error}");
    ExitCode::from(1)
}
