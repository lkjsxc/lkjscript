use super::*;

fn identity(byte: u8) -> crate::RuntimeLayoutId {
    crate::RuntimeLayoutId::new([byte; 32])
}

fn witness(byte: u8) -> crate::MemoryWitnessId {
    crate::MemoryWitnessId::new([byte; 32])
}

fn runtime_type(id: u64, kind: crate::StructuralKind) -> crate::StructuralType {
    crate::StructuralType::new(
        crate::LayoutIdentity::new(std::num::NonZeroU64::new(id).expect("test layout identity")),
        crate::SemanticTypeIdentity::new(
            std::num::NonZeroU64::new(id + 100).expect("test semantic identity"),
        ),
        kind,
    )
}

fn copy_field() -> crate::StructuralFieldMetadata {
    crate::StructuralFieldMetadata {
        identity: identity(3),
        runtime_type: Some(runtime_type(3, crate::StructuralKind::I64)),
        route: crate::StructuralFieldRoute::Copy,
        resource: None,
    }
}

fn product_chunk() -> Chunk {
    let mut chunk = Chunk::new();
    let plan = crate::MemoryPlanId::new([7; 32]);
    let product_identity = crate::RuntimeLayoutId::new([9; 32]);
    chunk.memory_plan = Some(plan);
    chunk.main.memory_plan = Some(plan);
    chunk.products = vec![crate::ProductMetadata {
        id: crate::ProductId::new(0),
        identity: product_identity,
        region: false,
        name: "product".into(),
        fields: vec!["value".into()],
        region_fields: Vec::new(),
    }];
    chunk.structural_layouts = vec![crate::StructuralLayoutMetadata {
        id: crate::StructuralLayoutId::new(0),
        identity: identity(1),
        kind: crate::StructuralLayoutKind::Product {
            product: crate::ProductId::new(0),
            fields: vec![copy_field()],
        },
    }];
    chunk.structural_types = vec![crate::StructuralTypeMetadata {
        id: crate::StructuralTypeId::new(0),
        witness: witness(1),
        identity: identity(2),
        runtime_type: crate::StructuralType::new(
            crate::product_layout_identity(product_identity),
            crate::product_semantic_identity(product_identity),
            crate::StructuralKind::Product,
        ),
        kind: crate::StructuralTypeKind::Product(crate::ProductId::new(0)),
        layout: crate::StructuralLayoutId::new(0),
        mode: crate::StructuralTypeMode::Affine,
    }];
    chunk.structural_representations = vec![
        crate::StructuralRepresentationMetadata {
            id: crate::StructuralRepresentationId::new(0),
            type_id: crate::StructuralTypeId::new(0),
            witness: witness(1),
            witness_group: crate::MemoryWitnessGroupId::new([0; 32]),
            witness_member: 0,
            layout: crate::StructuralLayoutId::new(0),
            category: crate::StructuralValueCategory::Owner,
            storage: crate::StructuralStorage::UniqueStructural,
            route: [1; 32],
        },
        crate::StructuralRepresentationMetadata {
            id: crate::StructuralRepresentationId::new(1),
            type_id: crate::StructuralTypeId::new(0),
            witness: witness(1),
            witness_group: crate::MemoryWitnessGroupId::new([0; 32]),
            witness_member: 0,
            layout: crate::StructuralLayoutId::new(0),
            category: crate::StructuralValueCategory::View,
            storage: crate::StructuralStorage::BorrowedView,
            route: [2; 32],
        },
        crate::StructuralRepresentationMetadata {
            id: crate::StructuralRepresentationId::new(2),
            type_id: crate::StructuralTypeId::new(0),
            witness: witness(1),
            witness_group: crate::MemoryWitnessGroupId::new([0; 32]),
            witness_member: 0,
            layout: crate::StructuralLayoutId::new(0),
            category: crate::StructuralValueCategory::Destination,
            storage: crate::StructuralStorage::UniqueStructural,
            route: [1; 32],
        },
    ];
    chunk.structural_destinations = vec![crate::StructuralDestinationMetadata {
        id: crate::StructuralDestinationId::new(0),
        representation: crate::StructuralRepresentationId::new(2),
        owner_representation: crate::StructuralRepresentationId::new(0),
        active_variant: None,
        fields: vec![copy_field()],
    }];
    chunk.structural_destination_fields = vec![crate::StructuralDestinationFieldRef {
        destination: crate::StructuralDestinationId::new(0),
        field: 0,
    }];
    chunk
}

fn emit_finished_product(chunk: &mut Chunk) {
    chunk.main.locals = 3;
    chunk.main.unique_places = 1;
    let value = chunk
        .add_const(crate::Constant::I64(11))
        .expect("add value constant");
    chunk.main.emit_op_u64(Op::StructuralDestinationCreate, 0);
    chunk.main.emit_op_u8(Op::StoreStructuralLocal, 0);
    chunk.main.emit_op_u8(Op::TakeStructuralLocal, 0);
    chunk.main.emit_op_u64(Op::LoadConst, value.0);
    chunk
        .main
        .emit_op_u64(Op::StructuralDestinationFieldInit, 0);
    chunk.main.emit_op_u8(Op::StoreStructuralLocal, 0);
    chunk.main.emit_op_u8(Op::TakeStructuralLocal, 0);
    chunk.main.emit_op_u64(Op::StructuralDestinationFinish, 0);
    chunk.main.emit_op_u8(Op::StoreStructuralLocal, 1);
    chunk.main.emit_op_u64_pair(Op::StructuralPlaceInit, 0, 1);
    chunk.main.emit(Op::Pop);
}

fn emit_product_cleanup(chunk: &mut Chunk) {
    chunk.main.emit_op_u64_pair(Op::StructuralDropPlace, 0, 1);
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u8(Op::StructuralPlaceEnd, 0);
    chunk.main.emit(Op::Pop);
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
}

include!("identity.rs");
include!("ownership.rs");
include!("cleanup.rs");
include!("authenticated_return.rs");
include!("authenticated_enum.rs");
include!("semantic_dag.rs");
include!("witness_authentication.rs");
include!("witness_closure_rejection.rs");
