//! The full detached map matrix cannot be substituted with its scalar cost oracle.
use super::*;

const MATRIX: &str = "native_map_entries::native_map_entries_preserve_results_and_linear_projection_work_after_detachment";
const ORACLE: &str = "native_map_entries::cases::map_entry_cost_oracle_rejects_skipped_ingress_repeated_search_and_raw_readmission";
const BASE: &str = "native_owned_fixture: test\nnative_byte_buffer_fixture: test\nnative_byte_ranges_fixture: test\nresident_policy::fixture: test\nparallel::native_parallel_fixture: test\nnative_parallel_reads::native_parallel_reads_fixture: test\nnative_refresh::fixture: test\ncopied_binary_authors_builds_and_serves_interactive_topology_from_minimal: test\nnative_declarations::native_fold::native_blocked_folds_import_exactly_and_preserve_complete_results_after_source_removal: test\nnative_declarations::native_fold::native_blocked_folds_reject_wrong_contract_without_changing_accepted_consumer: test\nnative_declarations::native_fold::native_blocked_folds_admit_the_complete_argument_before_invoking_a_callback: test\nnative_declarations::native_fold::history::native_blocked_folds_retain_every_nested_prefix_across_blocks_and_source_removal: test\nnative_declarations::native_fold::history::native_blocked_folds_admit_nested_input_and_unused_initial_history_completely: test\n";

#[test]
fn map_inventory_requires_the_complete_detached_matrix_not_only_a_cost_predicate() {
    for listing in [BASE.to_owned(), format!("{BASE}{ORACLE}: test\n")] {
        assert!(
            inventory(&listing).is_err(),
            "missing actual candidate execution"
        );
    }
}

#[test]
fn map_inventory_selects_every_map_case_and_checks_each_terminal() {
    let listing = format!(
        "{BASE}{MATRIX}: test\n{ORACLE}: test\nnative_map_entries::future_case: test\nunrelated: test\n"
    );
    let inventory = inventory(&listing).unwrap();
    for case in [MATRIX, ORACLE, "native_map_entries::future_case"] {
        assert!(inventory.selected.contains(case), "unselected {case}");
    }
    assert!(!inventory.selected.contains("unrelated"));
    let mut complete = String::from("\nrunning 16 tests\n");
    for name in &inventory.selected {
        complete.push_str(&format!("test {name} ... ok\n"));
    }
    complete.push_str("\ntest result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 1.0s\n");
    passed(&complete, &inventory).unwrap();
    for invalid in [
        complete.replace(&format!("test {MATRIX} ... ok\n"), ""),
        complete.replace(
            &format!("test {MATRIX} ... ok"),
            &format!("test {MATRIX} ... ignored"),
        ),
        complete.replace(
            &format!("test {MATRIX} ... ok"),
            &format!("test {ORACLE} ... ok"),
        ),
    ] {
        assert!(passed(&invalid, &inventory).is_err());
    }
}

#[test]
fn map_inventory_rejects_matrix_name_and_namespace_substitution() {
    for substitute in [
        MATRIX.replace("native_map_entries::", "prefix_native_map_entries::"),
        MATRIX.replace("native_map_entries::", "native_map_entries_extra::"),
        MATRIX.replace(
            "::native_map_entries_preserve",
            "::prefix_native_map_entries_preserve",
        ),
        "native_map_entries::cost_only".to_owned(),
    ] {
        assert!(inventory(&format!("{BASE}{substitute}: test\n{ORACLE}: test\n")).is_err());
    }
}
