use std::fs;

use crate::agent::storage::{self, FailurePoint};

use super::support;

#[test]
fn quarantines_corrupt_state_deterministically_without_overwrite() {
    let repo = support::repository("quarantine");
    let path = storage::state_path(&repo.root, "corrupt-task");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let corrupt = b"{not-json}\n";
    fs::write(&path, corrupt).unwrap();
    let first = storage::load(&repo.root, "corrupt-task").unwrap_err();
    assert!(first.contains("quarantined corrupt state"));
    let quarantine = path.parent().unwrap().join("quarantine");
    let names: Vec<_> = fs::read_dir(&quarantine)
        .unwrap()
        .map(|item| item.unwrap().path())
        .collect();
    assert_eq!(names.len(), 1);
    assert_eq!(fs::read(&names[0]).unwrap(), corrupt);

    fs::write(&path, corrupt).unwrap();
    let second = storage::load(&repo.root, "corrupt-task").unwrap_err();
    assert!(second.contains("quarantined corrupt state"));
    assert!(!path.exists());
    assert_eq!(fs::read_dir(quarantine).unwrap().count(), 1);
}

#[test]
fn oversized_state_is_quarantined_with_full_content_hash() {
    let repo = support::repository("oversized-quarantine");
    let path = storage::state_path(&repo.root, "oversized-task");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let bytes = vec![b'x'; crate::agent::bounds::STATE_BYTES + 1];
    let hash = crate::sha256::digest(&bytes);
    fs::write(&path, &bytes).unwrap();
    assert!(storage::load(&repo.root, "oversized-task")
        .unwrap_err()
        .contains("quarantined corrupt state"));
    let quarantined = path
        .parent()
        .unwrap()
        .join("quarantine")
        .join(format!("oversized-task-{hash}.json"));
    assert_eq!(fs::metadata(quarantined).unwrap().len(), bytes.len() as u64);
}

#[test]
fn atomic_failures_leave_previous_state_byte_identical() {
    let repo = support::repository("atomic-failure");
    let state = support::state(&repo, "atomic-task");
    support::publish(&repo, &state);
    let path = storage::state_path(&repo.root, "atomic-task");
    let original = fs::read(&path).unwrap();
    let replacement = b"replacement bytes\n";
    for failure in [
        FailurePoint::AfterCreate,
        FailurePoint::AfterWrite,
        FailurePoint::AfterFileSync,
    ] {
        assert!(storage::write_state_at(&repo.root, "atomic-task", replacement, failure).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        let temporary = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .any(|item| item.file_name().to_string_lossy().ends_with(".tmp"));
        assert!(!temporary);
    }
}

#[test]
fn concurrent_checkpoint_exclusion_rejects_second_writer() {
    let repo = support::repository("concurrent-lock");
    let first = storage::lock(&repo.root, "concurrent-task").unwrap();
    assert!(storage::lock(&repo.root, "concurrent-task")
        .unwrap_err()
        .contains("concurrent checkpoint"));
    drop(first);
    assert!(storage::lock(&repo.root, "concurrent-task").is_ok());
}
