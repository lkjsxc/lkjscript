use super::fixtures::*;
use crate::*;

const SCALE_BLOCKS: usize = 10_000;

fn scalar_instruction(id: u64, value: i64) -> Instruction {
    constant(id, value)
}

fn bool_instruction(id: u64, value: bool) -> Instruction {
    Instruction {
        id: ValueId::new(id),
        ty: SsaType::Bool,
        kind: InstructionKind::Constant(Constant::Bool(value)),
        metadata: metadata(EffectSet::PURE),
    }
}

fn branch(id: usize, target: usize) -> Block {
    Block {
        id: BlockId::new(u64::try_from(id).expect("test BlockId geometry fits u64")),
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: Terminator::Branch {
            target: BlockId::new(u64::try_from(target).expect("test BlockId geometry fits u64")),
            arguments: Vec::new(),
        },
        metadata: block_metadata(),
    }
}

fn program(function: Function) -> Program {
    Program {
        memory: StructuralMemoryMetadata::default(),
        region_products: Vec::new(),
        sources: Vec::new(),
        products: Vec::new(),
        enums: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![function],
        main: FunctionId::new(0),
    }
}

fn linear_function(block_count: usize) -> Function {
    let mut blocks = Vec::with_capacity(block_count);
    for id in 0..block_count {
        let mut block = if id + 1 == block_count {
            Block {
                id: BlockId::new(u64::try_from(id).expect("test BlockId geometry fits u64")),
                parameters: Vec::new(),
                instructions: Vec::new(),
                terminator: Terminator::Return(ValueId::new(0)),
                metadata: block_metadata(),
            }
        } else {
            branch(id, id + 1)
        };
        if id == 0 {
            block.instructions.push(scalar_instruction(0, 42));
        }
        blocks.push(block);
    }
    Function {
        id: FunctionId::new(0),
        name: "generated-linear-cfg".into(),
        signature: Signature::monomorphic(Vec::new(), SsaType::I64),
        places: Vec::new(),
        failure_cleanups: Vec::new(),
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks,
        origin: Origin::SYNTHETIC,
    }
}

fn branching_function() -> Function {
    const DIAMONDS: usize = (SCALE_BLOCKS - 1) / 3;
    const _: () = assert!(1 + 3 * DIAMONDS == SCALE_BLOCKS);
    let mut blocks = Vec::with_capacity(SCALE_BLOCKS);
    blocks.push(Block {
        id: BlockId::new(0),
        parameters: Vec::new(),
        instructions: vec![bool_instruction(0, true), scalar_instruction(1, 42)],
        terminator: Terminator::ConditionalBranch {
            condition: ValueId::new(0),
            true_target: BlockId::new(1),
            true_arguments: Vec::new(),
            false_target: BlockId::new(2),
            false_arguments: Vec::new(),
        },
        metadata: block_metadata(),
    });
    for diamond in 0..DIAMONDS {
        let true_block = 1 + 3 * diamond;
        let false_block = true_block + 1;
        let merge = true_block + 2;
        blocks.push(branch(true_block, merge));
        blocks.push(branch(false_block, merge));
        let terminator = if diamond + 1 == DIAMONDS {
            Terminator::Return(ValueId::new(1))
        } else {
            Terminator::ConditionalBranch {
                condition: ValueId::new(0),
                true_target: BlockId::new(
                    u64::try_from(merge + 1).expect("test BlockId geometry fits u64"),
                ),
                true_arguments: Vec::new(),
                false_target: BlockId::new(
                    u64::try_from(merge + 2).expect("test BlockId geometry fits u64"),
                ),
                false_arguments: Vec::new(),
            }
        };
        blocks.push(Block {
            id: BlockId::new(u64::try_from(merge).expect("test BlockId geometry fits u64")),
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminator,
            metadata: block_metadata(),
        });
    }
    Function {
        id: FunctionId::new(0),
        name: "generated-branch-merge-cfg".into(),
        signature: Signature::monomorphic(Vec::new(), SsaType::I64),
        places: Vec::new(),
        failure_cleanups: Vec::new(),
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks,
        origin: Origin::SYNTHETIC,
    }
}

fn loop_function(block_count: usize) -> Function {
    let mut function = linear_function(block_count);
    function.name = "generated-loop-cfg".into();
    function.blocks[block_count - 1].terminator = Terminator::Branch {
        target: BlockId::new(1),
        arguments: Vec::new(),
    };
    function.blocks[1].metadata.loop_header = true;
    function.blocks[1].metadata.frame_state = Some(FrameState {
        bytecode_position: 0,
        locals: Vec::new(),
        operand_stack: Vec::new(),
    });
    function
}

#[test]
fn verifier_accepts_ten_thousand_block_linear_branching_and_loop_cfgs() {
    verify(program(linear_function(SCALE_BLOCKS)))
        .expect("10,000-block linear CFG must verify without a shape ceiling");
    verify(program(branching_function()))
        .expect("10,000-block branch/merge CFG must verify without dense dominator state");
    verify(program(loop_function(SCALE_BLOCKS)))
        .expect("10,000-block cyclic CFG must verify on explicit graph worklists");
}

#[test]
#[ignore = "release-only verifier timing/RSS measurement geometry"]
fn verifier_release_measurement_geometry() {
    let block_count = std::env::var("LKJSCRIPT_SSA_VERIFY_BLOCKS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(SCALE_BLOCKS);
    verify(program(linear_function(block_count)))
        .expect("release verifier measurement geometry must remain valid");
}

#[test]
fn verifier_rejects_high_block_nondominating_use_and_missing_edge() {
    let mut nondominating = linear_function(SCALE_BLOCKS);
    nondominating.blocks[SCALE_BLOCKS - 1]
        .instructions
        .push(scalar_instruction(1, 7));
    nondominating.blocks[SCALE_BLOCKS / 2].terminator = Terminator::Return(ValueId::new(1));
    let error = verify(program(nondominating))
        .expect_err("a later definition must not dominate an earlier high-block use");
    assert!(error.to_string().contains("does not dominate"), "{error}");

    let mut missing_edge = linear_function(SCALE_BLOCKS);
    missing_edge.blocks[SCALE_BLOCKS - 2].terminator = Terminator::Branch {
        target: BlockId::new(u64::from(u32::MAX) + 1),
        arguments: Vec::new(),
    };
    let error = verify(program(missing_edge))
        .expect_err("a high-block edge to a missing block must fail before indexing");
    assert!(
        error
            .to_string()
            .contains("terminator references a missing block"),
        "{error}"
    );
}
