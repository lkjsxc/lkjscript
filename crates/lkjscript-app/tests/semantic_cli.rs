#![allow(clippy::expect_used)]

use std::io::Write;
use std::process::{Command, Stdio};

fn invoke(args: &[&str], input: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn semantic CLI");
    child
        .stdin
        .take()
        .expect("semantic stdin")
        .write_all(input)
        .expect("write semantic request");
    child.wait_with_output().expect("wait for semantic CLI")
}

fn framed(payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 8);
    frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

#[test]
fn semantic_session_uses_exact_framing_and_command() {
    let payload = format!(
        concat!(
            "{{\"schema\":\"lkjscript.semantic-session\",\"contract\":\"{}\",",
            "\"request_id\":\"stop\",\"revision\":0,",
            "\"request\":{{\"kind\":\"shutdown\"}}}}"
        ),
        lkjscript_contracts::AGENT_PROTOCOL_DIGEST,
    );
    let output = invoke(
        &["semantic", "serve", "--stdio"],
        &framed(payload.as_bytes()),
    );
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(output.stdout.len() >= 8);
    let length = u64::from_be_bytes(output.stdout[..8].try_into().expect("session header"));
    assert_eq!(output.stdout.len() as u64, length + 8);
    let response = String::from_utf8(output.stdout[8..].to_vec()).expect("session JSON");
    assert!(response.contains("\"request_id\":\"stop\""));
    assert!(response.contains("\"kind\":\"shutdown\""));

    let partial = invoke(&["semantic", "serve", "--stdio"], &[0, 0, 0]);
    assert!(!partial.status.success());
    assert!(partial.stdout.is_empty());
    assert!(String::from_utf8(partial.stderr)
        .expect("process diagnostic")
        .contains("partial_header"));
    assert!(!invoke(&["semantic", "serve"], &[]).status.success());
}

#[test]
fn semantic_cli_keeps_protocol_and_process_errors_separate() {
    let malformed = invoke(&["semantic", "-"], b"{\"schema\":0,\"schema\":1}");
    assert!(!malformed.status.success());
    assert!(malformed.stdout.is_empty());
    let stderr = String::from_utf8(malformed.stderr).expect("UTF-8 diagnostic");
    assert!(stderr.contains("invalid_json"));

    let directory =
        std::env::temp_dir().join(format!("lkjscript-semantic-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create CLI fixture directory");
    let root = directory.join("main.lkjscript");
    std::fs::write(&root, "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n").expect("write CLI fixture");
    let request = format!(
        concat!(
            "{{\"schema\":\"lkjscript.semantic-source\",\"contract\":\"{}\",",
            "\"profile\":\"default\",\"root\":{},",
            "\"operation\":{{\"kind\":\"snapshot\"}}}}",
        ),
        lkjscript_contracts::SEMANTIC_SOURCE_DIGEST,
        format!("{:?}", root.to_string_lossy())
    );
    let redirected = invoke(&["semantic"], request.as_bytes());
    let explicit = invoke(&["semantic", "-"], request.as_bytes());
    assert!(redirected.status.success());
    assert!(explicit.status.success());
    assert!(redirected.stderr.is_empty());
    assert!(explicit.stderr.is_empty());
    assert_eq!(redirected.stdout, explicit.stdout);
    let stdout = String::from_utf8(redirected.stdout).expect("UTF-8 protocol");
    assert!(stdout.contains("\"schema\":\"lkjscript.semantic-source\""));
    assert!(!stdout.contains("lkjscript: "));
}
