mod validation;

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
    let compiled = compile_source(&bool_match("true", &arms), "bool-match.lkjscript")
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
    let error = compile_source(&nonexhaustive, "bool-nonexhaustive.lkjscript")
        .expect_err("missing true arm")
        .to_string();
    assert!(
        error.contains("canonical typed witness: bool::true"),
        "{error}"
    );

    let arms = format!("arm/\nwildcard/\n/wildcard\n0\n/arm\n{}", bool_arm(true, 1),);
    let error = compile_source(&bool_match("true", &arms), "bool-useless.lkjscript")
        .expect_err("wildcard subsumes later arm")
        .to_string();
    assert!(error.contains("useless or subsumed match arm 1"), "{error}");
}

fn enum_match_source() -> String {
    concat!(
        "",
        "enum/\nname/\nmaybe\n/name\nforall/\nt\n/forall\nvariants/\n",
        "variant/\nname/\nnone\n/name\nfields/\n/fields\n/variant\n",
        "variant/\nname/\nsome\n/name\nfields/\nvariant-field/\nname/\nvalue\n/name\n",
        "type/\nt\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nmatch/\n",
        "variant-value/\ntype/\nmaybe/\ni64\n/maybe\n/type\nvariant/\nsome\n/variant\n",
        "fields/\nvariant-field/\nname/\nvalue\n/name\n42\n/variant-field\n/fields\n/variant-value\n",
        "arms/\narm/\nvariant-pattern/\ntype/\nmaybe/\ni64\n/maybe\n/type\n",
        "variant/\nsome\n/variant\nfields/\nvariant-field-pattern/\nname/\nvalue\n/name\n",
        "binding/\nname/\nx\n/name\n/binding\n/variant-field-pattern\n/fields\n",
        "/variant-pattern\nx\n/arm\n",
        "arm/\nvariant-pattern/\ntype/\nmaybe/\ni64\n/maybe\n/type\nvariant/\nnone\n/variant\n",
        "fields/\n/fields\n/variant-pattern\n0\n/arm\n/arms\n/match\n/main\n",
    ).into()
}

#[test]
fn enum_variant_binding_has_instantiated_field_type() {
    let compiled = compile_source(&enum_match_source(), "enum-match.lkjscript")
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
            InstructionKind::AggregateConsumePayload {
                representation,
                variant,
                ..
            } => Some((representation, variant)),
            _ => None,
        })
        .expect("match emits guarded structural payload projection");
    let type_id = forged
        .memory
        .representations
        .iter()
        .find(|item| item.id == projected.0)
        .map(|item| item.type_id)
        .expect("projection representation type");
    let layout_id = forged
        .memory
        .types
        .iter()
        .find(|item| item.id == type_id)
        .map(|item| item.layout)
        .expect("projection structural type layout");
    let wrong_variant = forged
        .memory
        .layouts
        .iter()
        .find(|layout| layout.id == layout_id)
        .and_then(|layout| match &layout.kind {
            lkjscript_ir::StructuralLayoutKind::Enum { variants, .. } => {
                variants.iter().find(|item| item.variant != projected.1)
            }
            _ => None,
        })
        .map(|item| item.variant)
        .expect("matched enum has another variant");
    let field = forged
        .functions
        .iter_mut()
        .flat_map(|function| &mut function.blocks)
        .flat_map(|block| &mut block.instructions)
        .find(|instruction| {
            matches!(
                instruction.kind,
                InstructionKind::AggregateConsumePayload { .. }
            )
        })
        .expect("match emits guarded structural payload projection");
    let InstructionKind::AggregateConsumePayload { variant, .. } = &mut field.kind else {
        unreachable!()
    };
    *variant = wrong_variant;
    let error = verify(forged).expect_err("forged active field must fail closed");
    assert!(
        error
            .to_string()
            .contains("wrong owner or payload identity"),
        "{error}"
    );
}
