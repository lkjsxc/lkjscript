//! Issue a journal write token using the separately granted signing-key directory.
//! Stdin is an optional positive lifetime in seconds (default3600); stdout is one
//! bearer token. The service never offers an unauthenticated token-issuing route.
mod journal_sessions;
use journal_sessions::Sessions;

fn io<T>(value: Result<T, i64>, operation: &str) -> Result<T, String> {
    match value {
        Ok(value) => Ok(value),
        Err(errno) => Err(format!("{operation}: errno {errno}")),
    }
}
fn issue(root: &mut Directory, input: &mut Stdin, entropy: &mut Entropy, clock: &mut Clock) -> Result<String, String> {
    let bytes = io(input.read_to_end(), "read session lifetime")?;
    let input = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => return Err("session lifetime must be UTF-8 decimal text".to_string()),
    };
    let lifetime = {
        let text = std::str::trim_ascii(input.as_str());
        if text.is_empty() { 3600usize } else {
            match text.parse::<usize>() {
                Ok(value) => value,
                Err(_) => return Err("session lifetime must be a positive integer number of seconds".to_string()),
            }
        }
    };
    let sessions = Sessions::read(root)?;
    let now = io(clock.unix_seconds(), "read session wall clock")?;
    sessions.issue(entropy, now, lifetime)
}
pub fn main(mut output: Stdout, mut errors: Stderr, mut input: Stdin,
    mut root: Directory, mut entropy: Entropy, mut clock: Clock) {
    match issue(&mut root, &mut input, &mut entropy, &mut clock) {
        Ok(token) => output.write(format!("{token}\n").as_bytes()),
        Err(problem) => {
            errors.write(format!("session token: {problem}\n").as_bytes());
            panic!("session token issuance failed");
        }
    }
}
