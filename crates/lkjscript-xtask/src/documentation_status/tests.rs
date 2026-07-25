use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::check;

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn fixture(name: &str, claim: &str) -> PathBuf {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "lkjscript-status-{name}-{}-{id}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    assert!(fs::create_dir_all(root.join("meta/config")).is_ok());
    assert!(fs::create_dir_all(root.join("docs")).is_ok());
    let registry = r#"{
  "schema": "lkjscript.capability-status",
  "version": 1,
  "capabilities": [{
    "id": "test/1",
    "status": "current",
    "interface": "test command",
    "authority": "docs/authority.md",
    "evidence": "docs/authority.md",
    "claimants": ["docs/claim.md"]
  }]
}
"#;
    assert!(fs::write(root.join("meta/config/capability-status.json"), registry).is_ok());
    assert!(fs::write(root.join("docs/authority.md"), "# Authority\n").is_ok());
    assert!(fs::write(
        root.join("docs/claim.md"),
        format!("# Claim\n\n## Status\n\n{claim}\n")
    )
    .is_ok());
    root
}

#[test]
fn matching_claim_passes() {
    let root = fixture("matching", "<!-- LKJ-STATUS id=test/1 status=current -->");
    assert_eq!(check(&root), 0);
    assert!(fs::remove_dir_all(root).is_ok());
}

#[test]
fn mismatched_claim_fails() {
    let root = fixture(
        "mismatch",
        "<!-- LKJ-STATUS id=test/1 status=accepted-target -->",
    );
    assert!(check(&root) > 0);
    assert!(fs::remove_dir_all(root).is_ok());
}

#[test]
fn unregistered_claim_fails() {
    let root = fixture(
        "unknown",
        "<!-- LKJ-STATUS id=test/1 status=current -->\n<!-- LKJ-STATUS id=other/1 status=current -->",
    );
    assert!(check(&root) > 0);
    assert!(fs::remove_dir_all(root).is_ok());
}

#[test]
fn claim_outside_status_fails() {
    let root = fixture("outside", "ordinary status prose");
    assert!(fs::write(
        root.join("README.md"),
        "<!-- LKJ-STATUS id=test/1 status=current -->\n"
    )
    .is_ok());
    assert!(check(&root) > 0);
    assert!(fs::remove_dir_all(root).is_ok());
}
