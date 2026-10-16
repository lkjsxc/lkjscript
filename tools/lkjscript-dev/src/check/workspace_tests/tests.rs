use super::*;
use serde_json::json;
use std::os::unix::fs::PermissionsExt;

fn units(root: &Path) -> Vec<Unit> {
    fs::create_dir_all(root.join("target/release/deps")).unwrap();
    TARGETS.iter().enumerate().map(|(index, target)| {
        let path = root.join(format!("target/release/deps/{}-{index}", target.name.replace('-', "_")));
        fs::write(&path, b"#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        Unit {
            package_id: format!("path+file:///fixture#{index}@0.0.0"),
            manifest_path: root.join(target.manifest).display().to_string(),
            target: json!({"name":target.name, "kind":[target.kind], "src_path":root.join(target.source)}),
            profile: json!({"test":true, "opt_level":"3"}),
            features: Vec::new(), filenames: vec![evidence::proof(&path, path.display().to_string()).unwrap()],
            retained: Vec::new(), build_script: None, fresh: true,
        }
    }).collect()
}

#[test]
fn complete_workspace_inventory_rejects_omission_substitution_and_unrecognized_targets() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let units = units(root);
    let selected = inventory(root, &units).unwrap();
    let live = json!({"schema":"lkjscript-live-cargo-1", "workspace_test_executables":selected});
    let archived = json!({"units":units});
    let expected = input_proofs(root, &live).unwrap();
    assert_eq!(expected.len(), TARGET_COUNT);
    assert_eq!(input_proofs(root, &archived).unwrap(), expected);
    for target in &TARGETS {
        let mut omitted = live.clone();
        omitted["workspace_test_executables"]
            .as_object_mut()
            .unwrap()
            .remove(target.source);
        assert!(input_proofs(root, &omitted).is_err());
    }
    let mut unknown = units.clone();
    unknown[0].target["src_path"] = json!(root.join("tests/new_target.rs"));
    assert!(inventory(root, &unknown).is_err());
    let mut duplicate = units.clone();
    duplicate[0].filenames = units[1].filenames.clone();
    assert!(input_proofs(root, &json!({"units":duplicate})).is_err());
    for (field, value) in [("test", json!(false)), ("opt_level", json!("0"))] {
        let mut altered = units.clone();
        altered[2].profile[field] = value;
        assert!(input_proofs(root, &json!({"units":altered})).is_err());
    }
}

#[test]
fn outcomes_bind_complete_named_tests_ignored_inventory_and_only_the_standalone_skip() {
    let all = BTreeSet::from(["unit::one".into(), "unit::diagnostic".into()]);
    let ignored = BTreeSet::from(["unit::diagnostic".into()]);
    let passed = "test unit::one ... ok\ntest unit::diagnostic ... ignored, explicit probe\ntest result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s\n";
    verify_outcomes(&TARGETS[1], &all, &ignored, passed).unwrap();
    for changed in [
        passed.replace("unit::one", "unit::other"),
        passed.replace("1 passed", "0 passed"),
        passed.replace("0 filtered", "1 filtered"),
        passed.replace("... ok", "... FAILED"),
    ] {
        assert!(verify_outcomes(&TARGETS[1], &all, &ignored, &changed).is_err());
    }
    assert!(
        verify_outcomes(
            &TARGETS[1],
            &BTreeSet::new(),
            &BTreeSet::new(),
            "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;"
        )
        .is_err()
    );
    let mut public = all;
    public.insert(cargo_tests::LIFECYCLE.into());
    let passed = passed.replace("0 filtered", "1 filtered");
    verify_outcomes(&TARGETS[2], &public, &ignored, &passed).unwrap();
    let mut ignored_lifecycle = ignored;
    ignored_lifecycle.insert(cargo_tests::LIFECYCLE.into());
    assert!(verify_outcomes(&TARGETS[2], &public, &ignored_lifecycle, &passed).is_err());
    assert!(listing("unit::one: test\nunit::one: test\n").is_err());
    assert!(listing("unit::benchmark: benchmark\n").is_err());
}

#[test]
fn retained_harness_streams_reject_rehashed_incomplete_results_and_changed_input_bytes() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let units = units(root);
    let product = root.join("target/release/lkjscript");
    fs::write(&product, b"product").unwrap();
    let mut inputs = vec![evidence::proof(&product, "target/release/lkjscript".into()).unwrap()];
    inputs.extend(input_proofs(root, &json!({"units":units})).unwrap());
    let request = Request {
        repository: root.into(),
        directory: root.join("workspace-tests"),
        inputs,
    };
    fs::create_dir(&request.directory).unwrap();
    let path = root.join(&request.inputs[2].path);
    fs::write(&path, b"#!/bin/sh\ncase \"$1\" in\n--list) printf 'unit::one: test\\n';;\n--ignored) :;;\n*) printf 'test unit::one ... ok\\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\\n';;\nesac\n").unwrap();
    let mut request = request;
    request.inputs[2] = evidence::proof(&path, request.inputs[2].path.clone()).unwrap();
    let mut harness = run_harness(&request, 1);
    assert!(harness.failure.is_none(), "{:?}", harness.failure);
    verify_harness(&request, 1, &harness).unwrap();
    let stdout = request.directory.join("1-run.stdout");
    fs::write(&stdout, b"test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n").unwrap();
    harness.execution.stdout =
        evidence::proof(&stdout, harness.execution.stdout.path.clone()).unwrap();
    assert!(verify_harness(&request, 1, &harness).is_err());
    validate_inputs(root, &request.inputs).unwrap();
    fs::write(&path, b"substituted").unwrap();
    assert!(validate_inputs(root, &request.inputs).is_err());
}
