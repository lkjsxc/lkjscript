use std::io::{Read, Write};
use std::process::ExitCode;

pub fn command(args: &[String]) -> Result<ExitCode, String> {
    if args.len() > 2 || args.get(1).is_some_and(|argument| argument != "-") {
        return Err("semantic accepts only stdin; use - or input redirection".to_string());
    }
    let limit = lkjscript_compiler::semantic::MAX_REQUEST_BYTES;
    let mut input = Vec::new();
    std::io::stdin()
        .lock()
        .take((limit + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|failure| format!("read semantic request from stdin: {failure}"))?;
    if input.len() > limit {
        return Err(format!("semantic request exceeds {limit} bytes"));
    }
    let output =
        lkjscript_compiler::semantic::execute(&input).map_err(|failure| failure.to_string())?;
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(&output)
        .and_then(|()| stdout.flush())
        .map_err(|failure| format!("write semantic protocol response: {failure}"))?;
    Ok(ExitCode::SUCCESS)
}
