use super::*;
use crate::release_container::tests::{fixture, gzip};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
fn archive(root: &Path, tag: &str) -> (PathBuf, String) {
    let bytes = gzip(&fixture(tag, None));
    let path = root.join(format!("{tag}.tar.gz"));
    fs::write(&path, &bytes).unwrap();
    (
        path,
        container::sha256_bytes(&bytes).unwrap().as_str().to_owned(),
    )
}
#[test]
fn native_install_select_inventory_and_immutable_conflicts() {
    let temp = tempfile::tempdir().unwrap();
    let prefix = temp.path().join("prefix with spaces");
    assert!(list(&prefix).unwrap().versions.is_empty());
    assert!(!prefix.exists());
    let (a, ad) = archive(temp.path(), "v0.1.32");
    let (b, bd) = archive(temp.path(), "v0.1.34");
    let first = install(&prefix, &a, &ad, false).unwrap();
    assert_eq!(first.selected, None);
    assert!(!prefix.join("bin/lkjscript").exists());
    let second = install(&prefix, &b, &bd, true).unwrap();
    assert_eq!(second.selected.as_deref(), Some("v0.1.34"));
    assert!(!second.retained_manager);
    assert_eq!(
        install(&prefix, &b, &bd, true).unwrap().outcome,
        "already-installed"
    );
    let old = select(&prefix, "v0.1.32").unwrap();
    assert_eq!(old.previous.as_deref(), Some("v0.1.34"));
    assert_eq!(
        fs::read_link(prefix.join("bin/lkjscript")).unwrap(),
        PathBuf::from("../lib/lkjscript/versions/v0.1.32/x86_64-unknown-linux-musl/lkjscript")
    );
    assert_eq!(select(&prefix, "v0.1.32").unwrap().selected, old.selected);
    let inventory = list(&prefix).unwrap();
    assert_eq!(
        inventory
            .versions
            .iter()
            .map(|v| v.tag.as_str())
            .collect::<Vec<_>>(),
        ["v0.1.32", "v0.1.34"]
    );
    assert!(
        inventory
            .versions
            .iter()
            .all(|v| v.integrity == "unchecked")
    );
    assert!(select(&prefix, "v0.1.99").is_err());
    let mut tar = fixture("v0.1.32", None);
    tar.resize(tar.len().next_multiple_of(10240), 0);
    let bytes = gzip(&tar);
    fs::write(&a, &bytes).unwrap();
    let digest = container::sha256_bytes(&bytes).unwrap();
    assert_eq!(
        install(&prefix, &a, digest.as_str(), false)
            .unwrap_err()
            .code,
        "runtime_version_conflict"
    );
    let payload = old.version.path;
    fs::write(&payload, b"corrupted").unwrap();
    assert!(select(&prefix, "v0.1.32").is_err());
    assert_eq!(fs::read(&payload).unwrap(), b"corrupted");
    install(&temp.path().join("recovery"), &a, digest.as_str(), true).unwrap();
}
struct Fail(&'static str);
impl Checkpoints for Fail {
    fn at(&self, point: &'static str) -> Result<(), Diagnostic> {
        if point == self.0 {
            Err(io_error(format!("injected {point}")))
        } else {
            Ok(())
        }
    }
}
#[test]
fn native_failure_state_model_and_restart() {
    for point in [
        "before-root-publish",
        "after-root-publish",
        "snapshot-complete",
        "before-version-sync",
        "before-version-publish",
        "after-version-publish",
        "after-version-sync",
        "before-pointer-replace",
        "after-pointer-replace",
        "after-pointer-sync",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let prefix = temp.path().join("prefix");
        let (a, digest) = archive(temp.path(), "v0.1.34");
        let error = install_with(&prefix, &a, &digest, true, &Fail(point)).unwrap_err();
        let inventory = list(&prefix).unwrap();
        assert!(inventory.versions.len() <= 1, "{point}");
        if let Some(tag) = inventory.selected {
            assert_eq!(tag, "v0.1.34");
            assert_eq!(inventory.versions.len(), 1);
            assert!(
                error.notes.iter().any(|n| n.contains("durability")),
                "{point}: {error:?}"
            );
        }
        install(&prefix, &a, &digest, true).unwrap();
        assert!(select(&prefix, "v0.1.34").is_ok(), "{point}");
    }
}
#[test]
fn native_prefix_conflicts_never_touch_unrelated_sentinels() {
    let temp = tempfile::tempdir().unwrap();
    let (a, d) = archive(temp.path(), "v0.1.34");
    let sentinel = temp.path().join("sentinel");
    fs::write(&sentinel, b"unrelated").unwrap();
    for kind in ["binary", "symlink", "unowned-root", "parent-link"] {
        let prefix = temp.path().join(kind);
        fs::create_dir(&prefix).unwrap();
        fs::create_dir(prefix.join("bin")).unwrap();
        match kind {
            "binary" => fs::write(prefix.join("bin/lkjscript"), b"manual").unwrap(),
            "symlink" => symlink(&sentinel, prefix.join("bin/lkjscript")).unwrap(),
            "unowned-root" => fs::create_dir_all(prefix.join("lib/lkjscript")).unwrap(),
            _ => {
                fs::remove_dir(prefix.join("bin")).unwrap();
                symlink(temp.path(), prefix.join("bin")).unwrap();
            }
        }
        assert!(install(&prefix, &a, &d, true).is_err(), "{kind}");
        assert_eq!(fs::read(&sentinel).unwrap(), b"unrelated");
    }
}
#[test]
fn native_lock_is_os_owned_and_concurrent_installations_converge() {
    let temp = tempfile::tempdir().unwrap();
    let prefix = temp.path().join("prefix");
    let (a, d) = archive(temp.path(), "v0.1.34");
    let root = Root::open(&prefix, true, &Ordinary).unwrap().unwrap();
    let held = root.lock(true).unwrap();
    assert_eq!(
        install(&prefix, &a, &d, true).unwrap_err().code,
        "runtime_busy"
    );
    drop(held);
    let outcomes = std::thread::scope(|scope| {
        let one = scope.spawn(|| install(&prefix, &a, &d, true));
        let two = scope.spawn(|| install(&prefix, &a, &d, true));
        [one.join().unwrap(), two.join().unwrap()]
    });
    assert!(outcomes.iter().any(Result::is_ok));
    for error in outcomes.into_iter().filter_map(Result::err) {
        assert_eq!(error.code, "runtime_busy");
    }
    assert_eq!(
        install(&prefix, &a, &d, true).unwrap().outcome,
        "already-installed"
    );
}
#[test]
fn native_cli_rejects_closed_grammar_before_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let prefix = temp.path().join("absent");
    for arguments in [
        vec!["install", "--activate", "--activate"],
        vec!["list", "--project", "/any"],
        vec!["select", "latest"],
        vec!["install", "--sha256", "A"],
        vec!["list", "--prefix", "relative"],
        vec!["list", "--unknown"],
    ] {
        let mut args = arguments.into_iter().map(str::to_owned).collect::<Vec<_>>();
        args.extend(["--prefix".to_owned(), prefix.display().to_string()]);
        assert!(super::super::cli::execute_runtime(&args).is_err());
        assert!(!prefix.exists());
    }
}

