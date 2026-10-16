use super::*;
use crate::check::cargo_origin::tests::{prepare, produce};
use std::fs;

pub(super) fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("synthetic Cargo fixture");
    prepare(root.path()).expect("prepare record fixture");
    root
}

pub(super) fn build(repository: &Path, target: &Path, name: &str) -> GateReceipt {
    produce(repository, target, name, name == "warm").expect("observe synthetic Cargo records")
}

#[test]
fn redirected_cargo_records_cannot_certify_a_preexisting_regular_output() {
    let root = fixture();
    let original =
        fs::read(root.path().join("target/release/lkjscript")).expect("predecessor bytes");
    let receipt = build(root.path(), &root.path().join("elsewhere"), "redirected");
    assert_eq!(
        receipt.process.as_ref().expect("process").status,
        ProcessStatus::Passed
    );
    assert_eq!(
        fs::read(root.path().join("target/release/lkjscript")).expect("original bytes"),
        original
    );
    assert_eq!(receipt.status, GateStatus::Failed);
    assert!(
        receipt
            .reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with("cargo_output_binding:"))
    );
}

#[test]
fn compiled_and_cached_cargo_records_bind_the_same_exact_output() {
    let root = fixture();
    for name in ["cold", "warm"] {
        let receipt = build(root.path(), &root.path().join("target"), name);
        assert_eq!(receipt.status, GateStatus::Passed, "{receipt:?}");
        assert_eq!(receipt.outputs.len(), 1);
        assert_eq!(receipt.retained_outputs.len(), 1);
        let log = fs::read_to_string(root.path().join(format!(
            ".artifacts/lkjscript-dev/check/{name}/{name}.stdout.log"
        )))
        .expect("observed records");
        assert!(log.contains(if name == "cold" {
            "\"fresh\":false"
        } else {
            "\"fresh\":true"
        }));
    }
}

#[test]
fn changed_source_cannot_substitute_a_predecessor_output() {
    let root = fixture();
    let before = build(root.path(), &root.path().join("target"), "before");
    assert_eq!(before.status, GateStatus::Passed);
    let path = root.path().join("target/release/lkjscript");
    let predecessor = fs::read(&path).expect("original bytes");
    fs::write(
        root.path().join("src/main.rs"),
        "fn main() { let _ = 1; }\n",
    )
    .expect("changed source");
    let changed = build(root.path(), &root.path().join("elsewhere"), "changed");
    assert_eq!(changed.status, GateStatus::Failed);
    assert_eq!(fs::read(path).expect("unchanged predecessor"), predecessor);
    assert_ne!(
        fs::read(root.path().join("elsewhere/release/lkjscript")).expect("selected bytes"),
        predecessor
    );
}
