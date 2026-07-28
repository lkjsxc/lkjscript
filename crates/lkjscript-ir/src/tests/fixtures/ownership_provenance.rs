use super::*;
use crate::*;

pub(crate) fn aliased_places() -> Function {
    Function {
        id: FunctionId::new(1),
        name: "aliased-places".into(),
        signature: Signature::monomorphic(vec![owned_buf_type()], SsaType::Unit),
        places: vec![owned_place(0, 0), owned_place(1, 1)],
        failure_cleanups: vec![FailureCleanupPlan {
            id: FailureCleanupId::new(0),
            actions: vec![FailureCleanupAction::DropOwner {
                place: Some(PlaceId::new(0)),
                value: ValueId::new(0),
                glue: DropGlueIdentity::ByteVector,
            }],
        }],
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: vec![BlockParameter {
                id: ValueId::new(0),
                ty: owned_buf_type(),
                owner_place: Some(PlaceId::new(0)),
                origin: Origin::SYNTHETIC,
            }],
            instructions: vec![Instruction {
                id: ValueId::new(1),
                ty: SsaType::Unit,
                kind: InstructionKind::PlaceInit {
                    place: PlaceId::new(1),
                    value: ValueId::new(0),
                },
                metadata: metadata_cleanup(EffectSet::PURE, 0),
            }],
            terminator: Terminator::Return(ValueId::new(1)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    }
}

pub(crate) fn missing_local_provenance() -> Function {
    Function {
        id: FunctionId::new(1),
        name: "missing-local-provenance".into(),
        signature: Signature::monomorphic(Vec::new(), SsaType::Unit),
        places: vec![owned_place(0, 0)],
        failure_cleanups: Vec::new(),
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: Vec::new(),
            instructions: vec![Instruction {
                id: ValueId::new(0),
                ty: SsaType::Unit,
                kind: InstructionKind::Constant(Constant::Unit),
                metadata: metadata(EffectSet::PURE),
            }],
            terminator: Terminator::Return(ValueId::new(0)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    }
}