#[test]
fn native_first_install_race_and_input_snapshot_identity() {
    let temp = tempfile::tempdir().unwrap();
    let prefix = temp.path().join("new prefix");
    let (archive, digest) = archive(temp.path(), "v0.1.34");
    let barrier = std::sync::Barrier::new(2);
    let outcomes = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            barrier.wait();
            install(&prefix, &archive, &digest, true)
        });
        let b = scope.spawn(|| {
            barrier.wait();
            install(&prefix, &archive, &digest, true)
        });
        [a.join().unwrap(), b.join().unwrap()]
    });
    assert!(outcomes.iter().any(Result::is_ok), "{outcomes:?}");
    for error in outcomes.into_iter().filter_map(Result::err) {
        assert_eq!(error.code, "runtime_busy");
    }
    assert_eq!(list(&prefix).unwrap().versions.len(), 1);
    struct ReplaceInput(PathBuf);
    impl Checkpoints for ReplaceInput {
        fn at(&self, point: &'static str) -> Result<(), Diagnostic> {
            if point == "snapshot-complete" {
                fs::rename(&self.0, self.0.with_extension("retained")).unwrap();
                fs::write(&self.0, b"replaced after snapshot").unwrap();
            }
            Ok(())
        }
    }
    install_with(
        &temp.path().join("snapshot prefix"),
        &archive,
        &digest,
        true,
        &ReplaceInput(archive.clone()),
    )
    .unwrap();
    assert_eq!(fs::read(&archive).unwrap(), b"replaced after snapshot");
}

