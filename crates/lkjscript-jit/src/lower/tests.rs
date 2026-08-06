use super::{
    lower_type, preflight_failure_cleanups, FunctionId, LayoutInterner, LoweringError,
    LoweringFailureCode, SsaType,
};

#[test]
fn source_string_and_path_types_are_unsupported_without_structural_metadata() {
    for ty in [SsaType::Str, SsaType::Path] {
        let error = lower_type(FunctionId::new(0), &ty, &LayoutInterner::default())
            .expect_err("source structural type must fail closed before native entry");
        assert_eq!(error.code(), LoweringFailureCode::UnsupportedType);
        assert!(error
            .detail()
            .contains("no compiler-produced native structural owner"));
    }
}

#[test]
fn nested_layout_interner_is_injective_for_previous_result_tag_collision() {
    let id = lkjscript_ir::EnumId::new([1; 32]);
    let first = SsaType::Enum {
        id,
        arguments: vec![
            SsaType::Product(lkjscript_ir::ProductId::new(11)),
            SsaType::Product(lkjscript_ir::ProductId::new(0)),
        ],
    };
    let second = SsaType::Enum {
        id,
        arguments: vec![
            SsaType::Product(lkjscript_ir::ProductId::new(19)),
            SsaType::Unit,
        ],
    };
    let mut layouts = LayoutInterner {
        identities: std::collections::HashMap::new(),
        semantics: std::collections::HashMap::new(),
        region_products: std::collections::HashMap::new(),
        structural: Default::default(),
        witness_slots: Default::default(),
        next: LayoutInterner::FIRST_NESTED_IDENTITY,
    };
    layouts.intern(&first).expect("first exact layout");
    layouts.intern(&second).expect("second exact layout");
    assert_ne!(layouts.identity(&first), layouts.identity(&second));
}

#[test]
fn layout_identity_exhaustion_is_structured() {
    let ty = SsaType::List(Box::new(SsaType::Unit));
    let mut layouts = LayoutInterner {
        identities: std::collections::HashMap::new(),
        semantics: std::collections::HashMap::new(),
        region_products: std::collections::HashMap::new(),
        structural: Default::default(),
        witness_slots: Default::default(),
        next: u32::MAX,
    };
    assert!(matches!(layouts.intern(&ty), Err(LoweringError { .. })));
}

#[test]
fn concrete_enum_layouts_are_injective_and_host_substitutions_reject() {
    let id = lkjscript_ir::EnumId::new([1; 32]);
    let scalar = SsaType::Enum {
        id,
        arguments: vec![SsaType::I64],
    };
    let host = SsaType::Enum {
        id,
        arguments: vec![SsaType::Resource(lkjscript_core::ResourceKind::FileReader)],
    };
    let mut layouts = LayoutInterner {
        identities: std::collections::HashMap::new(),
        semantics: std::collections::HashMap::new(),
        region_products: std::collections::HashMap::new(),
        structural: Default::default(),
        witness_slots: Default::default(),
        next: LayoutInterner::FIRST_NESTED_IDENTITY,
    };
    layouts.intern(&scalar).expect("scalar enum layout");
    layouts.intern(&host).expect("host enum layout identity");
    assert_ne!(layouts.identity(&scalar), layouts.identity(&host));
    let error = lower_type(lkjscript_ir::FunctionId::new(0), &host, &layouts)
        .expect_err("host enum substitution rejects");
    assert_eq!(error.code(), LoweringFailureCode::UnsupportedType);
}

#[test]
fn native_preflight_declines_quadratic_materialization_of_shared_cleanup_chains() {
    use lkjscript_ir::{
        Block, BlockId, BlockMetadata, Constant, DropGlueIdentity, EffectSet, FailureBehavior,
        FailureCleanupAction, FailureCleanupId, FailureCleanupNode, FailureCleanupRoots, Function,
        Instruction, InstructionKind, InstructionMetadata, Origin, Signature, Terminator, ValueId,
    };

    let mut failure_cleanups = Vec::new();
    let mut next = None;
    for index in 0..300_u64 {
        failure_cleanups.push(FailureCleanupNode {
            action: FailureCleanupAction::DropOwner {
                place: None,
                value: ValueId::new(0),
                glue: DropGlueIdentity::Bytes,
            },
            next,
        });
        next = Some(FailureCleanupId::new(index));
    }
    let cleanup = Some(FailureCleanupRoots::single(
        next.expect("nonempty cleanup chain"),
    ));
    let instructions = (0..220_u32)
        .map(|id| Instruction {
            id: ValueId::new(id),
            ty: SsaType::Unit,
            kind: InstructionKind::Constant(Constant::Unit),
            metadata: InstructionMetadata {
                origin: Origin::SYNTHETIC,
                effects: EffectSet::PURE,
                failure: FailureBehavior::None,
                failure_cleanup: cleanup,
                frame_state: None,
            },
        })
        .collect();
    let function = Function {
        id: FunctionId::new(0),
        name: "wide-cleanup-native-fallback".into(),
        signature: Signature::monomorphic(Vec::new(), SsaType::Unit),
        places: Vec::new(),
        failure_cleanups,
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: Vec::new(),
            instructions,
            terminator: Terminator::Return(ValueId::new(219)),
            metadata: BlockMetadata {
                loop_header: false,
                origin: Origin::SYNTHETIC,
                failure_cleanup: None,
                frame_state: None,
            },
        }],
        origin: Origin::SYNTHETIC,
    };
    let error = preflight_failure_cleanups(&function)
        .expect_err("wide shared cleanup expansion must stay on the generic VM");
    assert_eq!(error.code(), LoweringFailureCode::UnsupportedOperation);
    assert!(error
        .detail()
        .contains("wide shared failure cleanup chains"));
}
