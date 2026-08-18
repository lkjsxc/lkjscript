#![allow(clippy::expect_used, clippy::panic)]

use serde_json::Value;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDirectory(PathBuf);

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn private_store() -> TestDirectory {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "lkjscript-runtime-session-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&path).expect("create runtime store");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
        .expect("make runtime store private");
    TestDirectory(path)
}

fn request(version: u16, request_id: u64, kind: &str) -> String {
    format!(
        "{{\"version\":{version},\"request_id\":{request_id},\"request\":{{\"kind\":\"{kind}\"}}}}"
    )
}

#[test]
fn runtime_session_recovers_by_line_correlates_and_releases_its_store_lock() {
    let store = private_store();
    let mut child = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args(["runtime", "session", "--store"])
        .arg(&store.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start runtime session");
    {
        let stdin = child.stdin.as_mut().expect("runtime stdin");
        writeln!(stdin, "{{").expect("malformed request");
        writeln!(stdin, "{}", request(0, 11, "inspect_runtime")).expect("old version");
        writeln!(stdin, "{}", request(1, 0, "inspect_runtime")).expect("zero ID");
        writeln!(stdin, "{{\"version\":1,\"request_id\":12,\"request_id\":12,\"request\":{{\"kind\":\"inspect_runtime\"}}}}")
            .expect("duplicate field");
        writeln!(stdin, "{}", request(1, 13, "inspect_runtime")).expect("inspection");
        writeln!(stdin, "{}", request(1, 14, "shutdown")).expect("shutdown");
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("wait for runtime session");
    assert!(
        output.status.success(),
        "runtime session failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let responses = String::from_utf8(output.stdout)
        .expect("runtime output UTF-8")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("runtime response JSON"))
        .collect::<Vec<_>>();
    assert_eq!(responses.len(), 6);
    assert!(responses[0].get("request_id").is_none());
    assert_eq!(responses[0]["error"]["code"], "protocol_malformed");
    assert_eq!(responses[1]["request_id"], 11);
    assert_eq!(responses[1]["error"]["code"], "protocol_malformed");
    assert_eq!(responses[2]["request_id"], 0);
    assert_eq!(responses[2]["error"]["code"], "protocol_malformed");
    assert!(responses[3].get("request_id").is_none());
    assert_eq!(responses[3]["error"]["code"], "protocol_malformed");
    assert_eq!(responses[4]["request_id"], 13);
    assert_eq!(responses[4]["response"]["kind"], "runtime");
    assert_eq!(responses[5]["request_id"], 14);
    assert_eq!(responses[5]["response"]["kind"], "shutdown");

    let reopened = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args(["runtime", "inspect", "--store"])
        .arg(Path::new(&store.0))
        .output()
        .expect("reopen runtime store");
    assert!(
        reopened.status.success(),
        "runtime store lock was not released: {}",
        String::from_utf8_lossy(&reopened.stderr)
    );
}
