use super::*;
use crate::{ResourceCategory, ResourceProfile};

#[test]
fn match_matrix_categories_reserve_exactly_before_hir_allocation() {
    let source = concat!(
        "edition/\n2\n/edition\nmain/\nsig/\n->\nI64\n/sig\n",
        "match/\ntrue\narms/\n",
        "arm/\nbool-pattern/\nfalse\n/bool-pattern\n0\n/arm\n",
        "arm/\nbool-pattern/\ntrue\n/bool-pattern\n1\n/arm\n",
        "/arms\n/match\n/main\n",
    );
    let counts = [
        (ResourceCategory::Patterns, 2),
        (ResourceCategory::MatchArms, 2),
        (ResourceCategory::UsefulnessRows, 6),
        (ResourceCategory::UsefulnessColumns, 4),
        (ResourceCategory::UsefulnessSpecializationWork, 720),
        (ResourceCategory::MatchPlans, 1),
        (ResourceCategory::ExhaustivenessWitnessBytes, 65_600),
    ];
    let mut exact = ResourceProfile::default();
    for (category, count) in counts {
        exact = exact.lowered(category, count).unwrap();
    }
    compile_source_with_profile(source, "match-exact.lkjscript", &Limits::default(), exact)
        .expect("exact match matrix reservations");
    for (category, count) in counts {
        let profile = ResourceProfile::default()
            .lowered(category, count - 1)
            .unwrap();
        let error = compile_source_with_profile(
            source,
            "match-plus-one.lkjscript",
            &Limits::default(),
            profile,
        )
        .unwrap_err();
        let diagnostic = error.compiler_resource_diagnostic().unwrap();
        assert_eq!(diagnostic.category, category);
        assert_eq!(diagnostic.before, 0);
        assert_eq!(diagnostic.increment, count);
        assert!(!diagnostic.to_string().contains("nonexhaustive match"));
    }
}
