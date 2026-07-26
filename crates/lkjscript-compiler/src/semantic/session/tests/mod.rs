#![allow(clippy::expect_used, clippy::panic)]

mod framing;
mod ledger;
mod lifecycle;
mod transaction;

use std::path::PathBuf;

use super::SemanticSession;

fn case(label: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("unnamed")
        .replace(':', "-");
    let directory = std::env::temp_dir().join(format!(
        "lkjscript-session-{}-{thread}-{label}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create session case");
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&directory)
        .status()
        .expect("initialize session repository");
    assert!(status.success());
    directory
}

fn semantic_request(root: &std::path::Path, profile: &str, operation: &str) -> String {
    format!(
        concat!(
            "{{\"schema\":\"lkjscript.semantic-source\",\"contract\":\"{}\",",
            "\"profile\":\"{profile}\",\"root\":{},\"operation\":{operation}}}"
        ),
        crate::semantic::CONTRACT,
        serde_json::to_string(&root.to_string_lossy()).expect("encode root"),
        profile = profile,
        operation = operation,
    )
}

fn session_request(id: &str, revision: u64, operation: &str) -> Vec<u8> {
    format!(
        concat!(
            "{{\"schema\":\"lkjscript.semantic-session\",\"contract\":\"{}\",",
            "\"request_id\":{},\"revision\":{revision},\"request\":{operation}}}"
        ),
        super::CONTRACT,
        serde_json::to_string(id).expect("encode request id"),
        revision = revision,
        operation = operation,
    )
    .into_bytes()
}

fn execute_operation(request: &str) -> String {
    format!("{{\"kind\":\"execute\",\"request\":{request}}}")
}

fn handle(session: &mut SemanticSession, request: &[u8]) -> (Vec<u8>, serde_json::Value) {
    let encoded = session.handle(request).expect("handle session request");
    let decoded = serde_json::from_slice(&encoded).expect("decode session response");
    (encoded, decoded)
}

fn frame(payload: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(payload.len() + 8);
    output.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    output.extend_from_slice(payload);
    output
}
