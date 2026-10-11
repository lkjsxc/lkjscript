//! Nominal custody cannot be certified by a partial family or incomplete libtest outcomes.
use super::*;

#[test]
fn owned_nominal_inventory_rejects_missing_substituted_and_absent_behavior_families() {
    let original = super::tests::listing();
    for suffix in nominals::REQUIRED {
        let name = format!("{}{suffix}", nominals::PREFIX);
        let record = format!("{name}: test\n");
        assert!(inventory(&original.replace(&record, "")).is_err());
        assert!(inventory(&original.replace(&name, &format!("{name}_substitute"))).is_err());
        assert!(inventory(&original.replace(&name, &format!("impostor::{name}"))).is_err());
        assert!(inventory(&(original.clone() + &record)).is_err());
    }
    let absent = original
        .lines()
        .filter(|line| !line.starts_with(nominals::PREFIX))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(inventory(&absent).is_err());
}

#[test]
fn owned_nominal_inventory_selects_future_cases_and_requires_complete_passing_outcomes() {
    let mut listing = super::tests::listing();
    let future = format!("{}future_behavior", nominals::PREFIX);
    listing.push_str(&format!("{future}: test\n"));
    let inventory = inventory(&listing).unwrap();
    assert!(inventory.selected.contains(&future));
    let complete = super::tests::success(&inventory);
    passed(&complete, &inventory).unwrap();
    for name in nominals::REQUIRED
        .into_iter()
        .map(|suffix| format!("{}{suffix}", nominals::PREFIX))
        .chain([future])
    {
        assert!(inventory.selected.contains(&name));
        let record = format!("test {name} ... ok\n");
        assert!(passed(&complete.replace(&record, ""), &inventory).is_err());
        for outcome in ["ignored", "FAILED"] {
            assert!(
                passed(
                    &complete.replace(&record, &format!("test {name} ... {outcome}\n")),
                    &inventory,
                )
                .is_err()
            );
        }
        assert!(passed(&(complete.clone() + &record), &inventory).is_err());
    }
    passed(&complete, &inventory).unwrap();
}
