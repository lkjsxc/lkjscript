#[test]
fn mixed_legacy_heap_route_is_rejected() {
    let mut chunk = product_chunk();
    chunk.structural_layouts[0].kind = crate::StructuralLayoutKind::Product {
        product: crate::ProductId::new(0),
        fields: vec![crate::StructuralFieldMetadata {
            identity: identity(5),
            runtime_type: None,
            route: crate::StructuralFieldRoute::LegacyHeap,
            resource: None,
        }],
    };
    chunk.structural_destinations[0].fields = match &chunk.structural_layouts[0].kind {
        crate::StructuralLayoutKind::Product { fields, .. } => fields.clone(),
        crate::StructuralLayoutKind::String
        | crate::StructuralLayoutKind::Path
        | crate::StructuralLayoutKind::Enum { .. } => Vec::new(),
    };
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    assert!(error(chunk).contains("mixes a legacy heap route"));
}

#[test]
fn structural_failure_cleanup_checks_exact_action_identity() {
    let mut chunk = product_chunk();
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    let mut proto = Chunk::new().main;
    proto.name = "structural-cleanup".into();
    proto.arity = 1;
    proto.locals = 1;
    proto.memory_plan = chunk.memory_plan;
    proto.parameter_structurals = vec![Some(crate::StructuralRepresentationId::new(0))];
    proto.parameter_structural_places = vec![Some(0)];
    proto.unique_places = 1;
    proto.emit(Op::Nop);
    proto.emit_op_u64_pair(Op::StructuralDropPlace, 0, 0);
    proto.emit(Op::Pop);
    proto.emit_op_u8(Op::StructuralPlaceEnd, 0);
    proto.emit(Op::Pop);
    proto.emit(Op::Unit);
    proto.emit(Op::Return);
    proto.failure_cleanups = vec![crate::FailureCleanupNode {
        action: crate::FailureCleanupAction::DropStructural {
            local: 0,
            place: Some(0),
            representation: crate::StructuralRepresentationId::new(0),
        },
        next: None,
    }];
    proto.failure_cleanup_ranges = vec![crate::FailureCleanupRange {
        start: 0,
        end: u64::try_from(proto.code.len()).expect("small fixture"),
        plan: Some(crate::FailureCleanupRoots::single(
            crate::FailureCleanupId::new(0),
        )),
        unentered_plan: None,
    }];
    chunk.protos.push(proto);
    validate_chunk(chunk.clone(), ValidationPolicy::Unrestricted)
        .expect("exact structural failure cleanup validates");
    chunk.protos[0].failure_cleanups[0].action = crate::FailureCleanupAction::DropStructural {
        local: 0,
        place: Some(0),
        representation: crate::StructuralRepresentationId::new(1),
    };
    let actual = error(chunk);
    assert!(actual.contains("has wrong kind"), "{actual}");
}
