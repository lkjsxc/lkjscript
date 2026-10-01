//! Test-only executable adapters are written by a joined child, not the test parent.
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub(super) fn write_adapter(destination: &Path, bytes: &[u8]) {
    let parent = destination.parent().expect("owned adapter parent");
    let mut source = tempfile::NamedTempFile::new_in(parent).expect("owned adapter source");
    source.write_all(bytes).expect("complete adapter bytes");
    // Keep this staging writer open deliberately: only the child writes the
    // executable inode, so sibling test forks cannot inherit its writable fd.
    install_adapter(source.path(), destination);
}

fn install_adapter(source: &Path, destination: &Path) {
    let result = Command::new("/usr/bin/install")
        .args(["-m", "755", "--"])
        .arg(source)
        .arg(destination)
        .output()
        .expect("join independent adapter writer");
    assert!(
        result.status.success(),
        "adapter writer failed: status={}, stdout={:?}, stderr={:?}",
        result.status,
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read(destination).expect("written adapter"),
        fs::read(source).expect("adapter source"),
        "joined writer must preserve the exact independent adapter"
    );
}

#[test]
#[cfg(target_os = "linux")]
fn isolated_adapter_writer_does_not_share_the_executed_inode_with_a_held_writer() {
    use std::os::unix::fs::PermissionsExt;
    let temporary = tempfile::tempdir().expect("owned adapter control");
    let mut source = tempfile::NamedTempFile::new_in(temporary.path()).expect("held source");
    source
        .write_all(b"#!/bin/sh\nprintf 'once\\n' >> invocations\nprintf 'fixture\\n'\n")
        .expect("control adapter bytes");
    source
        .as_file()
        .set_permissions(fs::Permissions::from_mode(0o755))
        .expect("direct control mode");
    let error = Command::new(source.path())
        .current_dir(temporary.path())
        .output()
        .expect_err("held executable writer must prevent launch");
    assert_eq!(error.raw_os_error(), Some(26), "positive ETXTBSY control");
    assert!(!temporary.path().join("invocations").exists());

    let executable = temporary.path().join("independent-adapter");
    install_adapter(source.path(), &executable);
    let output = Command::new(&executable)
        .current_dir(temporary.path())
        .output()
        .expect("launch independently written adapter with staging writer still held");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"fixture\n");
    assert_eq!(
        fs::read(temporary.path().join("invocations")).expect("actual invocation"),
        b"once\n"
    );
    assert_eq!(
        fs::read(executable).expect("adapter bytes after execution"),
        fs::read(source.path()).expect("unchanged staging source")
    );
}
