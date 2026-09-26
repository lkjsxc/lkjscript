use super::*;
use std::sync::{Arc, Barrier};

#[cfg(unix)]
#[test]
fn private_publication_checks_access_without_changing_existing_files() {
    use std::os::unix::fs::PermissionsExt;
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("private.json");
    output::inspect_private_exact(&path, b"private", 7).unwrap();
    let created = output::publish_private_exact(&path, b"private", 7).unwrap();
    assert_eq!(created.visibility, "created");
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
    assert_eq!(
        output::publish_private_exact(&path, b"private", 7)
            .unwrap()
            .visibility,
        "reused-exact"
    );
    for mode in [0o640, 0o604, 0o620, 0o601] {
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        assert_eq!(
            output::inspect_private_exact(&path, b"private", 7)
                .unwrap_err()
                .code,
            "deployment_build_permissions"
        );
        assert_eq!(
            output::publish_private_exact(&path, b"private", 7)
                .unwrap_err()
                .code,
            "deployment_build_permissions"
        );
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            mode
        );
        assert_eq!(fs::read(&path).unwrap(), b"private");
        output::inspect_exact(&path, b"private", 7).unwrap();
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    output::publish_private_exact(&path, b"private", 7).unwrap();
    assert_eq!(
        output::publish_private_exact(&path, b"changed", 7)
            .unwrap_err()
            .code,
        "deployment_build_conflict"
    );
    assert_eq!(fs::read(&path).unwrap(), b"private");
    assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 1);
}

#[test]
fn output_and_data_roots_are_separate_by_path_components() {
    let root = Path::new("/owned/data");
    for (artifact, deployment) in [
        ("/owned/data/build.lkja", "/owned/run.json"),
        ("/owned/generated/build.lkja", "/owned/data/run.json"),
        ("/owned/data", "/owned/run.json"),
    ] {
        assert_eq!(
            reject_data_overlap(root, Path::new(artifact), Path::new(deployment))
                .unwrap_err()
                .code,
            "deployment_build_data_overlap"
        );
    }
    reject_data_overlap(
        root,
        Path::new("/owned/data-archive/build.lkja"),
        Path::new("/owned/run.json"),
    )
    .unwrap();
}

#[test]
fn exact_reuse_checks_bytes_and_preserves_conflicting_outputs() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("build-test.lkja");
    output::inspect_exact(&path, b"complete", 8).unwrap();
    assert!(!path.exists());
    let created = output::publish_exact(&path, b"complete", 8).unwrap();
    assert_eq!(created.visibility, "created");
    let reused = output::publish_exact(&path, b"complete", 8).unwrap();
    assert_eq!(reused.visibility, "reused-exact");
    assert_eq!(reused.durability, "synchronized");
    for bytes in [b"replaced".as_slice(), b"short", b"complete-extra"] {
        assert_eq!(
            output::inspect_exact(&path, bytes, 32).unwrap_err().code,
            "deployment_build_conflict"
        );
        assert_eq!(
            output::publish_exact(&path, bytes, 32).unwrap_err().code,
            "deployment_build_conflict"
        );
        assert_eq!(fs::read(&path).unwrap(), b"complete");
    }
    assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 1);
}

#[test]
fn concurrent_identical_publication_creates_one_complete_file() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("shared.lkja");
    let barrier = Arc::new(Barrier::new(8));
    let handles = (0..8)
        .map(|_| {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                output::inspect_exact(&path, b"immutable", 9).unwrap();
                barrier.wait();
                output::publish_exact(&path, b"immutable", 9).unwrap()
            })
        })
        .collect::<Vec<_>>();
    let receipts = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| receipt.visibility == "created")
            .count(),
        1
    );
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| receipt.visibility == "reused-exact")
            .count(),
        7
    );
    assert_eq!(fs::read(&path).unwrap(), b"immutable");
    assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 1);
}

