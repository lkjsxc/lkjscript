#![allow(clippy::expect_used, clippy::panic)]

use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_ir::{verify, BlockId, Terminator, ValueId};
use lkjscript_vm::{run_chunk, ExecutionInputs};

const SIMPLE: &str = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n1\n/main\n";
const LOOP: &str =
    "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nloop/\ntype/\ni64\n/type\nbreak/\n3\n/break\n/loop\n/main\n";
const TWO_TRAPS: &str = "def/\nname/\nf\n/name\nfn/\nsig/\ninputs/\nbool\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nx\nbool\n/params\nif/\nx\ntrap/\nstring-literal/\na\n/string-literal\n/trap\ntrap/\nstring-literal/\nb\n/string-literal\n/trap\n/if\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nf/\ntrue\n/f\n/main\n";

fn wide_control_source(branches: usize) -> String {
    let mut source = String::from(
        "def/\nname/\nwide-control\n/name\nfn/\nsig/\ninputs/\nbool\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nflag\nbool\n/params\nvar/\nname/\nx\n/name\ntype/\ni64\n/type\n0\ndo/\n",
    );
    for _ in 0..branches {
        source.push_str(
            "if/\nflag\nset/\nx\nadd/\nx\n1\n/add\n/set\nset/\nx\nadd/\nx\n2\n/add\n/set\n/if\n",
        );
    }
    source.push_str(
        "x\n/do\n/var\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nwide-control/\ntrue\n/wide-control\n/main\n",
    );
    source
}

#[test]
#[ignore = "release-only production source/SSA/bytecode/VM stress beyond 4,096 blocks"]
fn generated_source_executes_more_than_four_thousand_ssa_blocks_in_vm() {
    const BRANCHES: usize = 1_500;
    let compiled = compile_source(&wide_control_source(BRANCHES), "wide-control-cfg.lkjscript")
        .expect("compile source through a verifier CFG beyond 4,096 blocks");
    let wide = compiled
        .ssa()
        .program()
        .functions
        .iter()
        .find(|function| function.name.ends_with("wide-control"))
        .expect("generated wide-control SSA function");
    assert!(
        wide.blocks.len() > 4_096,
        "source produced only {} SSA blocks",
        wide.blocks.len()
    );
    assert!(matches!(
        run_chunk(
            compiled.bytecode(),
            &ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
        ),
        ExecutionOutcome::Returned(value)
            if value.as_i64() == i64::try_from(BRANCHES).ok()
    ));
}

#[test]
fn forged_trap_operand_type_fails_closed() {
    let compiled = compile_source(SIMPLE, "forged-trap.lkjscript").expect("compile fixture");
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
    let compiled =
        compile_source(TWO_TRAPS, "trap-dominance.lkjscript").expect("compile trap branches");
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
    let compiled = compile_source(LOOP, "stale-control.lkjscript").expect("compile loop");
    let mut stale_target = compiled.ssa().program().clone();
    stale_target.functions[0].blocks[0].terminator = Terminator::Branch {
        target: BlockId::new(u64::MAX),
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
