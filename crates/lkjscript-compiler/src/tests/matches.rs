use super::*;
use lkjscript_ir::{evaluate, verify, EvalConfig, EvalOutcome, EvalValue, InstructionKind};

fn bool_match(scrutinee: &str, arms: &str) -> String {
    format!(
        concat!(
            "edition/\n2\n/edition\nmain/\nsig/\n->\nI64\n/sig\n",
            "match/\n{}\narms/\n{}\n/arms\n/match\n/main\n",
        ),
        scrutinee, arms,
    )
}

fn bool_arm(value: bool, body: i64) -> String {
    format!("arm/\nbool-pattern/\n{value}\n/bool-pattern\n{body}\n/arm")
}

#[test]
fn bool_match_is_exhaustive_and_lowers_without_match_ssa() {
    let arms = format!("{}\n{}", bool_arm(false, 10), bool_arm(true, 20));
    let compiled = compile_source(
        &bool_match("true", &arms),
        "bool-match.lkjscript",
        &Limits::default(),
    )
    .expect("compile exhaustive Bool match");
    assert_eq!(
        evaluate(compiled.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(20)),
    );
    assert!(compiled
        .ssa()
        .program()
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .all(|instruction| !matches!(
            instruction.kind,
            InstructionKind::EnumValue { .. }
                | InstructionKind::EnumIsVariant { .. }
                | InstructionKind::EnumField { .. }
        )));
}

#[test]
fn rejects_nonexhaustive_and_source_order_useless_arms() {
    let nonexhaustive = bool_match("true", &bool_arm(false, 0));
    let error = compile_source(
        &nonexhaustive,
        "bool-nonexhaustive.lkjscript",
        &Limits::default(),
    )
    .expect_err("missing true arm")
    .to_string();
    assert!(
        error.contains("canonical typed witness: Bool::true"),
        "{error}"
    );

    let arms = format!("arm/\nwildcard/\n/wildcard\n0\n/arm\n{}", bool_arm(true, 1),);
    let error = compile_source(
        &bool_match("true", &arms),
        "bool-useless.lkjscript",
        &Limits::default(),
    )
    .expect_err("wildcard subsumes later arm")
    .to_string();
    assert!(error.contains("useless or subsumed match arm 1"), "{error}");
}

fn enum_match_source() -> String {
    concat!(
        "edition/\n2\n/edition\n",
        "enum/\nname/\nMaybe\n/name\nforall/\nT\n/forall\nvariants/\n",
        "variant/\nname/\nNone\n/name\nfields/\n/fields\n/variant\n",
        "variant/\nname/\nSome\n/name\nfields/\nvariant-field/\nname/\nvalue\n/name\ntype/\nT\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\n->\nI64\n/sig\nmatch/\n",
        "variant-value/\ntype/\nMaybe/\nI64\n/Maybe\n/type\nvariant/\nSome\n/variant\nfields/\nvariant-field/\nname/\nvalue\n/name\n42\n/variant-field\n/fields\n/variant-value\n",
        "arms/\narm/\nvariant-pattern/\ntype/\nMaybe/\nI64\n/Maybe\n/type\nvariant/\nSome\n/variant\nfields/\nvariant-field-pattern/\nname/\nvalue\n/name\nbinding/\nname/\nx\n/name\n/binding\n/variant-field-pattern\n/fields\n/variant-pattern\nx\n/arm\n",
        "arm/\nvariant-pattern/\ntype/\nMaybe/\nI64\n/Maybe\n/type\nvariant/\nNone\n/variant\nfields/\n/fields\n/variant-pattern\n0\n/arm\n/arms\n/match\n/main\n",
    ).into()
}

#[test]
fn enum_variant_binding_has_instantiated_field_type() {
    let compiled = compile_source(
        &enum_match_source(),
        "enum-match.lkjscript",
        &Limits::default(),
    )
    .expect("compile exhaustive enum match");
    assert_eq!(
        evaluate(compiled.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(42)),
    );

    let mut forged = compiled.ssa().program().clone();
    let wrong_variant = forged.enums[0].variants[0].id;
    let test = forged
        .functions
        .iter_mut()
        .flat_map(|function| &mut function.blocks)
        .flat_map(|block| &mut block.instructions)
        .find(|instruction| matches!(instruction.kind, InstructionKind::EnumIsVariant { .. }))
        .expect("match emits stable enum tag test");
    let InstructionKind::EnumIsVariant { variant, .. } = &mut test.kind else {
        unreachable!()
    };
    *variant = wrong_variant;
    let error = verify(forged).expect_err("forged active tag must fail closed");
    assert!(
        error.to_string().contains("active-variant provenance"),
        "{error}"
    );
}

#[test]
fn i64_requires_and_uses_a_remainder_pattern() {
    let arms = concat!(
        "arm/\ni64-pattern/\n7\n/i64-pattern\n1\n/arm\n",
        "arm/\nbinding/\nname/\nother\n/name\n/binding\nother\n/arm",
    );
    let compiled = compile_source(
        &bool_match("9", arms),
        "i64-match.lkjscript",
        &Limits::default(),
    )
    .expect("I64 binding is the remainder");
    assert_eq!(
        evaluate(compiled.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(9)),
    );
}
