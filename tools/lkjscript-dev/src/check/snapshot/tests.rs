//! Changed-profile coverage for ordinary native program inputs.
use super::{changed_profile, checked_bytes};
use crate::check::registry;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn git(root: &Path, arguments: &[&str]) {
    let mut command = vec![
        "git",
        "-c",
        "user.name=fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "commit.gpgsign=false",
    ];
    command.extend_from_slice(arguments);
    checked_bytes(root, &command).expect("owned fixture Git operation");
}

fn fixture() -> tempfile::TempDir {
    let root = tempfile::TempDir::new().expect("owned Git fixture");
    git(root.path(), &["init", "--quiet"]);
    root
}

fn write(root: &Path, path: &str, bytes: &[u8]) {
    let destination = root.join(path);
    fs::create_dir_all(destination.parent().expect("fixture parent")).unwrap();
    fs::write(destination, bytes).unwrap();
}

fn tracked(path: &str) -> tempfile::TempDir {
    let root = fixture();
    write(root.path(), path, b"original fixture\n");
    git(root.path(), &["add", "--", path]);
    git(
        root.path(),
        &["commit", "--quiet", "-m", "fixture baseline"],
    );
    root
}

fn assert_full(root: &Path, label: &str) {
    let selected = changed_profile(root).expect("changed selection");
    // These obligations are asserted independently of the profile's name or count.
    for required in ["release_build", "workspace_tests", "product_surface_audit"] {
        assert!(
            selected.iter().any(|gate| gate == required),
            "{label}: missing {required}: {selected:?}"
        );
    }
    assert_eq!(selected, registry::profile("full").unwrap(), "{label}");
}

#[test]
fn native_examples_select_full_when_untracked() {
    for path in [
        "docs/guides/examples/ui.lkjc",
        "docs/guides/examples/ui-tests.lkjc",
        "docs/guides/examples/web-starter.lkjc",
        "docs/guides/examples/form-codec.lkjc",
        "docs/guides/examples/editor.deployment.json",
        "docs/guides/examples/unowned.asset",
    ] {
        let root = fixture();
        write(root.path(), path, b"new fixture\n");
        assert_full(root.path(), path);
    }
}

#[test]
fn native_examples_select_full_for_modified_staged_and_deleted_files() {
    for path in [
        "docs/guides/examples/ui.lkjc",
        "docs/guides/examples/editor.deployment.json",
    ] {
        for state in ["modified", "staged", "deleted"] {
            let root = tracked(path);
            if state == "deleted" {
                fs::remove_file(root.path().join(path)).unwrap();
            } else {
                write(root.path(), path, b"changed fixture\n");
                if state == "staged" {
                    git(root.path(), &["add", "--", path]);
                }
            }
            assert_full(root.path(), &format!("{state}: {path}"));
        }
    }
}

#[test]
fn native_example_renames_select_both_source_and_destination() {
    let code = "docs/guides/examples/ui.lkjc";
    let prose = "docs/moved-ui.md";
    for (from, to) in [(code, prose), (prose, code)] {
        let root = tracked(from);
        fs::create_dir_all(root.path().join(to).parent().unwrap()).unwrap();
        git(root.path(), &["mv", "--", from, to]);
        assert_full(root.path(), &format!("{from} -> {to}"));
    }
}

#[test]
fn prose_and_similar_prefixes_remain_lightweight() {
    let expected = BTreeSet::from(["diff_check".to_owned(), "rust_only_tooling".to_owned()]);
    for path in [
        "README.md",
        "AGENTS.md",
        "docs/status.md",
        "docs/guides/native-web.md",
        "docs/guides/examples.md",
        "docs/guides/examples-extra/ui.lkjc",
        "docs/releases/v0.1.47.md",
    ] {
        let root = fixture();
        write(root.path(), path, b"ordinary documentation\n");
        let selected: BTreeSet<_> = changed_profile(root.path()).unwrap().into_iter().collect();
        assert_eq!(selected, expected, "{path}");
    }
}
