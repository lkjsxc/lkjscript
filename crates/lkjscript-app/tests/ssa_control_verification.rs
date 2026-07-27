#![allow(clippy::expect_used, clippy::panic)]

use lkjscript_compiler::compile_source;
use lkjscript_core::Limits;
use lkjscript_ir::{verify, BlockId, Terminator, ValueId};

const SIMPLE: &str = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n1\n/main\n";
const LOOP: &str =
    "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nloop/\ntype/\ni64\n/type\nbreak/\n3\n/break\n/loop\n/main\n";
const TWO_TRAPS: &str = "def/\nname/\nf\n/name\nfn/\nsig/\ninputs/\nbool\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nx\nbool\n/params\nif/\nx\ntrap/\nstring-literal/\na\n/string-literal\n/trap\ntrap/\nstring-literal/\nb\n/string-literal\n/trap\n/if\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nf/\ntrue\n/f\n/main\n";

#[test]
fn forged_trap_operand_type_fails_closed() {
    let compiled = compile_source(SIMPLE, "forged-trap.lkjscript", &Limits::default())
        .expect("compile fixture");
    let mut forged = compiled.ssa().program().clone();
    forged.functions[0].blocks[0].terminator = Terminator::Trap {
        value: ValueId::new(0),
    };
    let error = verify(forged).expect_err("I64 trap value must be rejected");
    assert!(error
        .to_string()
        .contains("trap terminator value is not Str"));
}

#[test]
fn non_dominating_trap_value_fails_closed() {
    let compiled = compile_source(TWO_TRAPS, "trap-dominance.lkjscript", &Limits::default())
        .expect("compile trap branches");
    let mut forged = compiled.ssa().program().clone();
    let values: Vec<_> = forged.functions[0]
        .blocks
        .iter()
        .filter_map(|block| match &block.terminator {
            Terminator::Trap { value } => Some(*value),
            _ => None,
        })
        .collect();
    assert_eq!(values.len(), 2);
    let first = forged.functions[0]
        .blocks
        .iter_mut()
        .find(|block| matches!(&block.terminator, Terminator::Trap { .. }))
        .expect("first trap block");
    first.terminator = Terminator::Trap { value: values[1] };
    let error = verify(forged).expect_err("sibling trap value must not dominate");
    assert!(error.to_string().contains("does not dominate"));
}

#[test]
fn stale_control_target_and_block_arguments_fail_closed() {
    let compiled =
        compile_source(LOOP, "stale-control.lkjscript", &Limits::default()).expect("compile loop");
    let mut stale_target = compiled.ssa().program().clone();
    stale_target.functions[0].blocks[0].terminator = Terminator::Branch {
        target: BlockId::new(u32::MAX),
        arguments: Vec::new(),
    };
    assert!(verify(stale_target).is_err());

    let mut forged_arguments = compiled.ssa().program().clone();
    let branch = &mut forged_arguments.functions[0].blocks[0].terminator;
    let Terminator::Branch { arguments, .. } = branch else {
        panic!("loop preheader must branch")
    };
    arguments.push(ValueId::new(0));
    assert!(verify(forged_arguments).is_err());
}
