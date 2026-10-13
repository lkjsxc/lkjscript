//! Source-tied read views require every actual public witness and complete namespace outcomes.
use super::{DevError, require};
use std::collections::BTreeSet;

pub(super) const PREFIX: &str = "native_declarations::native_associated_read_views::";
pub(super) const REQUIRED: [&str; 4] = [
    "native_associated_read_views_consumer_first_five_literals_parent_drop_and_detached_bytes",
    "native_associated_read_views_source_alias_path_custody_and_complete_category_refusals",
    "native_associated_read_views_closed_joined_reads_projected_sources_and_passive_metadata_custody",
    "native_associated_read_views_checked_traps_fuel_cleanup_and_subsequent_invocation",
];

pub(super) fn admit(all: &BTreeSet<String>) -> Result<(), DevError> {
    for suffix in REQUIRED {
        require(
            all.contains(&format!("{PREFIX}{suffix}")),
            &format!("required associated read-view witness is absent: {suffix}"),
        )?;
    }
    Ok(())
}

pub(super) fn selected(name: &str) -> bool {
    name.starts_with(PREFIX)
}

#[cfg(test)]
mod tests {
    use super::super::{inventory, passed, tests as fixtures};
    use super::*;

    const NAMESPACE: &str = "native_declarations::native_associated_read_views::";
    const CASES: [&str; 4] = [
        "native_declarations::native_associated_read_views::native_associated_read_views_consumer_first_five_literals_parent_drop_and_detached_bytes",
        "native_declarations::native_associated_read_views::native_associated_read_views_source_alias_path_custody_and_complete_category_refusals",
        "native_declarations::native_associated_read_views::native_associated_read_views_closed_joined_reads_projected_sources_and_passive_metadata_custody",
        "native_declarations::native_associated_read_views::native_associated_read_views_checked_traps_fuel_cleanup_and_subsequent_invocation",
    ];

    #[test]
    fn read_view_inventory_requires_every_exact_public_case_and_retains_inherited_selection() {
        assert_eq!(PREFIX, NAMESPACE);
        assert_eq!(
            REQUIRED,
            [
                "native_associated_read_views_consumer_first_five_literals_parent_drop_and_detached_bytes",
                "native_associated_read_views_source_alias_path_custody_and_complete_category_refusals",
                "native_associated_read_views_closed_joined_reads_projected_sources_and_passive_metadata_custody",
                "native_associated_read_views_checked_traps_fuel_cleanup_and_subsequent_invocation",
            ]
        );
        let original = fixtures::listing();
        let healthy = inventory(&original).expect("complete inherited and view inventory");
        assert_eq!(
            (healthy.all.len(), healthy.selected.len()),
            (43, 42),
            "literal synthetic fixture counts"
        );
        for name in original
            .lines()
            .map(|line| line.strip_suffix(": test").expect("fixture test"))
        {
            assert_eq!(
                healthy.selected.contains(name),
                name != "unrelated_test",
                "inherited selection {name}"
            );
        }
        for name in CASES {
            let record = format!("{name}: test\n");
            assert_eq!(original.matches(&record).count(), 1);
            for changed in [
                original.replace(&record, ""),
                original.replace(name, &format!("{name}_substitute")),
                original.replace(name, &format!("impostor::{name}")),
                original.clone() + &record,
            ] {
                assert!(
                    inventory(&changed).is_err(),
                    "accepted omitted/substituted/duplicate {name}"
                );
            }
        }
        for namespace in [
            "native_associated_read_views::",
            "native_declarations::native_associated_read_views_extra::",
            "native_declarations::prefix_native_associated_read_views::",
        ] {
            assert!(
                inventory(&original.replace(NAMESPACE, namespace)).is_err(),
                "accepted wrong namespace {namespace}"
            );
        }
        inventory(&original).expect("healthy retry after inventory refusals");
    }

    #[test]
    fn read_view_inventory_requires_current_and_future_complete_passing_outcomes() {
        let future = format!("{NAMESPACE}nested::future_behavior");
        let unrelated = format!("impostor::{NAMESPACE}future_behavior");
        let listing = fixtures::listing() + &format!("{future}: test\n{unrelated}: test\n");
        let checked = inventory(&listing).expect("current and future namespace inventory");
        assert!(checked.selected.contains(&future));
        assert!(!checked.selected.contains(&unrelated));
        let complete = fixtures::success(&checked);
        passed(&complete, &checked).expect("all selected cases passed");
        for name in CASES.into_iter().chain([future.as_str()]) {
            assert!(checked.selected.contains(name));
            let record = format!("test {name} ... ok\n");
            assert_eq!(complete.matches(&record).count(), 1);
            for changed in [
                complete.replace(&record, ""),
                complete.replace(&record, &format!("test {name} ... ignored\n")),
                complete.replace(&record, &format!("test {name} ... FAILED\n")),
                complete.replace(&record, &format!("test impostor::{name} ... ok\n")),
                complete.clone() + &record,
            ] {
                assert!(
                    passed(&changed, &checked).is_err(),
                    "accepted incomplete outcome {name}"
                );
            }
        }
        passed(&complete, &checked).expect("healthy complete retry");
    }

    #[test]
    fn read_view_long_running_notices_never_replace_any_required_outcome() {
        let checked = inventory(&fixtures::listing()).expect("view inventory");
        let complete = fixtures::success(&checked);
        let header = format!("running {} tests\n", checked.selected.len());
        assert_eq!(complete.matches(&header).count(), 1);
        for name in CASES {
            let notice = format!("test {name} has been running for over 60 seconds\n");
            let outcome = format!("test {name} ... ok\n");
            let noticed = complete.replacen(&header, &format!("{header}{notice}"), 1);
            passed(&noticed, &checked).expect("one pending notice and actual complete outcomes");
            for changed in [
                noticed.replace(&outcome, ""),
                noticed.replacen(&notice, &format!("{notice}{notice}"), 1),
                noticed.replace(
                    &notice,
                    &format!("test impostor::{name} has been running for over 60 seconds\n"),
                ),
            ] {
                assert!(
                    passed(&changed, &checked).is_err(),
                    "notice supplied credit for {name}"
                );
            }
        }
        passed(&complete, &checked).expect("healthy outcomes after notice refusals");
    }
}
