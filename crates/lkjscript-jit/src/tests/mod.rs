mod enum_metadata;
mod heap_identity;
mod list_limits;
mod outcomes;
mod trap_sites;

use crate::*;
use lkjscript_core::{
    ExecutionConfig, ExecutionOutcome, GcHeap, HeapObj, Value, MAX_LIST_EQUAL_STEPS,
};
use lkjscript_ir::{
    verify, Block, BlockId, BlockMetadata, BlockParameter, CallTarget, Constant, EffectSet,
    FailureBehavior, FrameState, Function, FunctionId, Instruction, InstructionKind,
    InstructionMetadata, Origin, Program, Safepoint as IrSafepoint, Signature, SourceMetadata,
    SsaType, StructuredOutcome, Terminator, TraitId, TraitMetadata, TraitRole, ValueId,
};

use super::{execute_forced, JitConfig};

fn core_traits() -> Vec<TraitMetadata> {
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

fn terminal_program(terminator: Terminator, effects: EffectSet) -> lkjscript_ir::VerifiedProgram {
    let instructions = if matches!(terminator, Terminator::Trap { .. }) {
        vec![Instruction {
            id: ValueId::new(0),
            ty: SsaType::Str,
            kind: InstructionKind::Constant(Constant::Str("exact native trap".into())),
            metadata: InstructionMetadata {
                origin: Origin::SYNTHETIC,
                effects: EffectSet::PURE,
                safepoint: IrSafepoint::None,
                failure: FailureBehavior::None,
                frame_state: None,
            },
        }]
    } else {
        Vec::new()
    };
    verify(Program {
        sources: vec![SourceMetadata {
            id: 0,
            path: "terminal.lkjscript".into(),
        }],
        products: Vec::new(),
        enums: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![Function {
            id: FunctionId::new(0),
            name: "main".into(),
            signature: Signature::monomorphic(Vec::new(), SsaType::I64),
            places: Vec::new(),
            effects,
            entry: BlockId::new(0),
            blocks: vec![Block {
                id: BlockId::new(0),
                parameters: Vec::new(),
                instructions,
                terminator,
                metadata: BlockMetadata {
                    loop_header: false,
                    origin: Origin::SYNTHETIC,
                    frame_state: None,
                },
            }],
            origin: Origin::SYNTHETIC,
        }],
        main: FunctionId::new(0),
    })
    .expect("verify terminal SSA")
}
