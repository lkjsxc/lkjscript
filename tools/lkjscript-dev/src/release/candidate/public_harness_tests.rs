use super::*;
use std::os::unix::fs::PermissionsExt;

fn listing() -> String {
    public_inventory::tests::listing()
}

fn success() -> String {
    public_inventory::tests::success(&public_inventory::inventory(&listing()).unwrap())
}

fn runner<'a>(root: &'a Path, control: &'a process::ProcessControl) -> Runner<'a> {
    let candidate = root.join("candidate");
    fs::write(&candidate, b"test-only candidate identity").unwrap();
    Runner {
        source: None,
        root,
        control,
        receipt: Receipt {
            schema: super::super::SchemaIdentity {
                identity: "lkjscript-native-boundary-harness".to_owned(),
                version: 1,
            },
            status: "incomplete",
            phase: "created".to_owned(),
            harness_source_commit: "1".repeat(40),
            purpose: "diagnostic_behavior_evidence",
            verifier: file(&candidate, "fixture-verifier").unwrap(),
            source_semantics: None,
            source_public_cases: None,
            workers: 2,
            candidate: file(&candidate, "lkjscript").unwrap(),
            harness: None,
            inventory: None,
            boundaries: Vec::new(),
            phases: Vec::new(),
            cleanup_complete: false,
            failure: None,
        },
    }
}

fn harness(root: &Path, output: &str, exit: i32, workers: usize, markers: bool) -> PathBuf {
    let path = root.join("fixture-harness");
    let candidate = file(&root.join("candidate"), "lkjscript").unwrap();
    let listing = listing();
    let mut marker_writes = String::new();
    if markers {
        for boundary in public_inventory::BOUNDARIES {
            let marker = serde_json::json!({
                "schema":{"identity":"lkjscript-native-boundary-smoke","version":1},
                "test":public_inventory::boundary_name(boundary),
                "candidate_sha256":candidate.sha256,
                "observations":[{"arguments":crate::release::candidate::boundary_evidence::tests::arguments(boundary),"exit_code":0,"joined":true,"environment_cleared":true,"executable_sha256":candidate.sha256}],
                "cleanup_complete":true,
            });
            marker_writes.push_str(&format!(
                "printf '%s' '{marker}' > \"$LKJSCRIPT_NATIVE_BOUNDARY_EVIDENCE/{boundary}.json\"\n"
            ));
        }
    }
    let script = format!(
        "#!/bin/sh\nset -eu\n[ \"$PATH\" = '' ]\n[ \"$HOME\" = \"$PWD/home\" ]\n[ \"$TMPDIR\" = \"$PWD/tmp\" ]\n[ -z \"${{CARGO_HOME+x}}\" ]\n[ -z \"${{GH_TOKEN+x}}\" ]\n[ -z \"${{GITHUB_TOKEN+x}}\" ]\n[ -f \"$LKJSCRIPT_RELEASE_CANDIDATE\" ]\n[ -d \"$LKJSCRIPT_NATIVE_BOUNDARY_EVIDENCE\" ]\nif [ \"$1\" = '--list' ]; then\nprintf '%s' '{listing}'\nelse\n[ \"$1\" = '--exact' ]\n[ \"$2\" = '--test-threads={workers}' ]\n[ \"$3\" = '--color=never' ]\n[ \"$#\" = 10 ]\n{marker_writes}printf '%s' '{output}'\nexit {exit}\nfi\n"
    );
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

#[test]
fn boundary_runner_binds_closed_environment_seven_cases_and_process_markers() {
    for workers in [1, 2, 16] {
        let root = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let control = process::ProcessControl::default();
        let mut runner = runner(root.path(), &control);
        runner.receipt.workers = workers;
        let executable = harness(root.path(), &success(), 0, workers, true);
        runner
            .evaluate(
                &executable,
                &executable,
                &root.path().join("candidate"),
                work.path(),
            )
            .unwrap();
        let inventory = runner.receipt.inventory.as_ref().unwrap();
        assert_eq!((inventory.all.len(), inventory.selected.len()), (50, 7));
        assert_eq!(runner.receipt.boundaries.len(), 7);
        let command = &runner.receipt.phases[1].command;
        assert_eq!(command[2], format!("--test-threads={workers}"));
        assert_eq!(
            command[4..]
                .iter()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>(),
            inventory.selected
        );
        assert_eq!(runner.receipt.phases.len(), 2);
        assert!(
            runner
                .receipt
                .phases
                .iter()
                .all(|phase| super::super::passed(&phase.process))
        );
        assert_eq!(runner.receipt.status, "incomplete");
        assert!(!runner.receipt.cleanup_complete); // evaluation alone is not owning acceptance
        let record: serde_json::Value =
            serde_json::from_slice(&fs::read(root.path().join("receipt.json")).unwrap()).unwrap();
        assert_eq!(
            record["schema"]["identity"],
            "lkjscript-native-boundary-harness"
        );
        assert_eq!(record["purpose"], "diagnostic_behavior_evidence");
        assert!(record["source_semantics"].is_null());
        assert_eq!(record["boundaries"].as_array().unwrap().len(), 7);
    }
}

#[test]
fn boundary_runner_rejects_invalid_workers_before_any_process() {
    for workers in [0, 17] {
        let root = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let control = process::ProcessControl::default();
        let mut runner = runner(root.path(), &control);
        runner.receipt.workers = workers;
        let executable = harness(root.path(), &success(), 0, workers, true);
        assert!(
            runner
                .evaluate(
                    &executable,
                    &executable,
                    &root.path().join("candidate"),
                    work.path()
                )
                .is_err()
        );
        assert!(runner.receipt.phases.is_empty());
        assert!(runner.receipt.inventory.is_none());
    }
}

#[test]
fn boundary_runner_keeps_zero_exit_distinct_from_complete_outcomes_and_markers() {
    let complete = success();
    let record = "test native_boundary_smokes::deployment_batch ... ok\n";
    assert_eq!(complete.matches(record).count(), 1);
    for (output, exit, markers) in [
        (complete.clone(), 37, true),
        (complete.replace(record, ""), 0, true),
        (
            complete.replace(record, &record.replace(" ... ok", " ... ignored")),
            0,
            true,
        ),
        (complete.clone() + record, 0, true),
        (complete.clone(), 0, false),
    ] {
        let root = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let control = process::ProcessControl::default();
        let mut runner = runner(root.path(), &control);
        let executable = harness(root.path(), &output, exit, 2, markers);
        assert!(
            runner
                .evaluate(
                    &executable,
                    &executable,
                    &root.path().join("candidate"),
                    work.path()
                )
                .is_err()
        );
        assert_eq!(runner.receipt.phases.len(), 2);
        assert_eq!(runner.receipt.phases[1].process.exit_code, Some(exit));
        assert_ne!(runner.receipt.status, "passed");
        assert!(!runner.receipt.cleanup_complete);
        assert_eq!(
            fs::read_to_string(root.path().join("execute.stdout.log")).unwrap(),
            output
        );
    }
}

#[test]
fn native_runner_cancellation_and_timeout_never_become_passing_evidence() {
    for cancel in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let control = process::ProcessControl::default();
        if cancel {
            control.kill();
        }
        let mut runner = runner(root.path(), &control);
        let result = runner.run(
            "bounded",
            vec!["/bin/sleep".into(), "5".into()],
            root.path(),
            BTreeMap::new(),
            Duration::from_millis(20),
        );
        assert!(result.is_err());
        assert_ne!(runner.receipt.status, "passed");
        assert!(!runner.receipt.cleanup_complete);
        assert_eq!(runner.receipt.phases.len(), 1);
        assert_ne!(
            runner.receipt.phases[0].process.status,
            process::ProcessStatus::Passed
        );
    }
}

