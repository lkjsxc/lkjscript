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
fn structural_enum_bypasses_legacy_heap_allocation_limits() {
    let float_source = source().replace("i64", "f64").replace("42", "1.5");
    let compiled =
        compile_source(&float_source, "enum-float-limit.lkjscript").expect("compile F64 enum");
    let execution = ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy {
        max_allocations: 0,
        ..lkjscript_core::LimitedExecutionPolicy::conservative()
    });
    for result in [
        execute_forced(compiled.ssa(), &execution, JitConfig::default())
            .expect("baseline returns enum allocation resource outcome"),
        execute_optimizing(compiled.ssa(), &execution, JitConfig::default())
            .expect("proof returns enum allocation resource outcome"),
    ] {
        assert!(matches!(result.outcome, ExecutionOutcome::Returned(_)));
        assert_eq!(result.stats.runtime_heap_successes, 0);
        assert!(result.stats.structural_runtime_calls > 0);
        assert!(result.stats.native_entries > 0);
        assert_eq!(result.stats.vm_fallbacks, 0);
    }
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
