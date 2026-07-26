use super::*;
use crate::*;

pub(crate) fn bounded_call_program() -> Program {
    let declared = Signature {
        type_parameters: vec!["T".into()],
        bounds: vec![TraitBound {
            parameter: "T".into(),
            trait_id: TraitId::new(0),
        }],
        parameters: vec![SsaType::TypeParameter("T".into())],
        result: Box::new(SsaType::TypeParameter("T".into())),
    };
    let resolved = Signature::monomorphic(vec![SsaType::I64], SsaType::I64);
    Program {
        sources: vec![SourceMetadata {
            id: 0,
            path: "traits.lkjscript".into(),
        }],
        products: Vec::new(),
        enums: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![
            Function {
                id: FunctionId::new(0),
                name: "copy-value".into(),
                signature: declared,
                places: Vec::new(),
                effects: EffectSet::PURE,
                entry: BlockId::new(0),
                blocks: vec![Block {
                    id: BlockId::new(0),
                    parameters: vec![BlockParameter {
                        id: ValueId::new(0),
                        ty: SsaType::TypeParameter("T".into()),
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    }],
                    instructions: Vec::new(),
                    terminator: Terminator::Return(ValueId::new(0)),
                    metadata: block_metadata(),
                }],
                origin: Origin::SYNTHETIC,
            },
            Function {
                id: FunctionId::new(1),
                name: "main".into(),
                signature: Signature::monomorphic(Vec::new(), SsaType::I64),
                places: Vec::new(),
                effects: EffectSet::PURE,
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
                                signature: resolved,
                                instantiation: Some(GenericInstantiation {
                                    substitutions: vec![TypeSubstitution {
                                        parameter: "T".into(),
                                        ty: SsaType::I64,
                                    }],
                                    witnesses: vec![TraitWitness {
                                        trait_id: TraitId::new(0),
                                        ty: SsaType::I64,
                                        kind: TraitWitnessKind::AutoTrait,
                                    }],
                                }),
                            },
                            metadata: InstructionMetadata {
                                origin: Origin::SYNTHETIC,
                                effects: EffectSet::PURE,
                                safepoint: Safepoint::Required,
                                failure: FailureBehavior::None,
                                frame_state: Some(FrameState {
                                    bytecode_position: 1,
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
