//! FIFO admission requires complete exact identities, then complete passing outcomes.
use super::*;

#[test]
fn fifo_inventory_rejects_each_missing_substituted_and_entirely_absent_family() {
    let original = super::tests::listing();
    for suffix in fifo::REQUIRED {
        let name = format!("{}{suffix}", fifo::PREFIX);
        let record = format!("{name}: test\n");
        assert!(inventory(&original.replace(&record, "")).is_err());
        assert!(inventory(&original.replace(&name, &format!("{name}_substitute"))).is_err());
        assert!(inventory(&original.replace(&name, &format!("impostor::{name}"))).is_err());
    }
    let absent = original
        .lines()
        .filter(|line| !line.starts_with(fifo::PREFIX))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(inventory(&absent).is_err());
}

#[test]
fn every_fifo_case_is_selected_and_missing_or_ignored_outcomes_reject() {
    let mut listing = super::tests::listing();
    let future = format!("{}future_behavior", fifo::PREFIX);
    listing.push_str(&format!("{future}: test\n"));
    let inventory = inventory(&listing).unwrap();
    assert!(inventory.selected.contains(&future));
    let complete = super::tests::success(&inventory);
    passed(&complete, &inventory).unwrap();
    for suffix in fifo::REQUIRED {
        let name = format!("{}{suffix}", fifo::PREFIX);
        assert!(inventory.selected.contains(&name));
        let record = format!("test {name} ... ok\n");
        assert!(passed(&complete.replace(&record, ""), &inventory).is_err());
        assert!(
            passed(
                &complete.replace(&record, &format!("test {name} ... ignored\n")),
                &inventory,
            )
            .is_err()
        );
    }
}
