use super::*;
use crate::*;

pub(crate) fn pass_program() -> Program {
    let add_signature = Signature::monomorphic(vec![SsaType::I64, SsaType::I64], SsaType::I64);
    Program {
        prepared_identity: lkjscript_contracts::PreparedProgramIdentity::UNBOUND,
        memory: StructuralMemoryMetadata::default(),
        region_products: Vec::new(),
        sources: Vec::new(),
        products: Vec::new(),
        enums: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![Function {
            id: FunctionId::new(0),
            name: "main".into(),
            signature: Signature::monomorphic(Vec::new(), SsaType::I64),
            places: Vec::new(),
            failure_cleanups: Vec::new(),
            effects: EffectSet::PURE,
            entry: BlockId::new(0),
            blocks: vec![
                Block {
                    id: BlockId::new(0),
                    parameters: Vec::new(),
                    instructions: vec![
                        Instruction {
                            id: ValueId::new(0),
                            ty: SsaType::Bool,
                            kind: InstructionKind::Constant(Constant::Bool(true)),
                            metadata: metadata(EffectSet::PURE),
                        },
                        constant(1, 2),
                        constant(2, 3),
                        Instruction {
                            id: ValueId::new(3),
                            ty: SsaType::I64,
                            kind: InstructionKind::Runtime {
                                operation: RuntimeOp::Add,
                                arguments: vec![ValueId::new(1), ValueId::new(2)],
                                signature: add_signature,
                            },
                            metadata: metadata(EffectSet::MAY_TRAP),
                        },
                        Instruction {
                            id: ValueId::new(4),
                            ty: SsaType::I64,
                            kind: InstructionKind::Copy(ValueId::new(3)),
                            metadata: metadata(EffectSet::PURE),
                        },
                        constant(5, 99),
                    ],
                    terminator: Terminator::ConditionalBranch {
                        condition: ValueId::new(0),
                        true_target: BlockId::new(1),
                        true_arguments: vec![ValueId::new(4)],
                        false_target: BlockId::new(2),
                        false_arguments: vec![ValueId::new(4)],
                    },
                    metadata: block_metadata(),
                },
                Block {
                    id: BlockId::new(1),
                    parameters: vec![BlockParameter {
                        id: ValueId::new(6),
                        ty: SsaType::I64,
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    }],
                    instructions: Vec::new(),
                    terminator: Terminator::Branch {
                        target: BlockId::new(3),
                        arguments: vec![ValueId::new(6)],
                    },
                    metadata: block_metadata(),
                },
                Block {
                    id: BlockId::new(2),
                    parameters: vec![BlockParameter {
                        id: ValueId::new(7),
                        ty: SsaType::I64,
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    }],
                    instructions: Vec::new(),
                    terminator: Terminator::Branch {
                        target: BlockId::new(3),
                        arguments: vec![ValueId::new(7)],
                    },
                    metadata: block_metadata(),
                },
                Block {
                    id: BlockId::new(3),
                    parameters: vec![BlockParameter {
                        id: ValueId::new(8),
                        ty: SsaType::I64,
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    }],
                    instructions: Vec::new(),
                    terminator: Terminator::Return(ValueId::new(8)),
                    metadata: block_metadata(),
                },
                Block {
                    id: BlockId::new(4),
                    parameters: Vec::new(),
                    instructions: vec![constant(9, -1)],
                    terminator: Terminator::Return(ValueId::new(9)),
                    metadata: block_metadata(),
                },
            ],
            origin: Origin::SYNTHETIC,
        }],
        main: FunctionId::new(0),
    }
}
