use super::*;
use std::os::unix::fs::PermissionsExt;

const SEQUENCE_CASES: [&str; 10] = [
    "native_declarations::native_owned_fifo::native_owned_fifo_exact_generic_library_exhaustive_interleavings_and_source_free_results",
    "native_declarations::native_owned_fifo::native_owned_fifo_traps_preserve_complete_input_admission_cleanup_and_following_invocations",
    "native_declarations::native_owned_fifo::native_owned_fifo_finite_fuel_refusal_joins_detached_custody_without_partial_results",
    "native_declarations::native_owned_fifo::negative::native_owned_fifo_rejects_read_escape_and_protected_consumption_without_publication",
    "native_declarations::native_owned_fifo::oracle::native_owned_fifo_oracle_distinguishes_lifo_and_nonconsuming_peek",
    "native_declarations::native_data_sequences::native_data_sequences_generic_export_finite_model_and_detached_execution",
    "native_declarations::native_data_sequences::native_data_sequences_immutable_reads_survive_replacement_removal_disposal_and_transfer",
    "native_declarations::native_data_sequences::native_data_sequences_owned_replacement_preserves_displaced_custody",
    "native_declarations::native_data_sequences::negative::native_data_sequences_reject_hidden_types_active_loans_and_unsafe_substitutions",
    "native_declarations::native_data_sequences::native_data_sequences_bounds_and_fuel_refusals_join_cleanup_and_recover",
];

fn listing() -> String {
    let mut listing = LIST.to_owned();
    for name in SEQUENCE_CASES {
        listing.push_str(&format!("{name}: test\n"));
    }
    listing
}

fn success() -> String {
    let listing = listing();
    let names = listing
        .lines()
        .map(|line| line.strip_suffix(": test").unwrap())
        .filter(|name| *name != "unrelated")
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 27);
    let mut output = format!("\nrunning {} tests\n", names.len());
    for name in &names {
        output.push_str(&format!("test {name} ... ok\n"));
    }
    output.push_str(&format!(
        "\ntest result: ok. {} passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.01s\n",
        names.len()
    ));
    output
}

const LIST: &str = "native_owned_fixture: test\nnative_byte_buffer_fixture: test\nnative_byte_ranges_fixture: test\nresident_policy::fixture: test\nnative_declarations::native_owned_products::parallel::native_parallel_fixture: test\nnative_declarations::native_parallel_reads::native_parallel_reads_fixture: test\nnative_refresh::fixture: test\ncopied_binary_authors_builds_and_serves_interactive_topology_from_minimal: test\nnative_map_entries::native_map_entries_preserve_results_and_linear_projection_work_after_detachment: test\nnative_declarations::native_fold::native_blocked_folds_import_exactly_and_preserve_complete_results_after_source_removal: test\nnative_declarations::native_fold::native_blocked_folds_reject_wrong_contract_without_changing_accepted_consumer: test\nnative_declarations::native_fold::native_blocked_folds_admit_the_complete_argument_before_invoking_a_callback: test\nnative_declarations::native_fold::history::native_blocked_folds_retain_every_nested_prefix_across_blocks_and_source_removal: test\nnative_declarations::native_fold::history::native_blocked_folds_admit_nested_input_and_unused_initial_history_completely: test\nnative_declarations::incremental_units::new_components_do_not_promote_ports_to_compiler_units: test\nnative_declarations::incremental_units::new_command_targets_keep_exact_incremental_cache_and_detached_behavior: test\nnative_declarations::incremental_units::new_port_type_failure_preserves_authority_and_cache_then_recovers: test\nunrelated: test\n";

fn runner<'a>(root: &'a Path, control: &'a process::ProcessControl) -> Runner<'a> {
    let candidate = root.join("candidate");
    fs::write(&candidate, b"test-only candidate identity").unwrap();
    Runner {
        root,
        control,
        receipt: Receipt {
            schema: super::super::SchemaIdentity {
                identity: "lkjscript-native-public-harness".to_owned(),
                version: 1,
            },
            status: "incomplete",
            phase: "created".to_owned(),
            harness_source_commit: "1".repeat(40),
            candidate: file(&candidate, "lkjscript").unwrap(),
            harness: None,
            inventory: None,
            phases: Vec::new(),
            cleanup_complete: false,
            failure: None,
        },
    }
}

