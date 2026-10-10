use super::*;

pub(super) fn listing() -> String {
    [
        "owned::native_owned_producer: test",
        "buffers::native_byte_buffer_move: test",
        "ranges::native_byte_ranges_slice: test",
        "resident_policy::deadline: test",
        "native_declarations::native_owned_products::parallel::native_parallel_joined: test",
        "native_parallel_reads::native_parallel_reads_scoped: test",
        "native_refresh::reviewed: test",
        "copied_binary_authors_builds_and_serves_interactive_topology_from_minimal: test",
        "native_map_entries::native_map_entries_preserve_results_and_linear_projection_work_after_detachment: test",
        "native_declarations::native_fold::native_blocked_folds_import_exactly_and_preserve_complete_results_after_source_removal: test",
        "native_declarations::native_fold::native_blocked_folds_reject_wrong_contract_without_changing_accepted_consumer: test",
        "native_declarations::native_fold::native_blocked_folds_admit_the_complete_argument_before_invoking_a_callback: test",
        "native_declarations::native_fold::history::native_blocked_folds_retain_every_nested_prefix_across_blocks_and_source_removal: test",
        "native_declarations::native_fold::history::native_blocked_folds_admit_nested_input_and_unused_initial_history_completely: test",
        "native_declarations::incremental_units::new_components_do_not_promote_ports_to_compiler_units: test",
        "native_declarations::incremental_units::new_command_targets_keep_exact_incremental_cache_and_detached_behavior: test",
        "native_declarations::incremental_units::new_port_type_failure_preserves_authority_and_cache_then_recovers: test",
        "unrelated_test: test",
    ]
    .join("\n")
        + "\n"
}

fn success(inventory: &Inventory) -> String {
    let mut text = format!("\nrunning {} tests\n", inventory.selected.len());
    for name in &inventory.selected {
        text.push_str(&format!("test {name} ... ok\n"));
    }
    text.push_str(&format!("\ntest result: ok. {} passed; 0 failed; 0 ignored; 0 measured; {} filtered out; finished in 0.01s\n\n", inventory.selected.len(), inventory.all.len() - inventory.selected.len()));
    text
}

#[test]
fn all_required_families_and_exact_topology_are_selected_without_unrelated_cases() {
    let inventory = inventory(&listing()).unwrap();
    assert_eq!(inventory.all.len(), 18);
    assert_eq!(inventory.selected.len(), 17);
    assert!(!inventory.selected.contains("unrelated_test"));
    passed(&success(&inventory), &inventory).unwrap();
}

#[test]
fn missing_families_or_topology_never_become_vacuous_success() {
    let original = listing();
    for omitted in 0..17 {
        let text = original
            .lines()
            .enumerate()
            .filter(|(i, _)| *i != omitted)
            .map(|(_, line)| line)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(inventory(&text).is_err(), "omitted {omitted}");
    }
    assert!(inventory("").is_err());
    assert!(inventory(&original.replace(TOPOLOGY, &format!("prefix_{TOPOLOGY}"))).is_err());
}

#[test]
fn required_parallel_namespaces_accept_nested_paths_and_reject_substring_impostors() {
    let original = listing();
    for (from, to) in [
        (
            "native_owned_products::parallel::native_parallel_joined",
            "native_owned_products::parallel_extra::native_parallel_joined",
        ),
        (
            "native_owned_products::parallel::native_parallel_joined",
            "native_owned_products::prefix_parallel::native_parallel_joined",
        ),
        (
            "native_owned_products::parallel::native_parallel_joined",
            "native_owned_products::parallel::prefix_native_parallel_joined",
        ),
        (
            "native_parallel_reads::native_parallel_reads_scoped",
            "native_declarations::native_parallel_reads_extra::native_parallel_reads_scoped",
        ),
        (
            "native_parallel_reads::native_parallel_reads_scoped",
            "native_declarations::prefix_native_parallel_reads::native_parallel_reads_scoped",
        ),
        (
            "native_parallel_reads::native_parallel_reads_scoped",
            "native_parallel_reads::prefix_native_parallel_reads_scoped",
        ),
        (
            "native_parallel_reads::native_parallel_reads_scoped",
            "native_declarations::native_parallel_reads_scoped",
        ),
    ] {
        assert!(
            inventory(&original.replace(from, to)).is_err(),
            "accepted impostor {to}"
        );
    }
    let nested = original.replace(
        "native_parallel_reads::native_parallel_reads_scoped",
        "native_declarations::native_parallel_reads::nested::native_parallel_reads_scoped",
    );
    let inventory = inventory(&nested).unwrap();
    assert_eq!(inventory.selected.len(), 17);
    passed(&success(&inventory), &inventory).unwrap();
}

