//! A passing summary or neighboring namespace cannot replace the incremental product witnesses.
use super::*;

const PREFIX: &str = "native_declarations::incremental_units::";
const CASES: [&str; 3] = [
    "new_components_do_not_promote_ports_to_compiler_units",
    "new_command_targets_keep_exact_incremental_cache_and_detached_behavior",
    "new_port_type_failure_preserves_authority_and_cache_then_recovers",
];

fn success(inventory: &Inventory) -> String {
    let mut output = format!("\nrunning {} tests\n", inventory.selected.len());
    for name in &inventory.selected {
        output.push_str(&format!("test {name} ... ok\n"));
    }
    output.push_str(&format!("\ntest result: ok. {} passed; 0 failed; 0 ignored; 0 measured; {} filtered out; finished in 1.0s\n",
        inventory.selected.len(), inventory.all.len() - inventory.selected.len()));
    output
}

#[test]
fn incremental_inventory_requires_every_actual_behavior_case() {
    for suffix in CASES {
        let missing = tests::listing().replace(&format!("{PREFIX}{suffix}: test\n"), "");
        assert!(inventory(&missing).is_err(), "missing {suffix}");
        assert!(inventory(&(missing + &format!("{PREFIX}host_only_oracle: test\n"))).is_err());
    }
}

#[test]
fn incremental_inventory_selects_future_cases_and_requires_each_terminal_once() {
    let listing =
        tests::listing() + &format!("{PREFIX}future_case: test\n{PREFIX}nested::oracle: test\n");
    let inventory = inventory(&listing).unwrap();
    assert_eq!(inventory.selected.len(), 29);
    assert_eq!(inventory.all.len(), 30);
    assert!(!inventory.selected.contains("unrelated_test"));
    let complete = success(&inventory);
    passed(&complete, &inventory).unwrap();
    for suffix in CASES.into_iter().chain(["future_case", "nested::oracle"]) {
        let record = format!("test {PREFIX}{suffix} ... ok");
        assert!(inventory.selected.contains(&format!("{PREFIX}{suffix}")));
        for invalid in [
            complete.replace(&format!("{record}\n"), ""),
            complete.replace(&record, &record.replace(" ... ok", " ... ignored")),
            complete.replace(&record, &record.replace(" ... ok", " ... FAILED")),
            complete.clone() + &record + "\n",
        ] {
            assert!(passed(&invalid, &inventory).is_err());
        }
    }
}

#[test]
fn incremental_inventory_rejects_namespace_and_function_substitution() {
    for substitute in [
        "incremental_units::",
        "native_declarations::incremental_units_extra::",
        "native_declarations::prefix_incremental_units::",
    ] {
        assert!(inventory(&tests::listing().replace(PREFIX, substitute)).is_err());
    }
    for suffix in CASES {
        assert!(inventory(&tests::listing().replace(suffix, &format!("prefix_{suffix}"))).is_err());
    }
}
