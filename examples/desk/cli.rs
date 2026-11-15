//! Ordinary command arguments and named environment values, with no World authority.
use super::store::{self, Row};
const HELP: &str = "desk: the owner's and the agents' shared notebook.\n\n  desk note TEXT [--from NAME] [--stdin]\n  desk ask TEXT [--options A,B,C] [--from NAME] [--stdin]\n  desk close ID [--from NAME]\n  desk inbox [--peek]\n  desk path\n\nDESK_STATE must name the granted, existing state directory. TZ defaults to UTC.\nThe service is a separate serve entry; child commands and wake are not granted.\n";
fn option(args: &[String], name: &str, default: &str) -> String {
    let mut result = default.to_string();
    let mut index = 1usize;
    while index + 1usize < args.len() {
        if args[index] == name { result = args[index + 1usize].clone(); }
        index += 1usize;
    }
    result
}
fn present(args: &[String], name: &str) -> bool {
    for arg in args { if arg == name { return true; } }
    false
}
fn words(args: &[String]) -> Vec<String> {
    let mut words = Vec::new();
    for index in 1usize..args.len() {
        if !args[index].starts_with("--") && !args[index - 1usize].starts_with("--") {
            words.push(args[index].clone());
        }
    }
    words
}
fn environment(value: &mut Environment) -> Result<String, String> {
    match value.get() {
        None => Ok(String::new()),
        Some(bytes) => match String::from_utf8(bytes) {
            Ok(text) => Ok(text), Err(_) => Err("granted environment value is not UTF-8".to_string()),
        },
    }
}
pub fn run(args: &mut Arguments, state: &mut Directory, state_env: &mut Environment,
    timezone: &mut Environment, output: &mut Stdout, input: &mut Stdin,
    clock: &mut Clock, entropy: &mut Entropy) -> Result<u8, String> {
    let mut values = Vec::new();
    let raw = args.args();
    // Arguments grants contain user arguments; the executable name is omitted.
    for index in 0usize..raw.len() {
        match String::from_utf8(raw[index].clone()) {
            Ok(text) => values.push(text), Err(_) => return Err("command argument is not UTF-8".to_string()),
        }
    }
    let command = if values.is_empty() { "help" } else { values[0usize].as_str() };
    let words = words(&values);
    let who = option(&values, "--from", "claude");
    if command == "note" || command == "ask" {
        let mut text = String::new();
        for word in &words {
            if !text.is_empty() { text.push(' '); }
            text.push_str(word);
        }
        if present(&values, "--stdin") {
            let bytes = store::io(input.read_to_end(), "read desk standard input")?;
            let from_input = match String::from_utf8(bytes) {
                Ok(text) => text, Err(_) => return Err("standard input is not UTF-8".to_string()),
            };
            let from_input = std::text::trim(&from_input);
            if !text.is_empty() && !from_input.is_empty() { text.push(' '); }
            text.push_str(from_input);
        }
        if !text.is_empty() {
            let mut row = Row::new(command, &who, &text);
            let options = option(&values, "--options", "");
            if !options.is_empty() {
                row.has_options = true;
                for option in options.as_str().split(',') {
                    let option = std::text::trim(option);
                    if !option.is_empty() { row.options.push(option.to_string()); }
                }
            }
            let row = store::append(state, "notes.jsonl", row, clock, entropy)?;
            output.write(format!("{}\n", row.id).as_bytes());
            store::touch_alive(state)?;
            return Ok(0u8);
        }
    } else if command == "close" && !words.is_empty() {
        let mut row = Row::new("close", &who, "");
        row.reference = words[0usize].clone();
        store::append(state, "notes.jsonl", row, clock, entropy)?;
        return Ok(0u8);
    } else if command == "inbox" {
        let name = environment(timezone)?;
        let offset = std::time::timezone_offset(if name.is_empty() { "UTC" } else { name.as_str() })?;
        let text = store::inbox(state, present(&values, "--peek"), offset)?;
        output.write(text.as_bytes());
        return Ok(0u8);
    } else if command == "path" {
        let path = environment(state_env)?;
        if path.is_empty() { return Err("DESK_STATE must name the granted state directory".to_string()); }
        output.write(format!("{path}\n").as_bytes());
        return Ok(0u8);
    }
    output.write(HELP.as_bytes());
    Ok(if command == "help" { 0u8 } else { 2u8 })
}

#[test]
fn argument_words_and_options() {
    let args = vec!["ask".to_string(), "日本語".to_string(), "question".to_string(),
        "--options".to_string(), " A, B ".to_string(), "--from".to_string(), "agent".to_string()];
    let words = words(&args);
    assert!(words.len() == 2usize && words[0usize] == "日本語" && words[1usize] == "question");
    assert!(option(&args, "--from", "claude") == "agent");
}
