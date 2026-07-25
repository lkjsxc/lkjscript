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
            "{{\"schema\":\"lkjscript.agent-foundation\",\"version\":1,",
            "\"profile\":\"default\",\"root\":{},",
            "\"operation\":{{\"kind\":\"snapshot\"}}}}",
        ),
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
    assert!(stdout.contains("\"schema\":\"lkjscript.agent-foundation\""));
    assert!(!stdout.contains("lkjscript: "));
}
