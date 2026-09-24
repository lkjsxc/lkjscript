use super::*;
use crate::check::cache::VerificationCache;
use crate::check::executor::{self, ExecutionOptions};
use crate::check::model::{
    CacheLookupStatus, ExecutionKind, Gate, GateReceipt, GateStatus, InputSnapshot,
    PlatformIdentity, RuntimeIdentity,
};
use crate::check::{registry::GateRegistry, snapshot};
use crate::evidence::VerificationDigest;
use crate::process::{self, ProcessSpec, ProcessStatus};
use std::collections::BTreeMap;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::process::{Command, Stdio};
use std::time::Duration;

fn script(path: &Path, exit: u8) {
    fs::write(path, format!("#!/bin/sh\nexit {exit}\n")).expect("owned executable fixture");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("executable mode");
}

#[test]
fn unchanged_link_observes_same_length_target_replacement_and_mode() {
    let root = tempfile::tempdir().expect("owned executable directory");
    let target = root.path().join("payload");
    let alias = root.path().join("alias");
    script(&target, 0);
    symlink("payload", &alias).expect("relative executable alias");
    let before = proof(root.path(), "./alias").expect("initial linked proof");
    assert_eq!(before.entry.kind, FileKind::Symlink);
    assert_eq!(before.entry.link_target.as_deref(), Some("payload"));
    let expected = evidence::proof(&target, before.entry.path.clone()).expect("leaf oracle");
    assert_eq!(before.resolved.as_ref(), Some(&expected));

    script(&target, 7);
    let changed = proof(root.path(), "./alias").expect("changed linked proof");
    // This is the predecessor's negative control: its entire observation is unchanged.
    assert_eq!(before.entry, changed.entry);
    assert_eq!(
        before.resolved.as_ref().unwrap().bytes,
        changed.resolved.as_ref().unwrap().bytes
    );
    assert_ne!(before.resolved, changed.resolved);
    let plain = evidence::proof(&alias, before.entry.path.clone()).expect("ordinary source link");
    assert_eq!(plain, before.entry, "source symlink semantics must not change");

    fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).expect("change leaf mode");
    let mode = proof(root.path(), "./alias").expect("new leaf mode");
    assert_eq!(mode.entry, changed.entry);
    assert_eq!(
        mode.resolved.as_ref().unwrap().digest,
        changed.resolved.as_ref().unwrap().digest
    );
    assert_ne!(mode.resolved, changed.resolved);
}

#[test]
fn link_chains_and_retargeting_preserve_both_observation_roles() {
    let root = tempfile::tempdir().expect("owned link chain");
    script(&root.path().join("payload"), 0);
    script(&root.path().join("replacement"), 0);
    symlink("payload", root.path().join("middle")).expect("middle link");
    symlink("middle", root.path().join("alias")).expect("entry link");
    let mut before = proof(root.path(), "./alias").expect("complete chain");
    before.relabel("$TOOL");
    assert_eq!(before.resolved.as_ref().unwrap().kind, FileKind::File);
    fs::remove_file(root.path().join("alias")).expect("replace only owned link");
    symlink("replacement", root.path().join("alias")).expect("new entry target");
    let mut after = proof(root.path(), "./alias").expect("retargeted chain");
    after.relabel("$TOOL");
    assert_ne!(before.entry, after.entry);
    assert_eq!(before.resolved, after.resolved, "identical leaf bytes and modes");
}

#[test]
fn unresolved_or_nonregular_link_targets_fail_without_inventing_file_evidence() {
    let root = tempfile::tempdir().expect("owned broken links");
    symlink("absent", root.path().join("dangling")).expect("dangling link");
    symlink("cycle", root.path().join("cycle")).expect("cyclic link");
    fs::create_dir(root.path().join("directory")).expect("non-file target");
    symlink("directory", root.path().join("directory-link")).expect("directory link");
    for command in ["./dangling", "./cycle", "./directory-link"] {
        assert!(proof(root.path(), command).is_err(), "accepted {command}");
    }
    let absent = proof(root.path(), "./absent").expect("ordinary absent producer output");
    assert_eq!(absent.entry.kind, FileKind::Missing);
    assert!(absent.resolved.is_none());
}

#[test]
fn logical_labels_preserve_reuse_across_immutable_copies() {
    let root = tempfile::tempdir().expect("owned copies");
    let mut linked = Vec::new();
    let mut plain = Vec::new();
    for directory in ["one", "two"] {
        let directory = root.path().join(directory);
        fs::create_dir(&directory).expect("copy directory");
        script(&directory.join("payload"), 0);
        symlink("payload", directory.join("alias")).expect("same relative alias");
        let mut link = proof(&directory, "./alias").expect("copy link");
        link.relabel("$TOOL");
        assert_eq!(link.entry.path, "$TOOL");
        assert_eq!(link.resolved.as_ref().unwrap().path, "$TOOL");
        linked.push(link);
        let mut file = proof(&directory, "./payload").expect("copy file");
        file.relabel("$TOOL");
        assert!(file.resolved.is_none(), "plain files need no duplicate proof");
        plain.push(file);
    }
    assert_eq!(linked[0], linked[1]);
    assert_eq!(plain[0], plain[1]);
}

