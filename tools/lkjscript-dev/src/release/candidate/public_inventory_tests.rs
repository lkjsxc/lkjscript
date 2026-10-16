use super::*;

pub(crate) fn listing() -> String {
    let mut listing = [
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
        "native_declarations::incremental_units::native_package_planner_rebuilds_exactly_and_runs_checked_plan_after_source_deletion: test",
        "native_declarations::incremental_units::native_package_planner_compiles_independently_authored_diamond_and_detached_results: test",
        "native_declarations::native_associated_blockread::native_associated_blockread_consumer_first_four_literals_three_targets_and_detached_bytes: test",
        "native_declarations::native_associated_read_views::native_associated_read_views_consumer_first_five_literals_parent_drop_and_detached_bytes: test",
        "native_declarations::native_associated_read_views::native_associated_read_views_source_alias_path_custody_and_complete_category_refusals: test",
        "native_declarations::native_associated_read_views::native_associated_read_views_closed_joined_reads_projected_sources_and_passive_metadata_custody: test",
        "native_declarations::native_associated_read_views::native_associated_read_views_checked_traps_fuel_cleanup_and_subsequent_invocation: test",
        "unrelated_test: test",
    ]
    .join("\n")
        + "\n";
    for (prefix, required) in [
        (fifo::PREFIX, fifo::REQUIRED.as_slice()),
        (sequences::PREFIX, sequences::REQUIRED.as_slice()),
        (nominals::PREFIX, nominals::REQUIRED.as_slice()),
        (storage::PREFIX, storage::REQUIRED.as_slice()),
    ] {
        for suffix in required {
            listing.push_str(&format!("{prefix}{suffix}: test\n"));
        }
    }
    for boundary in BOUNDARIES {
        listing.push_str(&format!("{}: test\n", boundary_name(boundary)));
    }
    listing
}

pub(crate) fn success(inventory: &Inventory) -> String {
    let mut text = format!("\nrunning {} tests\n", inventory.selected.len());
    for name in &inventory.selected {
        text.push_str(&format!("test {name} ... ok\n"));
    }
    text.push_str(&format!("\ntest result: ok. {} passed; 0 failed; 0 ignored; 0 measured; {} filtered out; finished in 0.01s\n\n", inventory.selected.len(), inventory.all.len() - inventory.selected.len()));
    text
}

