use super::*;
use crate::check::model::{CacheLookupStatus, CacheObservation, ExecutionKind, GateStatus};
use crate::evidence::VerificationDigest;
use serde_json::json;
use std::fs;
use std::os::unix::fs::PermissionsExt;

fn unit(root: &Path) -> Unit {
    let path = root.join("target/release/deps/public_cli-exact");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    Unit {
        package_id: "path+file:///fixture#lkjscript@0.0.0".into(),
        manifest_path: root.join("Cargo.toml").display().to_string(),
        target: json!({"name":"public_cli", "kind":["test"], "src_path":root.join("tests/public_cli.rs")}),
        profile: json!({"test":true, "opt_level":"3"}),
        features: Vec::new(),
        filenames: vec![evidence::proof(&path, path.display().to_string()).unwrap()],
        retained: Vec::new(),
        build_script: None,
        fresh: true,
    }
}

fn producer(origin: FileProof) -> GateReceipt {
    GateReceipt {
        name: "release_build".into(),
        status: GateStatus::Passed,
        execution: ExecutionKind::Fresh,
        command: vec!["cargo".into(), "build".into()],
        dependencies: Vec::new(),
        failed_dependencies: Vec::new(),
        started_unix_nanoseconds: 0,
        completed_unix_nanoseconds: 0,
        elapsed_nanoseconds: 0,
        process: None,
        inputs: Vec::new(),
        outputs: Vec::new(),
        retained_outputs: Vec::new(),
        cargo_origin: Some(origin),
        input_fingerprint: VerificationDigest::of(b"input"),
        evidence_digest: VerificationDigest::of(b"evidence"),
        cache: CacheObservation {
            eligible: false,
            lookup: CacheLookupStatus::Bypassed,
            reason: None,
            record: None,
            write: None,
        },
        reason: None,
        stdout_excerpt: None,
        stderr_excerpt: None,
    }
}

#[test]
fn test_inventory_requires_exact_root_release_target_and_rejects_ambiguous_outputs() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let original = unit(root);
    assert_eq!(
        inventory(root, std::slice::from_ref(&original))
            .unwrap()
            .len(),
        1
    );
    assert!(inventory(root, &[original.clone(), original.clone()]).is_err());
    for (pointer, value) in [
        ("/kind", json!(["bin"])),
        ("/src_path", json!(root.join("foreign/public_cli.rs"))),
    ] {
        let mut altered = original.clone();
        *altered.target.pointer_mut(pointer).unwrap() = value;
        if let Ok(tests) = inventory(root, &[altered]) {
            assert!(tests.is_empty());
        }
    }
    for field in ["test", "opt_level"] {
        let mut altered = original.clone();
        altered.profile[field] = Value::Null;
        assert!(inventory(root, &[altered]).unwrap().is_empty());
    }
    let mut altered = original.clone();
    altered.manifest_path = root.join("tools/foreign/Cargo.toml").display().to_string();
    assert!(inventory(root, &[altered]).unwrap().is_empty());
    for path in [
        root.join("target/debug/deps/public_cli-exact"),
        root.join("target/release/deps/foreign-exact"),
    ] {
        let mut altered = original.clone();
        altered.filenames[0].path = path.display().to_string();
        assert!(inventory(root, &[altered]).is_err());
    }
    let mut altered = original;
    altered.filenames[0].mode = Some(0o644);
    assert!(inventory(root, &[altered]).is_err());
}

#[test]
fn schema_attached_test_inventory_keeps_exact_executable_and_rejects_extra_artifacts() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let mut selected = unit(root);
    let executable = selected.filenames[0].clone();
    let out = root.join("target/release/build/lkjscript-bound/out");
    fs::create_dir_all(&out).unwrap();
    selected.build_script = Some(json!({"out_dir":out}));
    for (name, bytes) in lkjscript::platform::canonical_schema::generated_outputs() {
        let path = out.join(name);
        fs::write(&path, bytes).unwrap();
        selected
            .filenames
            .push(evidence::proof(&path, path.display().to_string()).unwrap());
    }
    let log = out.parent().unwrap().join("output");
    fs::write(&log, b"cargo:rerun-if-changed=build.rs\n").unwrap();
    selected
        .filenames
        .push(evidence::proof(&log, log.display().to_string()).unwrap());
    assert_eq!(
        inventory(root, std::slice::from_ref(&selected)).unwrap()["public_cli"].executable,
        executable,
    );
    let mut no_binding = selected.clone();
    no_binding.build_script = None;
    assert!(inventory(root, &[no_binding]).is_err());
    selected.filenames.push(executable);
    assert!(inventory(root, &[selected]).is_err());
}

#[test]
fn live_and_archived_test_selection_bind_producer_digest_without_copying_executables() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let selected = unit(root);
    let tests = inventory(root, std::slice::from_ref(&selected)).unwrap();
    let input = CargoTestInput {
        producer: "release_build".into(),
        target: "public_cli".into(),
    };
    let origin = root.join("producer.json");
    for value in [
        json!({"schema":"lkjscript-live-cargo-1", "test_executables":tests}),
        json!({"units":[selected]}),
    ] {
        fs::write(&origin, serde_json::to_vec(&value).unwrap()).unwrap();
        let proof = evidence::proof(&origin, "producer.json".into()).unwrap();
        let mut dependencies = BTreeMap::from([("release_build".into(), producer(proof))]);
        let selected = input_proof(root, &input, &dependencies).unwrap();
        assert_eq!(selected.path, "target/release/deps/public_cli-exact");
        assert_eq!(
            selected,
            evidence::proof(&root.join(&selected.path), selected.path.clone()).unwrap()
        );
        let mut gate = Gate::new(
            "lifecycle",
            vec![COMMAND.into(), LIFECYCLE.into(), "--exact".into()],
        );
        gate.cargo_test = Some(input.clone());
        assert_eq!(
            crate::check::executor::expected_input_proofs(root, &gate, &dependencies).unwrap(),
            [selected]
        );
        fs::write(&origin, b"{}\n").unwrap();
        assert!(input_proof(root, &input, &dependencies).is_err());
        fs::write(&origin, serde_json::to_vec(&value).unwrap()).unwrap();
        dependencies.get_mut("release_build").unwrap().status = GateStatus::Failed;
        assert!(input_proof(root, &input, &dependencies).is_err());
    }
    assert_eq!(
        fs::read_dir(root.join("target/release/deps"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn exact_lifecycle_requires_one_passing_test_and_rejects_empty_or_ignored_selection() {
    let temporary = tempfile::tempdir().unwrap();
    let stdout = temporary.path().join("stdout");
    let mut gate = Gate::new(
        "lifecycle",
        vec![COMMAND.into(), LIFECYCLE.into(), "--exact".into()],
    );
    gate.cargo_test = Some(CargoTestInput {
        producer: "release_build".into(),
        target: "public_cli".into(),
    });
    let passed = format!(
        "running 1 test\ntest {LIFECYCLE} ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 234 filtered out; finished in 0.01s\n"
    );
    fs::write(&stdout, &passed).unwrap();
    verify_result(temporary.path(), &gate, &stdout, &[]).unwrap();
    for rejected in [
        passed.replace("running 1 test", "running 0 tests"),
        passed.replace("1 passed;", "0 passed;"),
        passed.replace("0 ignored;", "1 ignored;"),
        passed.replace(LIFECYCLE, "another_test"),
    ] {
        fs::write(&stdout, rejected).unwrap();
        assert!(verify_result(temporary.path(), &gate, &stdout, &[]).is_err());
    }
}
