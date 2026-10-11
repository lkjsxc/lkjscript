//! The full detached map matrix cannot be substituted with its scalar cost oracle.
use super::*;

const MATRIX: &str = "native_map_entries::native_map_entries_preserve_results_and_linear_projection_work_after_detachment";
const ORACLE: &str = "native_map_entries::cases::map_entry_cost_oracle_rejects_skipped_ingress_repeated_search_and_raw_readmission";
fn base() -> String {
    tests::listing()
        .replace(&format!("{MATRIX}: test\n"), "")
        .replace("unrelated_test: test\n", "")
}

#[test]
fn map_inventory_requires_the_complete_detached_matrix_not_only_a_cost_predicate() {
    for listing in [base(), format!("{}{ORACLE}: test\n", base())] {
        assert!(
            inventory(&listing).is_err(),
            "missing actual candidate execution"
        );
    }
}

#[test]
fn map_inventory_selects_every_map_case_and_checks_each_terminal() {
    let listing = format!(
        "{}{MATRIX}: test\n{ORACLE}: test\nnative_map_entries::future_case: test\nunrelated: test\n",
        base()
    );
    let inventory = inventory(&listing).unwrap();
    for case in [MATRIX, ORACLE, "native_map_entries::future_case"] {
        assert!(inventory.selected.contains(case), "unselected {case}");
    }
    assert!(!inventory.selected.contains("unrelated"));
    let complete = tests::success(&inventory);
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
        assert!(inventory(&format!("{}{substitute}: test\n{ORACLE}: test\n", base())).is_err());
    }
}
