//! Changed-profile coverage for native programs and embedded Rust-site documents.
use super::{
    capture_profile, capture_profile_between_samples, changed_profile, checked_bytes, site_inputs,
};
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

fn assert_development(root: &Path, label: &str, requested: &[&str], closure: &[&str]) {
    let selected = changed_profile(root).expect("changed selection");
    let expected = requested.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(
        selected.iter().map(String::as_str).collect::<BTreeSet<_>>(),
        expected,
        "{label}: requested"
    );
    let outputs = tempfile::tempdir().unwrap();
    let registry = registry::base_registry(root, outputs.path(), Path::new("/bin/true")).unwrap();
    let actual = registry.closure(&selected).unwrap();
    assert_eq!(
        actual.iter().map(String::as_str).collect::<BTreeSet<_>>(),
        closure.iter().copied().collect(),
        "{label}: dependency-complete gates"
    );
}

fn assert_site(root: &Path, label: &str) {
    assert_development(
        root,
        label,
        &["diff_check", "rust_only_tooling", "site_tests"],
        &[
            "fmt",
            "diff_check",
            "rust_only_tooling",
            "site_clippy",
            "site_tests",
        ],
    );
}

fn assert_checker(root: &Path, label: &str) {
    assert_development(
        root,
        label,
        &[
            "diff_check",
            "rust_only_tooling",
            "checker_self_test",
            "checker_library_tests",
        ],
        &[
            "fmt",
            "diff_check",
            "rust_only_tooling",
            "checker_self_test",
            "checker_library_tests",
        ],
    );
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
fn embedded_document_selects_only_site_execution_and_policy_dependencies() {
    let root = tracked("docs/status.md");
    write(
        root.path(),
        "docs/status.md",
        b"changed embedded document\n",
    );
    assert_site(root.path(), "embedded status");
}

#[test]
fn every_published_document_selects_execution_for_all_git_states() {
    assert!(!site_inputs::PATHS.is_empty());
    for path in site_inputs::PATHS {
        for state in ["untracked", "modified", "staged", "deleted"] {
            let root = if state == "untracked" {
                fixture()
            } else {
                tracked(path)
            };
            if state == "deleted" {
                fs::remove_file(root.path().join(path)).unwrap();
            } else {
                write(root.path(), path, b"changed embedded document\n");
                if state == "staged" {
                    git(root.path(), &["add", "--", path]);
                }
            }
            assert_site(root.path(), &format!("{path}: {state}"));
        }
    }
}

#[test]
fn embedded_document_renames_select_both_source_and_destination() {
    let document = "docs/status.md";
    let historical = "docs/campaigns/moved-status.md";
    for (from, to) in [(document, historical), (historical, document)] {
        let root = tracked(from);
        fs::create_dir_all(root.path().join(to).parent().unwrap()).unwrap();
        git(root.path(), &["mv", "--", from, to]);
        assert_site(root.path(), &format!("{from} -> {to}"));
    }
}

#[test]
fn checker_changes_select_library_regressions_and_self_test_for_all_git_states() {
    for path in [
        "tools/lkjscript-dev/src/check/snapshot.rs",
        "tools/lkjscript-dev/src/check/snapshot/tests.rs",
        "tools/lkjscript-dev/src/check/policy.rs",
        "tools/lkjscript-dev/src/check/policy/native/tests.rs",
        "tools/lkjscript-dev/Cargo.toml",
    ] {
        for state in ["untracked", "modified", "staged", "deleted"] {
            let root = if state == "untracked" {
                fixture()
            } else {
                tracked(path)
            };
            if state == "deleted" {
                fs::remove_file(root.path().join(path)).unwrap();
            } else {
                write(root.path(), path, b"changed checker fixture\n");
                if state == "staged" {
                    git(root.path(), &["add", "--", path]);
                }
            }
            assert_checker(root.path(), &format!("{path}: {state}"));
        }
    }
}

#[test]
fn checker_renames_select_both_source_and_destination() {
    let checker = "tools/lkjscript-dev/src/check/policy.rs";
    let prose = "docs/campaigns/checker-fixture.md";
    for (from, to) in [(checker, prose), (prose, checker)] {
        let root = tracked(from);
        fs::create_dir_all(root.path().join(to).parent().unwrap()).unwrap();
        git(root.path(), &["mv", "--", from, to]);
        assert_checker(root.path(), &format!("{from} -> {to}"));
    }
}

#[test]
fn site_and_checker_changes_combine_without_dropping_development_only_gates() {
    let root = fixture();
    write(root.path(), "docs/status.md", b"site input\n");
    write(
        root.path(),
        "tools/lkjscript-dev/src/check/policy.rs",
        b"checker input\n",
    );
    assert_development(
        root.path(),
        "combined development inputs",
        &[
            "diff_check",
            "rust_only_tooling",
            "checker_self_test",
            "checker_library_tests",
            "site_tests",
        ],
        &[
            "fmt",
            "diff_check",
            "rust_only_tooling",
            "checker_self_test",
            "checker_library_tests",
            "site_clippy",
            "site_tests",
        ],
    );
}

#[test]
fn product_source_and_unclassified_tool_changes_keep_full_acceptance() {
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "src/lib.rs",
        "tests/public_cli.rs",
        "docs/guides/examples/workload.lkjc",
        "tools/lkjscript-site/src/documents.rs",
        "tools/lkjscript-site/Cargo.toml",
        "tools/lkjscript-dev/src/check-extra/unknown.rs",
        "tools/lkjscript-dev/Cargo.toml.extra",
    ] {
        let root = fixture();
        write(root.path(), "docs/status.md", b"site input\n");
        write(
            root.path(),
            "tools/lkjscript-dev/src/check/policy.rs",
            b"checker input\n",
        );
        write(root.path(), path, b"requires complete acceptance\n");
        assert_full(root.path(), path);
    }
}

#[test]
fn prose_and_similar_prefixes_remain_lightweight() {
    let expected = BTreeSet::from(["diff_check".to_owned(), "rust_only_tooling".to_owned()]);
    for path in [
        "README.md",
        "AGENTS.md",
        "docs/status.md.extra",
        "docs/guides/native-web.md.extra",
        "docs/campaigns/20260929-observation.md",
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

#[test]
fn changed_selection_rejects_source_edits_between_selection_and_snapshot_admission() {
    let root = tracked("Cargo.lock");
    write(root.path(), "docs/status.md", b"embedded status edit\n");
    let (_, before) = capture_profile(root.path(), "changed").unwrap();
    assert!(before.iter().any(|gate| gate == "site_tests"));
    assert!(!before.iter().any(|gate| gate == "workspace_tests"));
    let failure = capture_profile_between_samples(root.path(), "changed", || {
        write(root.path(), "src/lib.rs", b"this is a compilation error\n");
        Ok(())
    })
    .unwrap_err();
    assert!(
        failure
            .message()
            .contains("changed during verification admission")
    );
    let (_, after) = capture_profile(root.path(), "changed").unwrap();
    assert_eq!(after, registry::profile("full").unwrap());
}

#[test]
fn changed_selection_rejects_commit_that_cleans_status_during_sampling() {
    let root = tracked("Cargo.lock");
    write(root.path(), "src/lib.rs", b"pub fn candidate() {}\n");
    let failure = capture_profile_between_samples(root.path(), "changed", || {
        git(root.path(), &["add", "--", "src/lib.rs"]);
        git(
            root.path(),
            &["commit", "--quiet", "-m", "concurrent candidate"],
        );
        Ok(())
    })
    .unwrap_err();
    assert!(
        failure
            .message()
            .contains("changed during verification admission")
    );
}
