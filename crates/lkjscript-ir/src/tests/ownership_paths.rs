use super::fixtures::*;
use crate::*;

#[test]
fn ownership_cfg_dataflow_accepts_equal_moves_and_rejects_mismatched_paths() {
    verify(ownership_program(owned_branch_function(true)))
        .expect("equal explicit moves on both branch arms must join");

    let mut implicit_edge = owned_branch_function(true);
    implicit_edge.blocks[0].terminator = Terminator::ConditionalBranch {
        condition: ValueId::new(1),
        true_target: BlockId::new(1),
        true_arguments: Vec::new(),
        false_target: BlockId::new(2),
        false_arguments: Vec::new(),
    };
    let error = verify(ownership_program(implicit_edge))
        .expect_err("current affine owners require explicit edge arguments");
    assert!(
        error.to_string().contains("explicit block argument"),
        "{error}"
    );

    let mismatch = verify(ownership_program(owned_branch_function(false)))
        .expect_err("one moved and one initialized branch must not join");
    assert!(
        mismatch.to_string().contains("implicitly transfer")
            || mismatch.to_string().contains("join exactly")
            || mismatch
                .to_string()
                .contains("lacks nonempty failure cleanup"),
        "wrong branch mismatch diagnostic: {mismatch}"
    );

    let mut returned_original = ownership_program(owned_branch_function(true));
    returned_original.functions[1].blocks[1].terminator = Terminator::Return(ValueId::new(0));
    returned_original.functions[1].blocks[1]
        .metadata
        .failure_cleanup = Some(FailureCleanupRoots::single(FailureCleanupId::new(2)));
    let error = verify(returned_original)
        .expect_err("Move followed by Return of the original owner must fail");
    assert!(error.to_string().contains("unavailable affine"), "{error}");

    let direct_return = Function {
        id: FunctionId::new(1),
        name: "implicit-return".into(),
        signature: Signature::monomorphic(vec![byte_vector_type()], byte_vector_type()),
        places: vec![owned_place(0, 0)],
        failure_cleanups: vec![FailureCleanupNode {
            action: FailureCleanupAction::DropOwner {
                place: Some(PlaceId::new(0)),
                value: ValueId::new(0),
                glue: DropGlueIdentity::ByteVector,
            },
            next: None,
        }],
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: vec![BlockParameter {
                id: ValueId::new(0),
                ty: byte_vector_type(),
                owner_place: Some(PlaceId::new(0)),
                origin: Origin::SYNTHETIC,
            }],
            instructions: Vec::new(),
            terminator: Terminator::Return(ValueId::new(0)),
            metadata: block_metadata_cleanup(0),
        }],
        origin: Origin::SYNTHETIC,
    };
    let error = verify(ownership_program(direct_return))
        .expect_err("Owned entry parameter return requires explicit Move");
    assert!(
        error.to_string().contains("without explicit Move"),
        "{error}"
    );
}
