use super::*;

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
