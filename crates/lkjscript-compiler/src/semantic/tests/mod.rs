#![allow(clippy::expect_used, clippy::panic)]

mod codec;
mod diagnostics;
mod publication;
mod publication_conflict;
mod query;
mod transaction;

use std::path::PathBuf;

use crate::semantic::schema::Response;

fn case_dir(label: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("unnamed")
        .replace(':', "-");
    let path = std::env::temp_dir().join(format!(
        "lkjscript-semantic-{}-{thread}-{label}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create semantic test directory");
    let initialized = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&path)
        .status()
        .expect("run git init");
    assert!(initialized.success());
    path
}

fn request(root: &std::path::Path, operation: &str) -> Vec<u8> {
    format!(
        "{{\"schema\":\"{}\",\"version\":1,\"profile\":\"default\",\"root\":{},\"operation\":{operation}}}",
        crate::semantic::SCHEMA,
        serde_json::to_string(&root.to_string_lossy()).expect("encode root")
    )
    .into_bytes()
}

fn response(bytes: &[u8]) -> Response {
    serde_json::from_slice(bytes).expect("decode typed semantic response")
}