#[test]
fn relative_empty_and_nonexecutable_path_entries_match_actual_linux_execution() {
    let root = tempfile::tempdir().expect("owned PATH fixture");
    for directory in ["first", "second"] {
        fs::create_dir(root.path().join(directory)).expect("PATH directory");
    }
    script(&root.path().join("first/probe"), 11);
    fs::set_permissions(
        root.path().join("first/probe"),
        fs::Permissions::from_mode(0o644),
    )
    .expect("nonexecutable earlier candidate");
    script(&root.path().join("second/probe"), 37);
    script(&root.path().join("probe"), 19);
    for (path, leaf, expected) in [
        ("first:second", "second/probe", 37),
        ("", "probe", 19),
        (":first", "probe", 19),
    ] {
        let observed = proof_with_path(root.path(), "probe", Some(OsStr::new(path)))
            .expect("explicit PATH observation");
        let expected_file = evidence::proof(&root.path().join(leaf), observed.entry.path.clone())
            .expect("selected file oracle");
        assert_eq!(observed.entry, expected_file);
        let status = Command::new("probe")
            .current_dir(root.path())
            .env_clear()
            .env("PATH", path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("independent real PATH execution");
        assert_eq!(status.code(), Some(expected));
    }
    assert!(proof_with_path(root.path(), "probe", None).is_err());
    assert!(proof_with_path(root.path(), "./probe", None).is_ok());
}

fn fixed_inputs(root: &Path) -> (InputSnapshot, RuntimeIdentity) {
    let snapshot = InputSnapshot {
        digest: VerificationDigest::of(b"fixed input outside the executable fixture"),
        git_head: "fixture".to_owned(),
        cargo_lock_digest: VerificationDigest::of(b"fixed lock"),
        file_count: 0,
        total_bytes: 0,
        entries: Vec::new(),
    };
    let runtime = RuntimeIdentity {
        digest: VerificationDigest::of(b"fixed runtime isolates the per-gate executable key"),
        rustc: "fixture".to_owned(),
        cargo: "fixture".to_owned(),
        platform: PlatformIdentity {
            operating_system: "linux".to_owned(),
            architecture: env::consts::ARCH.to_owned(),
            family: "unix".to_owned(),
            child_process_control: "linux_process_group_sigkill".to_owned(),
        },
        environment_digest: VerificationDigest::of(b"fixed environment"),
        environment_names: Vec::new(),
        harness: evidence::proof(&root.join("payload"), "$FIXTURE".to_owned()).unwrap(),
        command_executables: BTreeMap::new(),
    };
    (snapshot, runtime)
}

fn cached_gate(
    root: &Path,
    run: &str,
    gate: &Gate,
    cache: &VerificationCache,
    snapshot: &InputSnapshot,
    runtime: &RuntimeIdentity,
) -> GateReceipt {
    let directory = root.join(run);
    fs::create_dir(&directory).expect("fresh gate log directory");
    let registry = GateRegistry::new(vec![gate.clone()]).expect("one real gate");
    executor::execute_dag(
        &registry,
        std::slice::from_ref(&gate.name),
        &ExecutionOptions {
            repository: root,
            run_directory: &directory,
            snapshot,
            runtime,
            maximum_workers: 1,
            allow_reuse: true,
            fresh_reason: "executable_fixture",
            cache,
        },
    )
    .expect("bounded real gate execution")
    .pop()
    .expect("one receipt")
}

#[test]
fn a_changed_symlink_target_cannot_reuse_a_previous_successful_gate() {
    let root = tempfile::tempdir().expect("owned cache regression");
    let target = root.path().join("payload");
    script(&target, 0);
    symlink("payload", root.path().join("alias")).expect("stable executable alias");
    let mut gate = Gate::new(
        "linked",
        vec![root.path().join("alias").display().to_string()],
    );
    gate.identity_command = Some(vec!["$TOOL".to_owned()]);
    gate.timeout = Duration::from_secs(2);
    let (snapshot, runtime) = fixed_inputs(root.path());
    let cache = VerificationCache::new(root.path(), &root.path().join("cache"));
    let first = cached_gate(root.path(), "first", &gate, &cache, &snapshot, &runtime);
    assert_eq!(first.status, GateStatus::Passed);
    assert_eq!(first.execution, ExecutionKind::Fresh);
    cache.store(&gate, &first).expect("retain genuine success");
    let second = cached_gate(root.path(), "second", &gate, &cache, &snapshot, &runtime);
    assert_eq!(second.status, GateStatus::Passed);
    assert_eq!(second.execution, ExecutionKind::Reused);
    assert_eq!(second.input_fingerprint, first.input_fingerprint);

    // A predecessor cache record must not be accepted even at a new key.
    let record = cache.record_path(&gate, &first.input_fingerprint).unwrap();
    let mut old: serde_json::Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    old["cache_contract_version"] = serde_json::json!(2);
    fs::write(&record, serde_json::to_vec(&old).unwrap()).unwrap();
    let rejected = cache.load(
        &gate,
        &first.input_fingerprint,
        &root.path().join("old.stdout"),
        &root.path().join("old.stderr"),
    );
    assert!(rejected.cached.is_none());
    cache.store(&gate, &first).expect("restore original current cache record");

    let original_link = proof(root.path(), "./alias").unwrap().entry;
    script(&target, 7);
    assert_eq!(proof(root.path(), "./alias").unwrap().entry, original_link);
    let failed = cached_gate(root.path(), "changed", &gate, &cache, &snapshot, &runtime);
    assert_eq!(failed.execution, ExecutionKind::Fresh);
    assert_eq!(failed.cache.lookup, CacheLookupStatus::Miss);
    assert_ne!(failed.input_fingerprint, first.input_fingerprint);
    assert_eq!(failed.status, GateStatus::Failed);
    assert_eq!(failed.process.as_ref().unwrap().exit_code, Some(7));

    script(&target, 0);
    let restored = cached_gate(root.path(), "restored", &gate, &cache, &snapshot, &runtime);
    assert_eq!(restored.input_fingerprint, first.input_fingerprint);
    assert_eq!(restored.execution, ExecutionKind::Reused);
    assert_eq!(restored.status, GateStatus::Passed);
}

#[test]
fn runtime_reader_rejects_changed_target_and_recovers_with_original_bytes() {
    let root = tempfile::tempdir().expect("owned runtime reader regression");
    let target = root.path().join("payload");
    let alias = root.path().join("alias");
    script(&target, 0);
    symlink("payload", &alias).expect("runtime alias");
    let commands = vec![(alias.display().to_string(), "$TOOL".to_owned())];
    let original = snapshot::runtime_identity(root.path(), commands.clone()).unwrap();
    let copy = root.path().join("verifier");
    fs::copy(env::current_exe().unwrap(), &copy).expect("original verifier copy");
    snapshot::validate_runtime(root.path(), &original, &copy, commands.clone())
        .expect("unchanged runtime");
    let selected = original.command_executables.get("$TOOL").unwrap();
    assert!(selected.resolved.is_some());
    let old_shape = serde_json::to_value(&selected.entry).unwrap();
    assert!(serde_json::from_value::<ExecutableProof>(old_shape).is_err());

    script(&target, 7);
    let changed = snapshot::runtime_identity(root.path(), commands.clone()).unwrap();
    assert_eq!(selected.entry, changed.command_executables["$TOOL"].entry);
    assert_ne!(original.digest, changed.digest);
    assert!(snapshot::validate_runtime(root.path(), &original, &copy, commands.clone()).is_err());
    script(&target, 0);
    snapshot::validate_runtime(root.path(), &original, &copy, commands)
        .expect("restored exact target and verifier");
}

#[test]
fn a_missing_interpreter_cannot_switch_execution_to_an_unobserved_path_candidate() {
    let root = tempfile::tempdir().expect("owned interpreter fallback regression");
    for directory in ["first", "second"] {
        fs::create_dir(root.path().join(directory)).unwrap();
    }
    let first = root.path().join("first/probe");
    let second = root.path().join("second/probe");
    let marker = root.path().join("unchecked");
    fs::write(
        &first,
        format!("#!{}\nexit 0\n", root.path().join("absent-interpreter").display()),
    )
    .unwrap();
    fs::set_permissions(&first, fs::Permissions::from_mode(0o755)).unwrap();
    let path = OsStr::new("first:second");
    let original = observe_with_path(root.path(), "probe", Some(path)).unwrap();
    assert_eq!(original.path.as_deref(), Some(first.as_path()));
    for exit in [37, 7] {
        fs::write(&second, format!("#!/bin/sh\nprintf visited > unchecked\nexit {exit}\n"))
            .unwrap();
        fs::set_permissions(&second, fs::Permissions::from_mode(0o755)).unwrap();
        let observed = observe_with_path(root.path(), "probe", Some(path)).unwrap();
        assert_eq!(observed.proof, original.proof);
        // Independent raw execution demonstrates why a second PATH search is unsound.
        let raw = Command::new("probe")
            .current_dir(root.path())
            .env_clear()
            .env("PATH", path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert_eq!(raw.code(), Some(exit));
        assert_eq!(fs::read(&marker).unwrap(), b"visited");
        fs::remove_file(&marker).unwrap();
        let spec = ProcessSpec {
            command: vec!["probe".to_owned()],
            cwd: root.path().to_path_buf(),
            environment: BTreeMap::from([("PATH".to_owned(), "first:second".to_owned())]),
            timeout: Duration::from_secs(2),
            maximum_stdout_bytes: 1_024,
            maximum_stderr_bytes: 1_024,
            stdout_path: root.path().join(format!("bound-{exit}.stdout")),
            stderr_path: root.path().join(format!("bound-{exit}.stderr")),
            unavailable_exit_code: None,
        };
        let result = process::run_selected(&spec, root.path(), observed.path.as_deref());
        assert_eq!(result.status, ProcessStatus::Unavailable);
        assert!(!marker.exists(), "unobserved fallback ran");
    }
}