#[test]
fn native_source_witness_accepts_linked_worktrees_without_relaxing_root_identity() {
    fn git(root: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args([
                "-c",
                "user.name=Native witness fixture",
                "-c",
                "user.email=native@example.invalid",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(root)
            .env_clear()
            .envs(process::environment())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("source");
    fs::create_dir(&root).unwrap();
    git(&root, &["init", "--initial-branch=fixture"]);
    fs::write(root.join("Cargo.toml"), b"fixture").unwrap();
    git(&root, &["add", "Cargo.toml"]);
    git(&root, &["commit", "--message", "owned fixture"]);
    let worktree = temporary.path().join("linked");
    git(
        &root,
        &[
            "worktree",
            "add",
            "--detach",
            worktree.to_str().unwrap(),
            "HEAD",
        ],
    );
    assert_eq!(harness_repository(&root).unwrap(), root);
    assert_eq!(harness_repository(&worktree).unwrap(), worktree);
    assert!(root.join(".git").is_dir());
    assert!(worktree.join(".git").is_file());
    let nested = root.join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("Cargo.toml"), b"not the root").unwrap();
    assert!(harness_repository(&nested).is_err());
    assert!(harness_repository(temporary.path()).is_err());
    fs::remove_file(worktree.join("Cargo.toml")).unwrap();
    std::os::unix::fs::symlink(root.join("Cargo.toml"), worktree.join("Cargo.toml")).unwrap();
    assert!(harness_repository(&worktree).is_err());
}

#[test]
fn native_runner_rejects_substituted_or_changed_candidate_even_with_passing_output() {
    for mutate_during in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let control = process::ProcessControl::default();
        let mut runner = runner(root.path(), &control);
        let executable = harness(root.path(), &success(), 0, 2, true);
        if mutate_during {
            let script = fs::read_to_string(&executable).unwrap().replace(
                "exit 0",
                "printf 'substituted' > \"$LKJSCRIPT_RELEASE_CANDIDATE\"\nexit 0",
            );
            fs::write(&executable, script).unwrap();
        } else {
            fs::write(root.path().join("candidate"), "substituted").unwrap();
        }
        assert!(
            runner
                .evaluate(
                    &executable,
                    &executable,
                    &root.path().join("candidate"),
                    work.path()
                )
                .is_err()
        );
        assert_eq!(
            runner.receipt.phases.len(),
            if mutate_during { 2 } else { 0 }
        );
        assert!(!runner.receipt.cleanup_complete);
    }
}