fn harness(root: &Path, output: &str, exit: i32) -> PathBuf {
    let path = root.join("fixture-harness");
    let listing = listing();
    let script = format!(
        "#!/bin/sh\nset -eu\n[ \"$PATH\" = '' ]\n[ \"$HOME\" = \"$PWD/home\" ]\n[ \"$TMPDIR\" = \"$PWD/tmp\" ]\n[ -z \"${{CARGO_HOME+x}}\" ]\n[ -z \"${{GH_TOKEN+x}}\" ]\n[ -z \"${{GITHUB_TOKEN+x}}\" ]\n[ -f \"$LKJSCRIPT_RELEASE_CANDIDATE\" ]\nif [ \"$1\" = '--list' ]; then\nprintf '%s' '{listing}'\nelse\n[ \"$1\" = '--exact' ]\n[ \"$2\" = '--test-threads=1' ]\n[ \"$3\" = '--color=never' ]\n[ \"$#\" = 30 ]\nprintf '%s' '{output}'\nexit {exit}\nfi\n"
    );
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

#[test]
fn native_runner_uses_closed_environment_exact_inventory_and_retained_processes() {
    let root = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let executable = harness(root.path(), &success(), 0);
    let control = process::ProcessControl::default();
    let mut runner = runner(root.path(), &control);
    runner
        .evaluate(&executable, &root.path().join("candidate"), work.path())
        .unwrap();
    assert_eq!(
        runner.receipt.inventory.as_ref().unwrap().selected.len(),
        27
    );
    assert_eq!(runner.receipt.phases.len(), 2);
    assert!(
        runner
            .receipt
            .phases
            .iter()
            .all(|phase| super::super::passed(&phase.process))
    );
    assert_eq!(runner.receipt.status, "incomplete"); // evaluation alone is not whole-owner acceptance
    assert!(!runner.receipt.cleanup_complete);
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(root.path().join("receipt.json")).unwrap()).unwrap();
    assert_eq!(
        record["inventory"]["selected"].as_array().unwrap().len(),
        27
    );
    assert!(root.path().join("inventory.stdout.log").is_file());
    assert!(root.path().join("execute.stderr.log").is_file());
}

#[test]
fn native_runner_rejects_failing_process_or_ignored_cases_even_with_a_zero_exit() {
    let success = success();
    for (output, exit) in [
        (success.clone(), 37),
        (success.replacen(" ... ok", " ... ignored", 1), 0),
    ] {
        let root = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let executable = harness(root.path(), &output, exit);
        let control = process::ProcessControl::default();
        let mut runner = runner(root.path(), &control);
        assert!(
            runner
                .evaluate(&executable, &root.path().join("candidate"), work.path())
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
        assert!(
            runner
                .run(
                    "controlled",
                    vec!["/bin/sh".into(), "-c".into(), "exec /bin/sleep 5".into()],
                    root.path(),
                    BTreeMap::new(),
                    Duration::from_millis(40),
                )
                .is_err()
        );
        assert_eq!(runner.receipt.phases.len(), 1);
        assert!(!super::super::passed(&runner.receipt.phases[0].process));
        assert!(!runner.receipt.cleanup_complete);
        assert!(root.path().join("receipt.json").is_file());
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
        let executable = harness(root.path(), &success(), 0);
        let control = process::ProcessControl::default();
        let mut runner = runner(root.path(), &control);
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
                .evaluate(&executable, &root.path().join("candidate"), work.path())
                .is_err()
        );
        assert_eq!(
            runner.receipt.phases.len(),
            if mutate_during { 2 } else { 0 }
        );
        assert!(!runner.receipt.cleanup_complete);
    }
}