#[test]
fn byte_limits_apply_before_creation_and_during_input_reading() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("bounded");
    assert_eq!(
        output::inspect_exact(&path, b"1234", 3).unwrap_err().code,
        "output_byte_limit"
    );
    assert_eq!(
        output::publish_exact(&path, b"1234", 3).unwrap_err().code,
        "output_byte_limit"
    );
    assert!(!path.exists());
    fs::write(&path, b"1234").unwrap();
    assert_eq!(output::read_regular(&path, 4).unwrap(), b"1234");
    assert_eq!(
        output::read_regular(&path, 3).unwrap_err().code,
        "deployment_input_limit"
    );
    assert!(output::read_regular(&path, usize::MAX).is_err());
    fs::write(&path, b"").unwrap();
    assert_eq!(output::read_regular(&path, 0).unwrap(), b"");
}

#[cfg(unix)]
#[test]
fn nonregular_and_symlink_outputs_are_not_reused_or_followed() {
    use std::os::unix::fs::symlink;
    let temporary = tempfile::tempdir().unwrap();
    let target = temporary.path().join("target");
    fs::write(&target, b"same").unwrap();
    let link = temporary.path().join("link");
    symlink(&target, &link).unwrap();
    let dangling = temporary.path().join("dangling");
    symlink(temporary.path().join("absent"), &dangling).unwrap();
    let directory = temporary.path().join("directory");
    fs::create_dir(&directory).unwrap();
    let fifo = temporary.path().join("fifo");
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        &fifo,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .unwrap();
    for path in [&link, &dangling, &directory, &fifo] {
        assert!(output::read_regular(path, 4).is_err());
        assert!(output::inspect_exact(path, b"same", 4).is_err());
        assert!(output::publish_exact(path, b"same", 4).is_err());
        assert!(output::inspect_private_exact(path, b"same", 4).is_err());
        assert!(output::publish_private_exact(path, b"same", 4).is_err());
    }
    assert_eq!(fs::read(&target).unwrap(), b"same");
    assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 5);
}

#[cfg(unix)]
#[test]
fn parent_admission_prevents_reuse_through_links_or_traversal() {
    use std::os::unix::fs::symlink;
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path().join("real");
    fs::create_dir(&directory).unwrap();
    fs::write(directory.join("existing"), b"same").unwrap();
    let link = temporary.path().join("link");
    symlink(&directory, &link).unwrap();
    for path in [
        link.join("existing"),
        directory.join("../real/existing"),
        temporary.path().join("missing/new"),
    ] {
        assert!(output::inspect_exact(&path, b"same", 4).is_err());
        assert!(output::publish_exact(&path, b"same", 4).is_err());
    }
    assert_eq!(fs::read(directory.join("existing")).unwrap(), b"same");
}

#[test]
fn strict_template_read_does_not_need_its_old_artifact() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("command.json");
    let descriptor = super::super::starter_command_deployment();
    let bytes = super::super::encode_deployment(&descriptor).unwrap();
    fs::write(&path, &bytes).unwrap();
    let template = BuildTemplate::read(&path).unwrap();
    assert_eq!(template.directory, temporary.path());
    assert_eq!(template.source, path);
    assert!(template.value.get("runtime").is_none());
    assert!(template.value.get("execution").is_none());
    assert!(!temporary.path().join(&descriptor.artifact).exists());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 1);
}

#[test]
fn malformed_or_excessive_templates_reject_before_output_creation() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("command.json");
    let bytes =
        super::super::encode_deployment(&super::super::starter_command_deployment()).unwrap();
    let source = String::from_utf8(bytes).unwrap();
    for invalid in [
        source.replacen('{', "{\"artifact\":\"other.lkja\",", 1),
        source.replacen('{', "{\"unknown\":null,", 1),
        source.replace("generated/application.lkja", "../outside.lkja"),
        " ".repeat(MAXIMUM_DEPLOYMENT_BYTES + 1),
        "null".to_owned(),
    ] {
        fs::write(&path, &invalid).unwrap();
        assert!(BuildTemplate::read(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), invalid.as_bytes());
        assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 1);
    }
    assert!(BuildTemplate::read(Path::new("")).is_err());
    assert!(BuildTemplate::read(Path::new(&"x".repeat(4097))).is_err());
}
