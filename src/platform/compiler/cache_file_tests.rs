//! Descriptor type admission must finish even when a cache entry has no FIFO writer.
#![allow(
    clippy::unwrap_used,
    reason = "test assertions retain their original failures"
)]

use super::*;
use std::os::unix::fs::{FileTypeExt, MetadataExt, symlink};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const ROOT: &str = "LKJSCRIPT_TEST_COMPILER_CACHE_FIFO_ROOT";

#[test]
fn regular_missing_and_nonregular_cache_entries_keep_exact_admission() {
    let root = tempfile::tempdir().unwrap();
    let directory = File::open(root.path()).unwrap();
    assert!(
        open_optional_regular(&directory, "missing")
            .unwrap()
            .is_none()
    );
    for (name, expected) in [
        ("empty", b"".as_slice()),
        ("regular", b"cache bytes".as_slice()),
    ] {
        std::fs::write(root.path().join(name), expected).unwrap();
        let mut file = open_optional_regular(&directory, name).unwrap().unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, expected);
    }
    std::fs::create_dir(root.path().join("directory")).unwrap();
    let error = open_optional_regular(&directory, "directory").unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Corrupt);
    assert_eq!(error.code, "compilation_cache_regular_type");
    symlink(root.path().join("regular"), root.path().join("link")).unwrap();
    symlink(root.path().join("missing"), root.path().join("dangling")).unwrap();
    for name in ["link", "dangling"] {
        let error = open_optional_regular(&directory, name).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Infrastructure);
        assert_eq!(error.code, "compilation_cache_regular_open");
        assert!(
            std::fs::symlink_metadata(root.path().join(name))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
    assert_eq!(
        std::fs::read(root.path().join("regular")).unwrap(),
        b"cache bytes"
    );
}

struct JoinedChild(Option<Child>);

impl Drop for JoinedChild {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn writerless_cache_fifo_rejects_in_a_bounded_joined_child() {
    let root = tempfile::tempdir().unwrap();
    let fifo = root.path().join("CURRENT");
    rustix::fs::mkfifoat(rustix::fs::CWD, &fifo, Mode::RUSR | Mode::WUSR).unwrap();
    let before = std::fs::symlink_metadata(&fifo).unwrap();
    assert!(before.file_type().is_fifo());
    let mut child = JoinedChild(Some(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "platform::compiler::cache::file_tests::cache_fifo_admission_child",
                "--nocapture",
            ])
            .env_clear()
            .env(ROOT, root.path())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    ));
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.0.as_mut().unwrap().try_wait().unwrap().is_none() {
        assert!(
            Instant::now() < deadline,
            "cache FIFO admission blocked before checking its type"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.0.take().unwrap().wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // A zero-test child, an unavailable executable or a timeout is never a passing probe.
    assert_eq!(
        std::fs::read(root.path().join("completed")).unwrap(),
        b"rejected"
    );
    let after = std::fs::symlink_metadata(&fifo).unwrap();
    assert!(after.file_type().is_fifo());
    assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
}

#[test]
fn cache_fifo_admission_child() {
    let Some(root) = std::env::var_os(ROOT) else {
        return;
    };
    let root = std::path::Path::new(&root);
    let directory = File::open(root).unwrap();
    let error = open_optional_regular(&directory, "CURRENT").unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Corrupt);
    assert_eq!(error.code, "compilation_cache_regular_type");
    std::fs::write(root.join("completed"), b"rejected").unwrap();
}

#[test]
fn foreign_profile_compilation_cache_rebuilds_without_changing_authority() {
    let source = crate::platform::kernel::tests::witness_snapshot();
    let directory = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&directory.path().join("repository"), &source, None)
        .unwrap()
        .repository;
    let before = repository.current().unwrap().head;
    let current = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let mut predecessor = current.manifest.clone();
    predecessor.compiler_contract_version ^= 1;
    let bytes = crate::platform::packed::encode_canonical(
        crate::platform::compiler::COMPILATION_MANIFEST_MAGIC,
        crate::platform::compiler::COMPILATION_MANIFEST_ENVELOPE_DOMAIN,
        &predecessor,
        MAXIMUM_COMPILATION_MANIFEST_BYTES,
    )
    .unwrap();
    let old = install_derived_manifest(&repository, &current.manifest, bytes);
    let error = load_current_compilation(&repository)
        .expect_err("a predecessor derived cache never becomes an exact-current hit");
    assert_eq!(error.class, DiagnosticClass::Source);
    assert_eq!(error.code, "compilation_manifest_contract");
    assert_eq!(
        load_exact_current_compilation(&repository, old)
            .unwrap_err()
            .code,
        "compilation_manifest_contract"
    );
    assert_eq!(
        crate::platform::compiler::link_artifact(&repository, old, &[])
            .unwrap_err()
            .code,
        "compilation_manifest_contract"
    );
    let rebuilt = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    assert_eq!(rebuilt.manifest_digest, current.manifest_digest);
    assert_eq!(rebuilt.units_reused, 0);
    assert_eq!(
        load_current_compilation(&repository)
            .unwrap()
            .unwrap()
            .digest,
        current.manifest_digest
    );
    assert_eq!(repository.current().unwrap().head, before);
}

#[test]
fn corrupt_foreign_profile_manifest_remains_corrupt_before_cache_recovery() {
    let source = crate::platform::kernel::tests::witness_snapshot();
    let directory = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&directory.path().join("repository"), &source, None)
        .unwrap()
        .repository;
    let before = repository.current().unwrap().head;
    let current = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let mut predecessor = current.manifest.clone();
    predecessor.compiler_contract_version ^= 1;
    let mut bytes = crate::platform::packed::encode_canonical(
        crate::platform::compiler::COMPILATION_MANIFEST_MAGIC,
        crate::platform::compiler::COMPILATION_MANIFEST_ENVELOPE_DOMAIN,
        &predecessor,
        MAXIMUM_COMPILATION_MANIFEST_BYTES,
    )
    .unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    install_derived_manifest(&repository, &current.manifest, bytes);
    let error = load_current_compilation(&repository)
        .expect_err("outdated contract metadata cannot hide corrupt envelope bytes");
    assert_eq!(error.class, DiagnosticClass::Corrupt);
    assert_eq!(error.code, "packed_checksum");
    assert_eq!(repository.current().unwrap().head, before);
}

/// Stage only disposable compiler metadata and its already admitted package revision.
fn install_derived_manifest(
    repository: &GraphRepository,
    current: &CompilationManifest,
    bytes: Vec<u8>,
) -> CompilationManifestDigest {
    let key = ObjectKey::for_bytes(ObjectDomain::CompilationManifest, &bytes);
    let digest = CompilationManifestDigest::from_bytes(key.digest.bytes());
    let revision_key = ObjectKey::from_digest(
        ObjectDomain::PackageRevision,
        current.package_revision.bytes(),
    );
    let revision_bytes = repository
        .object_store()
        .unwrap()
        .read(
            revision_key,
            ObjectDomain::PackageRevision.maximum_bytes(),
            &mut StoreWork::default(),
        )
        .unwrap()
        .unwrap();
    repository
        .stage_compilation_objects(
            repository.current().unwrap().head,
            &BTreeMap::from([(key, bytes), (revision_key, revision_bytes)]),
        )
        .unwrap();
    write_cache_head(
        repository,
        CompilationCacheHead {
            contract_version: CACHE_HEAD_CONTRACT_VERSION,
            repository_id: current.repository_id,
            revision: current.revision,
            manifest: digest,
        },
    )
    .unwrap();
    digest
}
