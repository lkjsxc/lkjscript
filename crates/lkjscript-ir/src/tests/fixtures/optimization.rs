use super::*;
use crate::*;

pub(crate) fn optimizable_checked_program() -> Program {
    let checked = EffectSet::MAY_TRAP;
    let runtime = |id, operation, arguments, effects| Instruction {
        id: ValueId::new(id),
        ty: SsaType::I64,
        kind: InstructionKind::Runtime {
            operation,
            arguments,
            signature: Signature::monomorphic(vec![SsaType::I64, SsaType::I64], SsaType::I64),
        },
        metadata: metadata(effects),
    };
    Program {
        sources: Vec::new(),
        products: Vec::new(),
        enums: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![
            Function {
                id: FunctionId::new(0),
                name: "step".into(),
                signature: Signature::monomorphic(vec![SsaType::I64], SsaType::I64),
                places: Vec::new(),
                failure_cleanups: Vec::new(),
                effects: checked,
                entry: BlockId::new(0),
                blocks: vec![Block {
                    id: BlockId::new(0),
                    parameters: vec![BlockParameter {
                        id: ValueId::new(0),
                        ty: SsaType::I64,
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    }],
                    instructions: vec![
                        constant(1, 0),
                        runtime(
                            2,
                            RuntimeOp::BitXor,
                            vec![ValueId::new(0), ValueId::new(1)],
                            EffectSet::PURE,
                        ),
                        constant(3, -1),
                        runtime(
                            4,
                            RuntimeOp::BitAnd,
                            vec![ValueId::new(2), ValueId::new(3)],
                            EffectSet::PURE,
                        ),
                        runtime(
                            5,
                            RuntimeOp::Add,
                            vec![ValueId::new(4), ValueId::new(0)],
                            checked,
                        ),
                        runtime(
                            6,
                            RuntimeOp::Add,
                            vec![ValueId::new(4), ValueId::new(0)],
                            checked,
                        ),
                        runtime(
                            7,
                            RuntimeOp::BitXor,
                            vec![ValueId::new(5), ValueId::new(6)],
                            EffectSet::PURE,
                        ),
                    ],
                    terminator: Terminator::Return(ValueId::new(7)),
                    metadata: block_metadata(),
                }],
                origin: Origin::SYNTHETIC,
            },
            Function {
                id: FunctionId::new(1),
                name: "main".into(),
                signature: Signature::monomorphic(Vec::new(), SsaType::I64),
                places: Vec::new(),
                failure_cleanups: Vec::new(),
                effects: checked,
                entry: BlockId::new(0),
                blocks: vec![Block {
                    id: BlockId::new(0),
                    parameters: Vec::new(),
                    instructions: vec![
                        constant(0, 9),
                        Instruction {
                            id: ValueId::new(1),
                            ty: SsaType::I64,
                            kind: InstructionKind::Call {
                                target: CallTarget::Direct(FunctionId::new(0)),
                                arguments: vec![ValueId::new(0)],
                                signature: Signature::monomorphic(vec![SsaType::I64], SsaType::I64),
                                instantiation: None,
                            },
                            metadata: InstructionMetadata {
                                origin: Origin::SYNTHETIC,
                                effects: checked,
                                safepoint: Safepoint::Required,
                                failure: FailureBehavior::Trap,
                                failure_cleanup: None,
                                frame_state: Some(FrameState {
                                    bytecode_position: 0,
                                    locals: Vec::new(),
                                    operand_stack: Vec::new(),
                                }),
                            },
                        },
                    ],
                    terminator: Terminator::Return(ValueId::new(1)),
                    metadata: block_metadata(),
                }],
                origin: Origin::SYNTHETIC,
            },
        ],
        main: FunctionId::new(1),
    }
}
