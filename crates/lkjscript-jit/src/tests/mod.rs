mod outcomes;
mod scalars;
mod structural_cutover;
mod trap_sites;

use crate::*;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome};
use lkjscript_ir::{
    verify, Block, BlockId, BlockMetadata, BlockParameter, CallTarget, Constant, EffectSet,
    FailureBehavior, FrameState, Function, FunctionId, Instruction, InstructionKind,
    InstructionMetadata, Origin, Program, Signature, SourceMetadata, SsaType, StructuredOutcome,
    Terminator, TraitId, TraitMetadata, TraitRole, ValueId,
};

use super::{execute_forced, JitConfig};

fn core_traits() -> Vec<TraitMetadata> {
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
        id: TraitId::new(u64::try_from(index).expect("test trait index")),
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
                failure: FailureBehavior::None,
                failure_cleanup: None,
                frame_state: None,
            },
        }]
    } else {
        Vec::new()
    };
    verify(Program {
        prepared_identity: lkjscript_ir::PreparedProgramIdentity::UNBOUND,
        memory: lkjscript_ir::StructuralMemoryMetadata::default(),
        sources: vec![SourceMetadata {
            id: 0,
            path: "terminal.lkjscript".into(),
        }],
        products: Vec::new(),
        region_products: Vec::new(),
        enums: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![Function {
            id: FunctionId::new(0),
            name: "main".into(),
            signature: Signature::monomorphic(Vec::new(), SsaType::I64),
            places: Vec::new(),
            failure_cleanups: Vec::new(),
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
                    failure_cleanup: None,
                    frame_state: None,
                },
            }],
            origin: Origin::SYNTHETIC,
        }],
        main: FunctionId::new(0),
    })
    .expect("verify terminal SSA")
}
