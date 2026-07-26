use super::enums::{edition2, maybe_declaration};
use super::*;

fn some_value(fields: &str) -> String {
    format!(
        "variant-value/\ntype/\nMaybe/\nI64\n/Maybe\n/type\nvariant/\nSome\n/variant\nfields/\n{fields}/fields\n/variant-value"
    )
}

#[test]
fn resolves_exact_variant_substitution_and_field_order() {
    let field = "variant-field/\nname/\nvalue\n/name\n42\n/variant-field\n";
    let source = edition2(&format!(
        "{}{}",
        maybe_declaration("Maybe"),
        main_source("Maybe/\nI64\n/Maybe", &some_value(field))
    ));
    let program = analyze_one(&source).expect("exact enum construction");
    assert!(
        matches!(program.main.body.kind, ExprKind::EnumValue { ref fields, .. }
        if fields.len() == 1 && fields[0].ty == Type::I64)
    );
}

#[test]
fn rejects_missing_extra_duplicate_order_and_wrong_type() {
    let declaration = maybe_declaration("Maybe");
    let missing = edition2(&format!(
        "{declaration}{}",
        main_source("Maybe/\nI64\n/Maybe", &some_value(""))
    ));
    assert!(analysis_error(&missing).contains("expected 1 fields, got 0"));
    let wrong_name = "variant-field/\nname/\nother\n/name\n42\n/variant-field\n";
    let wrong = edition2(&format!(
        "{declaration}{}",
        main_source("Maybe/\nI64\n/Maybe", &some_value(wrong_name))
    ));
    assert!(analysis_error(&wrong).contains("must be value in declaration order"));
    let duplicate = format!("{wrong_name}{wrong_name}");
    let wrong = edition2(&format!(
        "{declaration}{}",
        main_source("Maybe/\nI64\n/Maybe", &some_value(&duplicate))
    ));
    assert!(analysis_error(&wrong).contains("expected 1 fields, got 2"));
    let wrong_type = "variant-field/\nname/\nvalue\n/name\ntrue\n/variant-field\n";
    let wrong = edition2(&format!(
        "{declaration}{}",
        main_source("Maybe/\nI64\n/Maybe", &some_value(wrong_type))
    ));
    assert!(analysis_error(&wrong).contains("not assignable to I64"));
}
