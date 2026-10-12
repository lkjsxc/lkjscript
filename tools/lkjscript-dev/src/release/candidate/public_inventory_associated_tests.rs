//! Associated coverage requires exact identities and complete passing outcomes.
use super::*;

const PREFIX: &str = "native_declarations::native_associated_blockread::";
const CASE: &str = "native_declarations::native_associated_blockread::native_associated_blockread_consumer_first_four_literals_three_targets_and_detached_bytes";

#[test]
fn associated_inventory_requires_exact_complete_namespace_and_identity() {
    assert_eq!(associated::PREFIX, PREFIX);
    assert_eq!(
        associated::REQUIRED,
        [
            "native_associated_blockread_consumer_first_four_literals_three_targets_and_detached_bytes"
        ]
    );
    let original = super::tests::listing();
    let record = format!("{CASE}: test\n");
    assert_eq!(original.matches(&record).count(), 1);
    let healthy = inventory(&original).expect("complete public inventory");
    assert!(healthy.selected.contains(CASE));
    for changed in [
        original.replace(&record, ""),
        original.replace(CASE, &format!("{CASE}_substitute")),
        original.replace(CASE, &format!("impostor::{CASE}")),
        original.replace(
            "native_associated_blockread::",
            "native_associated_blockread_extra::",
        ),
        original.clone() + &record,
        original
            .lines()
            .filter(|line| !line.starts_with(PREFIX))
            .collect::<Vec<_>>()
            .join("\n"),
    ] {
        assert!(
            inventory(&changed).is_err(),
            "accepted missing or substituted case"
        );
    }
    inventory(&original).expect("healthy inventory after refusals");
}

#[test]
fn associated_inventory_requires_current_and_future_complete_passing_outcomes() {
    let future = format!("{PREFIX}future_behavior");
    let listing = super::tests::listing() + &format!("{future}: test\n");
    let inventory = inventory(&listing).expect("current and future associated inventory");
    assert!(inventory.selected.contains(CASE));
    assert!(inventory.selected.contains(&future));
    let complete = super::tests::success(&inventory);
    passed(&complete, &inventory).expect("every selected behavior passed");
    for name in [CASE, future.as_str()] {
        let record = format!("test {name} ... ok\n");
        for changed in [
            complete.replace(&record, ""),
            complete.replace(&record, &format!("test {name} ... ignored\n")),
            complete.replace(&record, &format!("test {name} ... FAILED\n")),
            complete.replace(&record, &format!("test impostor::{name} ... ok\n")),
            complete.clone() + &record,
        ] {
            assert!(
                passed(&changed, &inventory).is_err(),
                "accepted incomplete associated outcome with a complete summary"
            );
        }
    }
    passed(&complete, &inventory).expect("healthy complete outcomes after refusals");
}

#[test]
fn associated_long_running_notice_requires_its_complete_passing_outcome() {
    let inventory = inventory(&super::tests::listing()).expect("complete associated inventory");
    let header = format!("running {} tests\n", inventory.selected.len());
    let notice = format!("test {CASE} has been running for over 60 seconds\n");
    let outcome = format!("test {CASE} ... ok\n");
    let complete = super::tests::success(&inventory);
    assert_eq!(complete.matches(&header).count(), 1);
    assert_eq!(complete.matches(&outcome).count(), 1);
    let noticed = complete.replacen(&header, &format!("{header}{notice}"), 1);
    passed(&noticed, &inventory).expect("selected pending notice followed by every exact outcome");
    let missing = noticed.replace(&outcome, "");
    assert!(missing.contains(&notice));
    assert!(missing.contains(&format!(
        "test result: ok. {} passed;",
        inventory.selected.len()
    )));
    assert!(
        passed(&missing, &inventory).is_err(),
        "a selected associated notice cannot replace its missing passing outcome"
    );
    let wrong_name = noticed.replace(
        &notice,
        &format!("test impostor::{CASE} has been running for over 60 seconds\n"),
    );
    assert!(passed(&wrong_name, &inventory).is_err());
    passed(&noticed, &inventory).expect("healthy notice and complete outcomes after refusals");
}
