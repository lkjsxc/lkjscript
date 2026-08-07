mod enums;
pub(crate) use enums::*;

use crate::*;

pub(crate) fn metadata(effects: EffectSet) -> InstructionMetadata {
    InstructionMetadata {
        origin: Origin::SYNTHETIC,
        effects,
        failure: if effects.contains(EffectSet::MAY_TRAP) {
            FailureBehavior::Trap
        } else {
            FailureBehavior::None
        },
        failure_cleanup: None,
        frame_state: None,
    }
}

pub(crate) fn metadata_cleanup(effects: EffectSet, cleanup: u64) -> InstructionMetadata {
    let mut metadata = metadata(effects);
    metadata.failure_cleanup = Some(FailureCleanupRoots::single(FailureCleanupId::new(cleanup)));
    metadata
}

pub(crate) fn block_metadata() -> BlockMetadata {
    BlockMetadata {
        loop_header: false,
        origin: Origin::SYNTHETIC,
        failure_cleanup: None,
        frame_state: None,
    }
}

pub(crate) fn block_metadata_cleanup(cleanup: u64) -> BlockMetadata {
    let mut metadata = block_metadata();
    metadata.failure_cleanup = Some(FailureCleanupRoots::single(FailureCleanupId::new(cleanup)));
    metadata
}

pub(crate) fn constant(id: u64, value: i64) -> Instruction {
    Instruction {
        id: ValueId::new(id),
        ty: SsaType::I64,
        kind: InstructionKind::Constant(Constant::I64(value)),
        metadata: metadata(EffectSet::PURE),
    }
}

pub(crate) fn one_block_program() -> Program {
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
        ("copy", TraitRole::Copy),
        ("clone", TraitRole::Clone),
        ("drop", TraitRole::Drop),
        ("send", TraitRole::Send),
        ("sync", TraitRole::Sync),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (name, role))| TraitMetadata {
        id: TraitId::new(index as u64),
        name: name.into(),
        role,
        source: None,
    })
    .collect()
}

pub(crate) fn byte_vector_type() -> SsaType {
    SsaType::ByteVector
}

pub(crate) fn owned_place(id: u64, binding: u64) -> crate::PlaceMetadata {
    crate::PlaceMetadata {
        id: PlaceId::new(id),
        binding: crate::BindingId::new(binding),
        ty: byte_vector_type(),
        drop_glue: Some(DropGlueIdentity::ByteVector),
    }
}

pub(crate) fn drop_byte(id: u64, place: u64, value: u64) -> Instruction {
    Instruction {
        id: ValueId::new(id),
        ty: SsaType::Unit,
        kind: InstructionKind::Drop {
            place: PlaceId::new(place),
            value: ValueId::new(value),
            glue: DropGlueIdentity::ByteVector,
            kind: DropEventKind::ImplicitCleanup,
        },
        metadata: metadata(EffectSet::PURE),
    }
}

pub(crate) fn drop_byte_cleanup(id: u64, place: u64, value: u64, cleanup: u64) -> Instruction {
    let mut instruction = drop_byte(id, place, value);
    instruction.metadata.failure_cleanup =
        Some(FailureCleanupRoots::single(FailureCleanupId::new(cleanup)));
    instruction
}

pub(crate) fn place_end_cleanup(id: u64, place: u64, cleanup: u64) -> Instruction {
    let mut instruction = place_end(id, place);
    instruction.metadata.failure_cleanup =
        Some(FailureCleanupRoots::single(FailureCleanupId::new(cleanup)));
    instruction
}

pub(crate) fn place_end(id: u64, place: u64) -> Instruction {
    Instruction {
        id: ValueId::new(id),
        ty: SsaType::Unit,
        kind: InstructionKind::PlaceEnd {
            place: PlaceId::new(place),
        },
        metadata: metadata(EffectSet::PURE),
    }
}

pub(crate) fn ownership_program(function: Function) -> Program {
    let mut program = one_block_program();
    program.functions.push(function);
    program
}