#[test]
fn full_source_inventory_keeps_semantics_and_selects_only_seven_exact_boundaries() {
    let original = listing();
    let checked = inventory(&original).unwrap();
    assert_eq!((checked.all.len(), checked.selected.len()), (50, 7));
    assert_eq!(
        checked.selected,
        BOUNDARIES.map(boundary_name).into_iter().collect()
    );
    passed(&success(&checked), &checked).unwrap();
    // Exact source witnesses remain mandatory. Synthetic family witnesses identify
    // a required category; another exact case may also satisfy that category.
    for name in checked
        .all
        .iter()
        .filter(|name| name.as_str() != "unrelated_test")
    {
        if ![
            "owned::native_owned_producer",
            "buffers::native_byte_buffer_move",
            "ranges::native_byte_ranges_slice",
            "resident_policy::deadline",
            "native_declarations::native_owned_products::parallel::native_parallel_joined",
            "native_parallel_reads::native_parallel_reads_scoped",
            "native_refresh::reviewed",
        ]
        .contains(&name.as_str())
        {
            let omitted = original.replace(&format!("{name}: test\n"), "");
            assert!(inventory(&omitted).is_err(), "omitted {name}");
            let substituted = original.replace(name, &format!("impostor::{name}"));
            assert!(inventory(&substituted).is_err(), "substituted {name}");
        }
    }
    inventory(&original.replace("owned::native_owned_producer: test\n", ""))
        .expect("exact FIFO and nominal witnesses still satisfy the owned family");
    for (family, namespace) in REQUIRED_FAMILIES {
        let omitted = original
            .lines()
            .filter(|line| {
                !required_family(line.strip_suffix(": test").unwrap(), family, namespace)
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            inventory(&omitted).is_err(),
            "omitted source family {family}"
        );
    }
    let expanded = inventory(
        &(original + "future_source_matrix: test\nnative_boundary_smokes::future_boundary: test\n"),
    )
    .unwrap();
    assert_eq!(expanded.all.len(), 52);
    assert_eq!(expanded.selected, checked.selected);
}

#[test]
fn malformed_duplicate_and_excessive_inventory_never_weakens_exact_boundaries() {
    let original = listing();
    for extra in [
        "unrelated_test: test\n",
        "other: benchmark\n",
        "not a name: test\n",
        ": test\n",
        "not a record\n",
    ] {
        assert!(inventory(&(original.clone() + extra)).is_err());
    }
    let mut many = original;
    for index in 0..4096 {
        many.push_str(&format!("extra_{index}: test\n"));
    }
    assert!(inventory(&many).is_err());
}

#[test]
fn boundary_outcomes_reject_missing_duplicate_foreign_failed_and_ignored_cases() {
    let checked = inventory(&listing()).unwrap();
    let complete = success(&checked);
    for name in &checked.selected {
        let record = format!("test {name} ... ok\n");
        for changed in [
            complete.replace(&record, ""),
            complete.replace(&record, &format!("test {name} ... ignored\n")),
            complete.replace(&record, &format!("test {name} ... FAILED\n")),
            complete.replace(&record, &format!("test impostor::{name} ... ok\n")),
            complete.clone() + &record,
        ] {
            assert!(
                passed(&changed, &checked).is_err(),
                "accepted incomplete {name}"
            );
        }
    }
    for changed in [
        complete.replace("7 passed", "6 passed"),
        complete.replace("0 ignored", "1 ignored"),
        complete.replace("0 failed", "1 failed"),
        complete.replace("0 measured", "1 measured"),
        complete.replace("43 filtered out", "42 filtered out"),
        complete.replace("running 7 tests", "running 0 tests"),
        complete.clone() + &complete,
        "test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 43 filtered out; finished in 0.01s\n".to_owned(),
    ] { assert!(passed(&changed, &checked).is_err()); }
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

#[test]
fn completion_order_and_selected_long_running_notices_preserve_exact_outcomes() {
    let inventory = inventory(&listing()).unwrap();
    let names = inventory.selected.iter().collect::<Vec<_>>();
    let mut output = format!("running {} tests\n", names.len());
    output.push_str(&format!(
        "test {} has been running for over 60 seconds\n",
        names[0]
    ));
    for (index, name) in names.iter().rev().enumerate() {
        output.push_str(&format!("test {name} ... ok\n"));
        if index == 0 {
            output.push_str(&format!(
                "test {} has been running for over 60 seconds\n",
                names[1]
            ));
        }
    }
    output.push_str(&format!(
        "test result: ok. {} passed; 0 failed; 0 ignored; 0 measured; {} filtered out; finished in 65.00s\n",
        names.len(), inventory.all.len() - names.len()
    ));
    passed(&output, &inventory).unwrap();
}

#[test]
fn long_running_notices_never_replace_outcomes_or_grant_foreign_case_credit() {
    let inventory = inventory(&listing()).unwrap();
    let name = inventory.selected.first().unwrap();
    let notice = format!("test {name} has been running for over 60 seconds\n");
    let header = format!("running {} tests\n", inventory.selected.len());
    let original = success(&inventory);
    let missing = original.replace(&format!("test {name} ... ok\n"), "");
    let valid_prefix = format!("{header}{notice}");
    for changed in [
        format!("{notice}{original}"),
        original.replacen(&header, &format!("{valid_prefix}{notice}"), 1),
        original.replacen(
            &header,
            &format!("{header}test unrelated_test has been running for over 60 seconds\n"),
            1,
        ),
        original.replacen(
            &format!("test {name} ... ok\n"),
            &format!("test {name} ... ok\n{notice}"),
            1,
        ),
        format!("{original}{notice}"),
        missing.replacen(&header, &valid_prefix, 1),
        original.replacen(
            &header,
            &format!("{header}{}", notice.replace("60 seconds", "61 seconds")),
            1,
        ),
    ] {
        assert!(passed(&changed, &inventory).is_err(), "accepted {changed}");
    }
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
    assert_eq!(inventory.selected.len(), 7);
    passed(&success(&inventory), &inventory).unwrap();
}
