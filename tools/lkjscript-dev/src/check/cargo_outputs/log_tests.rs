use super::tests::{artifact, finish, transcript};
use super::*;
use crate::evidence;
use serde_json::json;

#[test]
fn recorded_stdout_bytes_and_limit_are_independently_rechecked() {
    let root = tempfile::tempdir().expect("record fixture");
    let log = root.path().join("stdout.log");
    let output = root.path().join("target/release/lkjscript");
    let mut message = artifact();
    message["manifest_path"] = json!(root.path().join("Cargo.toml"));
    message["executable"] = json!(output);
    message["filenames"] = json!([output]);
    std::fs::write(&log, transcript(&[message, finish()])).expect("original records");
    let proof = evidence::proof(&log, "stdout.log".into()).expect("original observation");
    let mut gate = Gate::new(
        "producer",
        vec![
            "cargo".into(),
            "build".into(),
            "--message-format=json".into(),
        ],
    );
    gate.required_outputs = vec![output];
    assert!(verify(root.path(), &gate, &log, &proof).is_ok());
    gate.maximum_stdout_bytes = proof.bytes.expect("length") - 1;
    assert!(verify(root.path(), &gate, &log, &proof).is_err());
    gate.maximum_stdout_bytes += 1;
    let mut changed = std::fs::read(&log).expect("records");
    changed[0] = b' ';
    std::fs::write(&log, changed).expect("same-length corruption");
    assert!(verify(root.path(), &gate, &log, &proof).is_err());
}

#[test]
fn json_option_after_test_argument_separator_is_not_a_build_selection() {
    let root = tempfile::tempdir().expect("record fixture");
    let log = root.path().join("stdout.log");
    std::fs::write(&log, b"").expect("empty log");
    let proof = evidence::proof(&log, "stdout.log".into()).expect("log proof");
    let mut gate = Gate::new(
        "producer",
        vec![
            "cargo".into(),
            "test".into(),
            "--".into(),
            "--message-format=json".into(),
        ],
    );
    gate.required_outputs
        .push(root.path().join("target/release/lkjscript"));
    assert!(verify(root.path(), &gate, &log, &proof).is_err());
    gate.required_outputs.clear();
    assert!(verify(root.path(), &gate, &log, &proof).is_ok());
}