#[test]
fn native_namespace_substitution_cannot_write_through_a_symlink() {
    let temp = tempfile::tempdir().unwrap();
    let prefix = temp.path().join("prefix");
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("sentinel"), b"untouched").unwrap();
    let (a, d) = archive(temp.path(), "v0.1.34");
    struct Swap(PathBuf, PathBuf);
    impl Checkpoints for Swap {
        fn at(&self, point: &'static str) -> Result<(), Diagnostic> {
            if point == "before-version-sync" {
                fs::rename(self.0.join("bin"), self.0.join("original-bin")).unwrap();
                symlink(&self.1, self.0.join("bin")).unwrap();
            }
            Ok(())
        }
    }
    assert!(
        install_with(
            &prefix,
            &a,
            &d,
            true,
            &Swap(prefix.clone(), outside.clone())
        )
        .is_err()
    );
    assert_eq!(fs::read(outside.join("sentinel")).unwrap(), b"untouched");
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
    fs::remove_file(prefix.join("bin")).unwrap();
    fs::rename(prefix.join("original-bin"), prefix.join("bin")).unwrap();
    install(&prefix, &a, &d, true).unwrap();
}

#[test]
#[ignore = "subprocess fixture; native_process_interruption_releases_lock_and_recovers invokes every cut"]
fn native_crash_child() {
    struct Kill(String);
    impl Checkpoints for Kill {
        fn at(&self, point: &'static str) -> Result<(), Diagnostic> {
            if point == self.0 {
                rustix::process::kill_process(
                    rustix::process::getpid(),
                    rustix::process::Signal::KILL,
                )
                .unwrap();
            }
            Ok(())
        }
    }
    let prefix = std::env::var("LKJSCRIPT_TEST_INSTALL_PREFIX").unwrap();
    let archive = std::env::var("LKJSCRIPT_TEST_INSTALL_ARCHIVE").unwrap();
    let digest = std::env::var("LKJSCRIPT_TEST_INSTALL_DIGEST").unwrap();
    let point = std::env::var("LKJSCRIPT_TEST_INSTALL_CUT").unwrap();
    install_with(
        Path::new(&prefix),
        Path::new(&archive),
        &digest,
        true,
        &Kill(point),
    )
    .unwrap();
    panic!("interruption cut was not reached");
}

#[test]
fn native_process_interruption_releases_lock_and_recovers() {
    use std::os::unix::process::ExitStatusExt;
    for point in [
        "before-root-publish",
        "after-root-publish",
        "snapshot-complete",
        "after-version-publish",
        "after-pointer-replace",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let prefix = temp.path().join("prefix");
        let (a, d) = archive(temp.path(), "v0.1.34");
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "platform::installation::tests::native_crash_child",
                "--ignored",
            ])
            .env("LKJSCRIPT_TEST_INSTALL_PREFIX", &prefix)
            .env("LKJSCRIPT_TEST_INSTALL_ARCHIVE", &a)
            .env("LKJSCRIPT_TEST_INSTALL_DIGEST", &d)
            .env("LKJSCRIPT_TEST_INSTALL_CUT", point)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.signal(), Some(9), "{point}");
        // The OS lock is released by death. The actual public grammar recovers the retained state.
        let command = [
            "install",
            "--archive",
            a.to_str().unwrap(),
            "--sha256",
            &d,
            "--prefix",
            prefix.to_str().unwrap(),
            "--activate",
        ]
        .map(str::to_owned);
        super::super::cli::execute_runtime(&command).unwrap();
        assert_eq!(list(&prefix).unwrap().selected.as_deref(), Some("v0.1.34"));
    }
}

