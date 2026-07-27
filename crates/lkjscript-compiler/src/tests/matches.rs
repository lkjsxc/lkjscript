use super::*;
use lkjscript_ir::{evaluate, verify, EvalConfig, EvalOutcome, EvalValue, InstructionKind};

fn bool_match(scrutinee: &str, arms: &str) -> String {
    format!(
        concat!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
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
        error.contains("canonical typed witness: bool::true"),
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
        "",
        "enum/\nname/\nmaybe\n/name\nforall/\nt\n/forall\nvariants/\n",
        "variant/\nname/\nnone\n/name\nfields/\n/fields\n/variant\n",
        "variant/\nname/\nsome\n/name\nfields/\nvariant-field/\nname/\nvalue\n/name\ntype/\nt\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nmatch/\n",
        "variant-value/\ntype/\nmaybe/\ni64\n/maybe\n/type\nvariant/\nsome\n/variant\nfields/\nvariant-field/\nname/\nvalue\n/name\n42\n/variant-field\n/fields\n/variant-value\n",
        "arms/\narm/\nvariant-pattern/\ntype/\nmaybe/\ni64\n/maybe\n/type\nvariant/\nsome\n/variant\nfields/\nvariant-field-pattern/\nname/\nvalue\n/name\nbinding/\nname/\nx\n/name\n/binding\n/variant-field-pattern\n/fields\n/variant-pattern\nx\n/arm\n",
        "arm/\nvariant-pattern/\ntype/\nmaybe/\ni64\n/maybe\n/type\nvariant/\nnone\n/variant\nfields/\n/fields\n/variant-pattern\n0\n/arm\n/arms\n/match\n/main\n",
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
    let projected = forged
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| match instruction.kind {
            InstructionKind::EnumField {
                enum_id, variant, ..
            } => Some((enum_id, variant)),
            _ => None,
        })
        .expect("match emits guarded field projection");
    let wrong_variant = forged
        .enums
        .iter()
        .find(|definition| definition.id == projected.0)
        .and_then(|definition| {
            definition
                .variants
                .iter()
                .find(|item| item.id != projected.1)
        })
        .map(|item| item.id)
        .expect("matched enum has another variant");
    let field = forged
        .functions
        .iter_mut()
        .flat_map(|function| &mut function.blocks)
        .flat_map(|block| &mut block.instructions)
        .find(|instruction| matches!(instruction.kind, InstructionKind::EnumField { .. }))
        .expect("match emits guarded field projection");
    let InstructionKind::EnumField { variant, .. } = &mut field.kind else {
        unreachable!()
    };
    *variant = wrong_variant;
    let error = verify(forged).expect_err("forged active field must fail closed");
    assert!(error.to_string().contains("enum projection"), "{error}");
}

#[test]
fn forged_prelude_layout_identity_is_rejected_by_ssa_verification() {
    let compiled = compile_source(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n",
        "forged-prelude.lkjscript",
        &Limits::default(),
    )
    .expect("compile prelude-bearing program");
    let mut forged = compiled.ssa().program().clone();
    let option = forged
        .enums
        .iter_mut()
        .find(|item| item.id.bytes() == lkjscript_core::OPTION_ID)
        .expect("Option metadata");
    option.layout.identity = lkjscript_ir::RuntimeLayoutId::new([9; 32]);
    let error = verify(forged).expect_err("forged prelude layout must fail closed");
    assert!(
        error.to_string().contains("identity/name/layout"),
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
