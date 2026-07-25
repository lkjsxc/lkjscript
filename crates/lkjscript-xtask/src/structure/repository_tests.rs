use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repository() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_nanos());
    std::env::temp_dir().join(format!(
        "lkjscript-structure-{}-{nonce}",
        std::process::id()
    ))
}

fn git(root: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(unix)]
#[test]
fn tracked_hidden_symlink_and_ignored_boundaries() {
    use std::os::unix::fs::symlink;
    let root = repository();
    assert!(fs::create_dir(&root).is_ok());
    assert!(git(&root, &["init", "-q"]));
    assert!(fs::write(root.join(".gitignore"), "ignored\n").is_ok());
    assert!(fs::write(root.join(".hidden"), "visible to git\n").is_ok());
    assert!(fs::write(root.join("ignored"), "not authority\n").is_ok());
    assert!(symlink(".hidden", root.join("link")).is_ok());
    assert!(git(&root, &["add", ".gitignore", ".hidden", "link"]));
    assert!(git(
        &root,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "base"
        ]
    ));
    let snapshot = super::repository::capture(&root, &[]);
    assert!(snapshot.is_ok());
    if let Ok(snapshot) = snapshot {
        let paths: Vec<_> = snapshot
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        assert!(paths.contains(&".hidden"));
        assert!(!paths.contains(&"ignored"));
        assert!(paths.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(snapshot
            .findings
            .iter()
            .any(|finding| finding.rule == "LKJ-REPO-SYMLINK"));
    }
    assert!(fs::remove_dir_all(root).is_ok());
}

#[test]
fn exact_data_width_classification_is_closed_and_visible() {
    let fixture = "let source = \"main/\\n/main\";";
    let measured = "| exact result | 123456 | <!-- LKJ-EXACT-DATA -->";
    let unmarked = "| prose-shaped record | not exempt |";
    let prose = "ordinary prose <!-- LKJ-EXACT-DATA -->";
    assert_eq!(
        super::repository_support::line_widths("crate/tests/example.rs", fixture),
        Ok((fixture.len() as u64, 0, 1))
    );
    assert_eq!(
        super::repository_support::line_widths("evidence.md", measured),
        Ok((measured.len() as u64, 0, 1))
    );
    assert_eq!(
        super::repository_support::line_widths("docs/current-state/evidence.md", unmarked),
        Ok((unmarked.len() as u64, unmarked.len() as u64, 0))
    );
    assert_eq!(
        super::repository_support::line_widths("evidence.md", prose),
        Ok((prose.len() as u64, prose.len() as u64, 0))
    );
}

#[test]
fn tracked_non_utf8_is_unclassified() {
    let root = repository();
    assert!(fs::create_dir(&root).is_ok());
    assert!(git(&root, &["init", "-q"]));
    assert!(fs::write(root.join("compressed.bin"), [0x1f, 0x8b, 0xff]).is_ok());
    assert!(git(&root, &["add", "compressed.bin"]));
    assert!(git(
        &root,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "base"
        ]
    ));
    let snapshot = super::repository::capture(&root, &[]);
    assert!(snapshot.is_ok());
    if let Ok(snapshot) = snapshot {
        assert!(snapshot
            .findings
            .iter()
            .any(|finding| finding.rule == "LKJ-REPO-UNCLASSIFIED"));
        assert_eq!(snapshot.files.first().map(|file| file.bytes), Some(3));
    }
    assert!(fs::remove_dir_all(root).is_ok());
}
