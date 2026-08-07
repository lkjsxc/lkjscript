use super::*;
use lkjscript_ir::InstructionKind;

fn two_payload_variants() -> String {
    concat!(
        "",
        "enum/\nname/\nchoice\n/name\nvariants/\n",
        "variant/\nname/\nleft\n/name\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\ntype/\ni64\n/type\n/variant-field\n",
        "/fields\n/variant\n",
        "variant/\nname/\nright\n/name\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\ntype/\ni64\n/type\n/variant-field\n",
        "/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\nchoice/\n/choice\n/output\n/sig\n",
        "variant-value/\ntype/\nchoice/\n/choice\n/type\n",
        "variant/\nleft\n/variant\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\n9\n/variant-field\n",
        "/fields\n/variant-value\n/main\n",
    )
    .into()
}

#[test]
fn deterministic_enum_has_no_legacy_projection_or_construction_path() {
    let compiled = compile_source(&two_payload_variants(), "enum-structural-only.lkjscript")
        .expect("compile two-variant enum");
    let instructions = compiled
        .ssa()
        .program()
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .map(|instruction| &instruction.kind)
        .collect::<Vec<_>>();
    assert!(instructions
        .iter()
        .any(|kind| { matches!(kind, InstructionKind::DestinationCreate { .. }) }));
    assert!(instructions
        .iter()
        .any(|kind| { matches!(kind, InstructionKind::DestinationFinish { .. }) }));
    assert!(instructions.iter().all(|kind| {
        !matches!(
            kind,
            InstructionKind::EnumValue { .. }
                | InstructionKind::EnumIsVariant { .. }
                | InstructionKind::EnumField { .. }
        )
    }));
}
