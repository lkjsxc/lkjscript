#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn registry_contains_exact_names_and_rejects_unknown_names() {
    let names: Vec<_> = ResourceProfileName::ALL
        .into_iter()
        .map(ResourceProfileName::as_str)
        .collect();
    assert_eq!(
        names,
        [
            "sandbox",
            "default",
            "build",
            "trusted-local",
            "deterministic"
        ]
    );
    for name in names {
        assert_eq!(ResourceProfile::named(name).unwrap().name().as_str(), name);
    }
    let error = ResourceProfile::named("server").unwrap_err();
    assert_eq!(error.name(), "server");
}

#[test]
fn implementation_maxima_dominate_every_bounded_profile() {
    let maxima = ResourceCeilings::implementation_maxima();
    for name in ResourceProfileName::ALL {
        let profile = ResourceProfile::new(name);
        for category in ResourceCategory::ALL {
            assert!(profile.ceilings().limit(category) <= maxima.limit(category));
            assert!(profile.ceilings().limit(category) > 0);
        }
    }
    assert_eq!(
        ResourceProfile::new(ResourceProfileName::TrustedLocal).ceilings(),
        maxima
    );
}

#[test]
fn lowered_profile_is_immutable_and_has_distinct_identity() {
    let profile = ResourceProfile::default();
    let category = ResourceCategory::Tokens;
    let current = profile.ceilings().limit(category);
    let lowered = profile.lowered(category, current - 1).unwrap();
    assert_eq!(profile.ceilings().limit(category), current);
    assert_eq!(lowered.ceilings().limit(category), current - 1);
    assert_ne!(lowered.identity(), profile.identity());
    assert!(lowered.lowered(category, current).is_err());
}

#[test]
fn profile_identity_is_versioned_and_stable() {
    let first = ResourceProfile::new(ResourceProfileName::Deterministic).identity();
    let second = ResourceProfile::named("deterministic").unwrap().identity();
    assert_eq!(first, second);
    assert_eq!(first.schema, RESOURCE_PROFILE_SCHEMA);
    assert_eq!(first.version, 1);
    assert_eq!(first.implementation_maxima_version, 1);
}
