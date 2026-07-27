use super::*;
use crate::semantic::schema::{ExpectedTypeFact, ScopeEntityKind};

#[test]
fn match_arm_hole_has_join_expectation_and_typed_pattern_scope() {
    let directory = case_dir("hole-match-arm");
    let root = directory.join("main.lkjscript");
    let source = format!(
        concat!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
            "match/\n7\narms/\narm/\nbinding/\nname/\nx\n/name\n/binding\n",
            "{}\n/arm\n/arms\n/match\n/main\n",
        ),
        hole("arm-body", None),
    );
    std::fs::write(&root, source).expect("write match hole");
    let context = context(&root);
    assert!(
        matches!(context.expected_type, ExpectedTypeFact::Available {
        ref canonical, instantiated: true,
    } if canonical == "i64")
    );
    assert!(context.scope_entities.iter().any(|entity| {
        entity.name == "x"
            && entity.kind == ScopeEntityKind::ImmutableLocal
            && entity.instantiated_type == "i64"
    }));
}

#[test]
fn match_scrutinee_hole_uses_exact_instantiated_enum_pattern_type() {
    let directory = case_dir("hole-match-scrutinee");
    let root = directory.join("main.lkjscript");
    let source = format!(
        concat!(
            "enum/\nname/\nmaybe\n/name\nforall/\nt\n/forall\nvariants/\n",
            "variant/\nname/\nnone\n/name\nfields/\n/fields\n/variant\n",
            "variant/\nname/\nsome\n/name\nfields/\nvariant-field/\nname/\nvalue\n/name\ntype/\nt\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\n",
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nmatch/\n{}\narms/\n",
            "arm/\nvariant-pattern/\ntype/\nmaybe/\ni64\n/maybe\n/type\nvariant/\nnone\n/variant\nfields/\n/fields\n/variant-pattern\n0\n/arm\n",
            "arm/\nvariant-pattern/\ntype/\nmaybe/\ni64\n/maybe\n/type\nvariant/\nsome\n/variant\nfields/\nvariant-field-pattern/\nname/\nvalue\n/name\nwildcard/\n/wildcard\n/variant-field-pattern\n/fields\n/variant-pattern\n1\n/arm\n",
            "/arms\n/match\n/main\n",
        ),
        hole("scrutinee", None),
    );
    std::fs::write(&root, source).expect("write enum scrutinee hole");
    let context = context(&root);
    assert!(
        matches!(context.expected_type, ExpectedTypeFact::Available {
        ref canonical, instantiated: true,
    } if canonical == "enum maybe i64"),
        "{:?}",
        context.expected_type
    );
}