#[test]
fn repeated_malformed_benchmark_and_excessive_inventory_records_reject() {
    let text = listing();
    for extra in [
        "unrelated_test: test\n",
        "other: benchmark\n",
        "not a name: test\n",
        ": test\n",
        "not a record\n",
    ] {
        assert!(inventory(&(text.clone() + extra)).is_err());
    }
    let mut many = text;
    for index in 0..4096 {
        many.push_str(&format!("extra_{index}: test\n"));
    }
    assert!(inventory(&many).is_err());
}

#[test]
fn a_successful_exit_or_summary_cannot_hide_unrun_ignored_or_duplicate_tests() {
    let inventory = inventory(&listing()).unwrap();
    let original = success(&inventory);
    for changed in [
        original.replace("17 passed", "7 passed"),
        original.replace("0 ignored", "1 ignored"),
        original.replace("0 failed", "1 failed"),
        original.replace("0 measured", "1 measured"),
        original.replace("1 filtered out", "2 filtered out"),
        original.replace("running 17 tests", "running 0 tests"),
        original.replacen(" ... ok", " ... ignored", 1),
        original.replacen(" ... ok", " ... FAILED", 1),
        original.lines().filter(|line| !line.starts_with("test buffers::")).collect::<Vec<_>>().join("\n"),
        original.clone() + "test native_declarations::native_owned_products::parallel::native_parallel_joined ... ok\n",
        original.clone() + "test unrelated_test ... ok\n",
        original.clone() + &original,
        "test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.01s\n".to_owned(),
    ] {
        assert!(passed(&changed, &inventory).is_err(), "accepted {changed}");
    }
}

fn artifact(source: &str, executable: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"reason":"compiler-artifact", "target":{"name":"public_cli", "src_path":source}, "profile":{"test":true}, "executable":executable})
}

#[test]
fn cargo_output_must_bind_one_completed_source_matched_test_executable() {
    let root = Path::new("/owned/repo");
    let good = artifact(
        "/owned/repo/tests/public_cli.rs",
        serde_json::json!("/owned/target/public_cli-123"),
    );
    let finished = serde_json::json!({"reason":"build-finished", "success":true});
    let output = format!("{good}\n{finished}\n");
    assert_eq!(
        cargo_harness(&output, root).unwrap(),
        Path::new("/owned/target/public_cli-123")
    );
    for output in [
        good.to_string(),
        finished.to_string(),
        format!("{good}\n{good}\n{finished}\n"),
        format!(
            "{good}\n{}\n",
            serde_json::json!({"reason":"build-finished", "success":false})
        ),
        format!("{good}\n{finished}\n{good}\n"),
        format!(
            "{}\n{finished}\n",
            artifact(
                "/foreign/tests/public_cli.rs",
                serde_json::json!("/owned/test")
            )
        ),
        format!(
            "{}\n{finished}\n",
            artifact("/owned/repo/tests/public_cli.rs", serde_json::Value::Null)
        ),
        format!(
            "{}\n{finished}\n",
            artifact(
                "/owned/repo/tests/public_cli.rs",
                serde_json::json!("relative")
            )
        ),
        "not Cargo JSON".to_owned(),
    ] {
        assert!(cargo_harness(&output, root).is_err(), "accepted {output}");
    }
}
