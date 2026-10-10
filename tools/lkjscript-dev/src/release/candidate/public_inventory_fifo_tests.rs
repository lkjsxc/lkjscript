//! Current native-owned selection must include every FIFO behavior witness.
use super::*;

const PREFIX: &str = "native_declarations::native_owned_fifo::";
const FIFO: [&str; 5] = [
    "native_owned_fifo_exact_generic_library_exhaustive_interleavings_and_source_free_results",
    "native_owned_fifo_traps_preserve_complete_input_admission_cleanup_and_following_invocations",
    "native_owned_fifo_finite_fuel_refusal_joins_detached_custody_without_partial_results",
    "negative::native_owned_fifo_rejects_read_escape_and_protected_consumption_without_publication",
    "oracle::native_owned_fifo_oracle_distinguishes_lifo_and_nonconsuming_peek",
];

#[test]
fn current_native_owned_family_selects_all_fifo_witnesses_and_rejects_partial_execution() {
    let mut listing = super::tests::listing();
    if !listing.ends_with('\n') {
        listing.push('\n');
    }
    for suffix in FIFO {
        listing.push_str(&format!("{PREFIX}{suffix}: test\n"));
    }
    let inventory = inventory(&listing).unwrap();
    for suffix in FIFO {
        assert!(inventory.selected.contains(&format!("{PREFIX}{suffix}")));
    }
    let count = inventory.selected.len();
    let header = format!("running {count} tests\n");
    let summary = format!(
        "test result: ok. {count} passed; 0 failed; 0 ignored; 0 measured; {} filtered out; finished in 1.00s\n",
        inventory.all.len() - count,
    );
    let mut complete = header.clone();
    for name in &inventory.selected {
        complete.push_str(&format!("test {name} ... ok\n"));
    }
    complete.push_str(&summary);
    passed(&complete, &inventory).unwrap();
    for suffix in FIFO {
        let record = format!("test {PREFIX}{suffix} ... ok\n");
        assert!(passed(&complete.replace(&record, ""), &inventory).is_err());
        assert!(
            passed(
                &complete.replace(&record, &format!("test {PREFIX}{suffix} ... ignored\n")),
                &inventory,
            )
            .is_err()
        );
    }
}
