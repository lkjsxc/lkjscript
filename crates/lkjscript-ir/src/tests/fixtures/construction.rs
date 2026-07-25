use crate::*;

pub(crate) fn metadata(effects: EffectSet) -> InstructionMetadata {
    InstructionMetadata {
        origin: Origin::SYNTHETIC,
        effects,
        safepoint: Safepoint::None,
        failure: if effects.contains(EffectSet::MAY_TRAP) {
            FailureBehavior::Trap
        } else {
            FailureBehavior::None
        },
        frame_state: None,
    }
}

pub(crate) fn block_metadata() -> BlockMetadata {
    BlockMetadata {
        loop_header: false,
        origin: Origin::SYNTHETIC,
        frame_state: None,
    }
}

pub(crate) fn constant(id: u32, value: i64) -> Instruction {
    Instruction {
        id: ValueId::new(id),
        ty: SsaType::I64,
        kind: InstructionKind::Constant(Constant::I64(value)),
        metadata: metadata(EffectSet::PURE),
    }
}

pub(crate) fn runtime(
    id: u32,
    operation: RuntimeOp,
    arguments: Vec<ValueId>,
    effects: EffectSet,
) -> Instruction {
    let parameters = arguments.iter().map(|_| SsaType::I64).collect();
    Instruction {
        id: ValueId::new(id),
        ty: SsaType::I64,
        kind: InstructionKind::Runtime {
            operation,
            arguments,
            signature: Signature::monomorphic(parameters, SsaType::I64),
        },
        metadata: metadata(effects),
    }
}

pub(crate) fn one_block_program() -> Program {
    Program {
        sources: Vec::new(),
        products: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![Function {
            id: FunctionId::new(0),
            name: "main".into(),
            signature: Signature::monomorphic(Vec::new(), SsaType::I64),
            places: Vec::new(),
            effects: EffectSet::PURE,
            entry: BlockId::new(0),
            blocks: vec![Block {
                id: BlockId::new(0),
                parameters: Vec::new(),
                instructions: vec![constant(0, 42)],
                terminator: Terminator::Return(ValueId::new(0)),
                metadata: block_metadata(),
            }],
            origin: Origin::SYNTHETIC,
        }],
        main: FunctionId::new(0),
    }
}

pub(crate) fn core_traits() -> Vec<TraitMetadata> {
    [
        ("Copy", TraitRole::Copy),
        ("Clone", TraitRole::Clone),
        ("Drop", TraitRole::Drop),
        ("Send", TraitRole::Send),
        ("Sync", TraitRole::Sync),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (name, role))| TraitMetadata {
        id: TraitId::new(index as u32),
        name: name.into(),
        role,
        source: None,
    })
    .collect()
}

pub(crate) fn owned_buf_type() -> SsaType {
    SsaType::Owned(Box::new(SsaType::Buf))
}

pub(crate) fn owned_place(id: u32, binding: u32) -> crate::PlaceMetadata {
    crate::PlaceMetadata {
        id: PlaceId::new(id),
        binding: crate::BindingId::new(binding),
        ty: owned_buf_type(),
    }
}

pub(crate) fn ownership_program(function: Function) -> Program {
    let mut program = one_block_program();
    program.functions.push(function);
    program
}
