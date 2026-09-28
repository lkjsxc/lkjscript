use super::*;
use crate::platform::deployment::AdapterDescriptor;

fn reference(package: u8, requirement: u8) -> RequirementReference {
    RequirementReference {
        package: format!("pkg_{package:032x}").parse().unwrap(),
        requirement: format!("req_{requirement:032x}").parse().unwrap(),
    }
}

fn grant(selector: impl Into<String>, label: &str) -> DeploymentGrant {
    DeploymentGrant {
        requirement: selector.into(),
        sharing_domain: label.to_owned(),
        authority_revision: "ab".repeat(32),
        adapter: AdapterDescriptor::Configuration,
    }
}

#[test]
fn selector_uses_canonical_existing_identity_domains() {
    let expected = reference(1, 2);
    assert!(matches!(parse("jobs").unwrap(), Selector::Name("jobs")));
    assert!(
        matches!(parse(&exact(expected)).unwrap(), Selector::Exact(actual) if actual == expected)
    );
    for value in [
        format!("/{}/{}", expected.package, expected.requirement),
        format!("{}/", expected.package),
        format!("/{}", expected.requirement),
        format!("{}/jobs", expected.package),
        format!("{}/{}", expected.requirement, expected.package),
        format!("pkg_{:032x}/{}", 0, expected.requirement),
        format!("{}/req_{:032x}", expected.package, 0),
        format!("{}/req_{}", expected.package, "A".repeat(32)),
        format!("{}/{}/extra", expected.package, expected.requirement),
        format!("{}/{}\n", expected.package, expected.requirement),
        format!("日本語/{}", expected.requirement),
    ] {
        assert_eq!(
            validate(&value).unwrap_err().code,
            "deployment_grant_selector",
            "{value}"
        );
    }
    for value in ["", "Jobs", "jobs other", "jobs::other"] {
        assert!(validate(value).is_err(), "{value}");
    }
    assert!(validate(&"j".repeat(129)).is_err());
}

#[test]
fn exact_selection_distinguishes_both_identity_coordinates() {
    let a = reference(1, 1);
    let b = reference(2, 1);
    let c = reference(1, 2);
    let grants = [
        grant(exact(a), "a"),
        grant(exact(b), "b"),
        grant(exact(c), "c"),
    ];
    let required = [Some((a, "jobs")), Some((b, "jobs")), Some((c, "jobs"))];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let reordered = order.map(|i| grants[i].clone());
        for required_order in [required, [required[2], required[0], required[1]]] {
            let selected = resolve(&reordered, required_order).unwrap();
            assert_eq!(selected.len(), 3);
            assert_eq!(selected[&a].sharing_domain, "a");
            assert_eq!(selected[&b].sharing_domain, "b");
            assert_eq!(selected[&c].sharing_domain, "c");
        }
    }
}

#[test]
fn unique_names_and_exact_selectors_compose_without_precedence() {
    let a = reference(1, 1);
    let b = reference(2, 1);
    let required = [Some((a, "jobs")), Some((b, "clock"))];
    for grants in [
        [grant("jobs", "a"), grant(exact(b), "b")],
        [grant(exact(b), "b"), grant("jobs", "a")],
    ] {
        let selected = resolve(&grants, required).unwrap();
        assert_eq!(selected[&a].sharing_domain, "a");
        assert_eq!(selected[&b].sharing_domain, "b");
    }
}

#[test]
fn ambiguous_names_never_select_the_unmatched_remainder() {
    let a = reference(1, 1);
    let b = reference(2, 1);
    for required in [
        [Some((a, "jobs")), Some((b, "jobs"))],
        [Some((b, "jobs")), Some((a, "jobs"))],
    ] {
        for grants in [
            vec![grant("jobs", "ambiguous")],
            vec![grant(exact(a), "a"), grant("jobs", "ambiguous")],
            vec![grant("jobs", "ambiguous"), grant(exact(a), "a")],
            vec![
                grant(exact(a), "a"),
                grant(exact(b), "b"),
                grant("jobs", "ambiguous"),
            ],
        ] {
            assert_eq!(
                resolve(&grants, required).unwrap_err().code,
                "deployment_grant_ambiguous"
            );
        }
    }
}

#[test]
fn aliases_cannot_double_grant_and_foreign_exact_references_cannot_fall_back() {
    let a = reference(1, 1);
    let required = [Some((a, "jobs"))];
    for grants in [
        vec![grant("jobs", "a"), grant(exact(a), "duplicate")],
        vec![grant(exact(a), "a"), grant("jobs", "duplicate")],
        vec![grant(exact(a), "a"), grant(exact(a), "duplicate")],
    ] {
        assert_eq!(
            resolve(&grants, required).unwrap_err().code,
            "deployment_grant_duplicate"
        );
    }
    for selector in [
        exact(reference(2, 1)),
        exact(reference(1, 2)),
        "other".to_owned(),
    ] {
        assert_eq!(
            resolve(&[grant(selector, "foreign")], required)
                .unwrap_err()
                .code,
            "deployment_grant_foreign"
        );
    }
    assert_eq!(
        resolve(&[], required).unwrap_err().code,
        "deployment_grant_missing"
    );
}

#[test]
fn incomplete_or_repeated_component_tables_are_not_silently_filtered() {
    let a = reference(1, 1);
    assert_eq!(
        resolve(&[], [None]).unwrap_err().code,
        "deployment_requirement_missing"
    );
    assert_eq!(
        resolve(&[], [Some((a, "jobs")), Some((a, "jobs"))])
            .unwrap_err()
            .code,
        "deployment_requirement_duplicate"
    );
    assert!(resolve(&[], []).unwrap().is_empty());
}
