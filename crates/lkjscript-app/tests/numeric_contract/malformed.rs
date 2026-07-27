use super::*;
use lkjscript_ir::{verify, InstructionKind, SsaType};

const EXACT: &str = concat!(
    "main/\nsig/\ninputs/\n/inputs\noutput/\nresult/\nf64\nnumeric-error\n/result\n/output\n/sig\n",
    "convert-i64-to-f64-exact/\n9007199254740993\n/convert-i64-to-f64-exact\n/main\n",
);

fn conversion(program: &mut lkjscript_ir::Program) -> &mut lkjscript_ir::Instruction {
    program
        .functions
        .iter_mut()
        .flat_map(|function| &mut function.blocks)
        .flat_map(|block| &mut block.instructions)
        .find(|instruction| {
            matches!(
                instruction.kind,
                InstructionKind::F64FromI64Exact { .. }
                    | InstructionKind::F64FromI64Rounded { .. }
                    | InstructionKind::I64FromF64Exact { .. }
                    | InstructionKind::I64FromF64Trunc { .. }
            )
        })
        .expect("conversion instruction")
}

#[test]
fn malformed_conversion_operand_and_result_types_fail_closed() {
    let compiled = compile_source(EXACT, "malformed-conversion.lkjscript", &Limits::default())
        .expect("compile conversion fixture");
    let mut wrong_operation = compiled.ssa().program().clone();
    let instruction = conversion(&mut wrong_operation);
    let InstructionKind::F64FromI64Exact { value } = instruction.kind else {
        panic!("expected exact conversion")
    };
    instruction.kind = InstructionKind::I64FromF64Exact { value };
    assert!(verify(wrong_operation).is_err());

    let mut wrong_result = compiled.ssa().program().clone();
    conversion(&mut wrong_result).ty = SsaType::F64;
    assert!(verify(wrong_result).is_err());
}

#[test]
fn malformed_numeric_error_identity_layout_and_cases_fail_closed() {
    let compiled = compile_source(EXACT, "malformed-error.lkjscript", &Limits::default())
        .expect("compile conversion fixture");
    let mut missing = compiled.ssa().program().clone();
    missing
        .enums
        .retain(|item| item.id.bytes() != lkjscript_core::NUMERIC_ERROR_ID);
    assert!(verify(missing).is_err());

    let mut layout = compiled.ssa().program().clone();
    let definition = layout
        .enums
        .iter_mut()
        .find(|item| item.id.bytes() == lkjscript_core::NUMERIC_ERROR_ID)
        .expect("NumericError metadata");
    definition.layout.identity = lkjscript_ir::RuntimeLayoutId::new([9; 32]);
    assert!(verify(layout).is_err());

    let mut cases = compiled.ssa().program().clone();
    let definition = cases
        .enums
        .iter_mut()
        .find(|item| item.id.bytes() == lkjscript_core::NUMERIC_ERROR_ID)
        .expect("NumericError metadata");
    definition.variants.pop();
    assert!(verify(cases).is_err());
}

#[test]
fn source_lowers_to_exactly_four_distinct_ssa_conversion_kinds() {
    let forms = [
        (
            "result/\nf64\nnumeric-error\n/result",
            "convert-i64-to-f64-exact/\n1\n/convert-i64-to-f64-exact",
            0,
        ),
        (
            "f64",
            "convert-i64-to-f64-rounded/\n1\n/convert-i64-to-f64-rounded",
            1,
        ),
        (
            "result/\ni64\nnumeric-error\n/result",
            "convert-f64-to-i64-exact/\n1.0\n/convert-f64-to-i64-exact",
            2,
        ),
        (
            "result/\ni64\nnumeric-error\n/result",
            "convert-f64-to-i64-truncating/\n1.5\n/convert-f64-to-i64-truncating",
            3,
        ),
    ];
    let mut seen = [false; 4];
    for (ty, expression, expected) in forms {
        let source = format!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\n{ty}\n/output\n/sig\n{expression}\n/main\n"
        );
        let compiled = compile_source(&source, "four-kinds.lkjscript", &Limits::default())
            .expect("compile conversion kind");
        let kind = compiled
            .ssa()
            .program()
            .functions
            .iter()
            .flat_map(|function| &function.blocks)
            .flat_map(|block| &block.instructions)
            .find_map(|instruction| match instruction.kind {
                InstructionKind::F64FromI64Exact { .. } => Some(0),
                InstructionKind::F64FromI64Rounded { .. } => Some(1),
                InstructionKind::I64FromF64Exact { .. } => Some(2),
                InstructionKind::I64FromF64Trunc { .. } => Some(3),
                _ => None,
            })
            .expect("distinct conversion SSA kind");
        assert_eq!(kind, expected);
        seen[kind] = true;
    }
    assert!(seen.into_iter().all(|item| item));
}
