use super::*;
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue};

fn product_source(arms: &str) -> String {
    format!(
        concat!(
            "product/\nname/\npair\n/name\nfields/\n",
            "field/\nname/\nleft\n/name\ntype/\nbool\n/type\n/field\n",
            "field/\nname/\nright\n/name\ntype/\nbool\n/type\n/field\n/fields\n/product\n",
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nmatch/\nproduct-value/\npair\n",
            "field/\nleft\ntrue\n/field\nfield/\nright\ntrue\n/field\n/product-value\n",
            "arms/\n{}\n/arms\n/match\n/main\n",
        ),
        arms,
    )
}

fn product_arm(left: &str, right: &str, body: i64) -> String {
    format!(
        concat!(
            "arm/\nproduct-pattern/\ntype/\nproduct\npair\n/type\nfields/\n",
            "product-field-pattern/\nname/\nleft\n/name\n{}\n/product-field-pattern\n",
            "product-field-pattern/\nname/\nright\n/name\n{}\n/product-field-pattern\n",
            "/fields\n/product-pattern\n{}\n/arm",
        ),
        left, right, body,
    )
}

fn bool_pattern(value: bool) -> String {
    format!("bool-pattern/\n{value}\n/bool-pattern")
}

#[test]
fn nested_product_matrix_is_complete_and_ordered() {
    let arms = [
        product_arm(&bool_pattern(false), &bool_pattern(false), 0),
        product_arm(&bool_pattern(false), &bool_pattern(true), 1),
        product_arm(&bool_pattern(true), "wildcard/\n/wildcard", 2),
    ]
    .join("\n");
    let compiled = compile_source(
        &product_source(&arms),
        "product-match.lkjscript",
        &Limits::default(),
    )
    .expect("nested product matrix is exhaustive");
    assert_eq!(
        evaluate(compiled.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(2)),
    );

    let useless = format!(
        "{arms}\n{}",
        product_arm(&bool_pattern(true), &bool_pattern(true), 3),
    );
    let error = compile_source(
        &product_source(&useless),
        "product-useless.lkjscript",
        &Limits::default(),
    )
    .expect_err("nested product arm is subsumed")
    .to_string();
    assert!(error.contains("useless or subsumed match arm 3"), "{error}");
}

fn enum_bool_source() -> String {
    concat!(
        "enum/\nname/\nflag\n/name\nvariants/\n",
        "variant/\nname/\nempty\n/name\nfields/\n/fields\n/variant\n",
        "variant/\nname/\nvalue\n/name\nfields/\nvariant-field/\nname/\nbit\n/name\ntype/\nbool\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nmatch/\nvariant-value/\ntype/\nflag/\n/flag\n/type\nvariant/\nvalue\n/variant\nfields/\nvariant-field/\nname/\nbit\n/name\ntrue\n/variant-field\n/fields\n/variant-value\n",
        "arms/\narm/\nvariant-pattern/\ntype/\nflag/\n/flag\n/type\nvariant/\nvalue\n/variant\nfields/\nvariant-field-pattern/\nname/\nbit\n/name\nbool-pattern/\nfalse\n/bool-pattern\n/variant-field-pattern\n/fields\n/variant-pattern\n0\n/arm\n",
        "arm/\nvariant-pattern/\ntype/\nflag/\n/flag\n/type\nvariant/\nvalue\n/variant\nfields/\nvariant-field-pattern/\nname/\nbit\n/name\nbool-pattern/\ntrue\n/bool-pattern\n/variant-field-pattern\n/fields\n/variant-pattern\n1\n/arm\n",
        "arm/\nvariant-pattern/\ntype/\nflag/\n/flag\n/type\nvariant/\nempty\n/variant\nfields/\n/fields\n/variant-pattern\n2\n/arm\n/arms\n/match\n/main\n",
    ).into()
}

#[test]
fn nested_enum_payload_matrix_is_exhaustive_and_active() {
    let compiled = compile_source(
        &enum_bool_source(),
        "enum-bool-match.lkjscript",
        &Limits::default(),
    )
    .expect("nested enum payload match");
    assert_eq!(
        evaluate(compiled.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(1)),
    );
}

#[test]
fn nested_product_witness_is_canonical_and_repeatable() {
    let arms = [
        product_arm(&bool_pattern(false), "wildcard/\n/wildcard", 0),
        product_arm(&bool_pattern(true), &bool_pattern(false), 1),
    ]
    .join("\n");
    let compile = || {
        compile_source(
            &product_source(&arms),
            "product-witness.lkjscript",
            &Limits::default(),
        )
        .expect_err("missing true/true product")
        .to_string()
    };
    let first = compile();
    assert_eq!(first, compile());
    assert!(first.contains("bool::true"), "{first}");
}