#[test]
fn native_special_inputs_and_same_length_payload_corruption_reject() {
    let temporary = tempfile::tempdir().unwrap();
    let (archive, digest) = archive(temporary.path(), "v0.1.34");
    let prefix = temporary.path().join("prefix");
    let linked = temporary.path().join("linked");
    fs::hard_link(&archive, &linked).unwrap();
    assert!(install(&prefix, &linked, &digest, true).is_err());
    fs::remove_file(&linked).unwrap();
    let socket = temporary.path().join("socket");
    let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    assert!(install(&prefix, &socket, &digest, true).is_err());
    assert!(install(&prefix, temporary.path(), &digest, true).is_err());
    assert!(!prefix.exists());
    let installed = install(&prefix, &archive, &digest, true).unwrap();
    let mut payload = fs::read(&installed.version.path).unwrap();
    payload[0] ^= 1;
    fs::write(&installed.version.path, &payload).unwrap();
    // Inventory does not claim an unperformed rehash. Selection and reinstallation must detect it.
    assert_eq!(list(&prefix).unwrap().versions[0].integrity, "unchecked");
    assert!(select(&prefix, "v0.1.34").is_err());
    assert!(install(&prefix, &archive, &digest, true).is_err());
    assert_eq!(fs::read(&installed.version.path).unwrap(), payload);
}

#[test]
fn native_primary_and_cleanup_failures_are_retained_without_deleting_foreign_stage_content() {
    let temporary = tempfile::tempdir().unwrap();
    let prefix = temporary.path().join("prefix");
    let (archive, digest) = archive(temporary.path(), "v0.1.34");
    struct CleanupFault(PathBuf);
    impl Checkpoints for CleanupFault {
        fn at(&self, point: &'static str) -> Result<(), Diagnostic> {
            if point == "snapshot-complete" {
                let stage = fs::read_dir(self.0.join("lib/lkjscript"))
                    .unwrap()
                    .map(Result::unwrap)
                    .find(|e| e.file_name().to_string_lossy().starts_with(".stage-"))
                    .unwrap();
                fs::write(stage.path().join("foreign-sentinel"), b"preserve").unwrap();
                return Err(io_error("primary write failure"));
            }
            Ok(())
        }
    }
    let error = install_with(
        &prefix,
        &archive,
        &digest,
        true,
        &CleanupFault(prefix.clone()),
    )
    .unwrap_err();
    assert!(error.message.contains("primary write failure"));
    assert!(error.notes.iter().any(|note| note.contains("cleanup")));
    let stage = fs::read_dir(prefix.join("lib/lkjscript"))
        .unwrap()
        .map(Result::unwrap)
        .find(|e| e.file_name().to_string_lossy().starts_with(".stage-"))
        .unwrap()
        .path();
    assert!(
        error
            .notes
            .iter()
            .any(|note| note.contains(stage.to_str().unwrap()))
    );
    assert_eq!(
        fs::read(stage.join("foreign-sentinel")).unwrap(),
        b"preserve"
    );
    let retry = install(&prefix, &archive, &digest, true).unwrap_err();
    assert!(
        retry
            .notes
            .iter()
            .any(|note| note.contains(stage.to_str().unwrap()))
    );
    fs::remove_file(stage.join("foreign-sentinel")).unwrap();
    install(&prefix, &archive, &digest, true).unwrap();
}

#[test]
fn native_cleanup_lookup_failure_preserves_primary_failure_and_recovery_location() {
    struct LookupFault {
        point: &'static str,
        directory: PathBuf,
    }
    impl Checkpoints for LookupFault {
        fn at(&self, point: &'static str) -> Result<(), Diagnostic> {
            if point == self.point {
                fs::set_permissions(&self.directory, fs::Permissions::from_mode(0o000)).unwrap();
                return Err(io_error("primary publication failure"));
            }
            Ok(())
        }
    }
    for (point, directory) in [
        ("before-root-publish", "lib"),
        ("before-pointer-replace", "bin"),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let prefix = temporary.path().join("prefix");
        let (archive, digest) = archive(temporary.path(), "v0.1.34");
        let directory = prefix.join(directory);
        let error = install_with(
            &prefix,
            &archive,
            &digest,
            true,
            &LookupFault {
                point,
                directory: directory.clone(),
            },
        )
        .unwrap_err();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(
            error.message.contains("primary publication failure"),
            "{point}: {error:?}"
        );
        assert!(
            error
                .notes
                .iter()
                .any(|note| note.contains(directory.to_str().unwrap())),
            "{point}: {error:?}"
        );
        install(&prefix, &archive, &digest, true).unwrap();
        assert_eq!(list(&prefix).unwrap().selected.as_deref(), Some("v0.1.34"));
    }
}
