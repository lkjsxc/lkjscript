use super::*;
use crate::{BudgetAuthority, ResourceCategory, ResourceProfile};

#[test]
fn hir_shape_reservations_are_exact_plus_one_and_deterministic() {
    let source = concat!(
        "edition/\n2\n/edition\n",
        "enum/\nname/\nMaybe\n/name\nforall/\nT\n/forall\nvariants/\n",
        "variant/\nname/\nNone\n/name\nfields/\n/fields\n/variant\n",
        "variant/\nname/\nSome\n/name\nfields/\nvariant-field/\n",
        "name/\nvalue\n/name\ntype/\nT\n/type\n/variant-field\n",
        "/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n",
    );
    let counts = [
        (ResourceCategory::EnumDeclarations, 1),
        (ResourceCategory::EnumVariants, 2),
        (ResourceCategory::VariantFields, 1),
        (
            ResourceCategory::EnumRecursionWork,
            u64::try_from(crate::hir::ENUM_RECURSION_MAX_WORK).unwrap(),
        ),
    ];
    let mut exact = ResourceProfile::default();
    for (category, count) in counts {
        exact = exact.lowered(category, count).unwrap();
    }
    compile_source_with_profile(source, "enum-exact.lkjscript", &Limits::default(), exact).unwrap();
    for (category, count) in counts {
        let profile = ResourceProfile::default()
            .lowered(category, count - 1)
            .unwrap();
        let compile = || {
            compile_source_with_profile(
                source,
                "enum-plus-one.lkjscript",
                &Limits::default(),
                profile,
            )
            .unwrap_err()
        };
        let first = compile();
        let second = compile();
        assert_eq!(first, second);
        let rejection = first.budget_error().unwrap();
        assert_eq!(rejection.category, category);
        assert_eq!(rejection.authority, Some(BudgetAuthority::Hir));
        assert_eq!(rejection.observed, 0);
        assert_eq!(rejection.attempted, count);
        assert!(!rejection.allocated_before_rejection);
        assert_eq!(rejection.prefix(), second.budget_error().unwrap().prefix());
    }
}
