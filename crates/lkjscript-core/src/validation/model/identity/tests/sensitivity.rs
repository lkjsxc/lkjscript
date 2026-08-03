use super::{fixtures, *};
use crate::*;

fn raw(value: &Chunk) -> ValidatedBytecodeIdentity {
    let mut out = Encoder::new(DOMAIN);
    encode_chunk(&mut out, value);
    ValidatedBytecodeIdentity(out.finish().expect("raw fixture identity"))
}
fn changed(mut change: impl FnMut(&mut Chunk)) {
    let baseline = Chunk::new();
    let before = raw(&baseline);
    let mut after = baseline;
    change(&mut after);
    assert!(before != raw(&after));
}

#[test]
fn every_chunk_table_changes_identity() {
    changed(|chunk| chunk.constants.push(Constant::I64(1)));
    changed(|chunk| {
        let mut proto = Chunk::new().main;
        proto.name = "prototype".into();
        chunk.protos.push(proto);
    });
    changed(|chunk| chunk.main.name = "entry".into());
    changed(|chunk| chunk.memory_plan = Some(MemoryPlanId::new([1; 32])));
    changed(|chunk| chunk.memory_witness_groups.push(fixtures::witness_group()));
    changed(|chunk| chunk.memory_witnesses.push(fixtures::witness()));
    changed(|chunk| {
        chunk.structural_types.push(StructuralTypeMetadata {
            id: StructuralTypeId::new(0),
            witness: MemoryWitnessId::new([2; 32]),
            identity: RuntimeLayoutId::new([3; 32]),
            runtime_type: fixtures::runtime_type(),
            kind: StructuralTypeKind::String,
            layout: StructuralLayoutId::new(0),
            mode: StructuralTypeMode::Copy,
        })
    });
    changed(|chunk| {
        chunk.structural_layouts.push(StructuralLayoutMetadata {
            id: StructuralLayoutId::new(0),
            identity: RuntimeLayoutId::new([3; 32]),
            kind: StructuralLayoutKind::String,
        })
    });
    changed(|chunk| {
        chunk
            .structural_representations
            .push(StructuralRepresentationMetadata {
                id: StructuralRepresentationId::new(0),
                type_id: StructuralTypeId::new(0),
                witness: MemoryWitnessId::new([1; 32]),
                witness_group: MemoryWitnessGroupId::new([2; 32]),
                witness_member: 0,
                layout: StructuralLayoutId::new(0),
                category: StructuralValueCategory::Owner,
                storage: StructuralStorage::Stack,
                route: [3; 32],
            })
    });
    changed(|chunk| {
        chunk
            .structural_destinations
            .push(StructuralDestinationMetadata {
                id: StructuralDestinationId::new(0),
                representation: StructuralRepresentationId::new(0),
                owner_representation: StructuralRepresentationId::new(1),
                active_variant: None,
                fields: vec![fixtures::field()],
            })
    });
    changed(|chunk| {
        chunk
            .structural_destination_fields
            .push(StructuralDestinationFieldRef {
                destination: StructuralDestinationId::new(0),
                field: 1,
            })
    });
    changed(|chunk| {
        chunk
            .structural_aggregate_fields
            .push(StructuralAggregateFieldRef {
                representation: StructuralRepresentationId::new(0),
                active_variant: None,
                field: 1,
                result: fixtures::field(),
                result_representation: None,
            })
    });
    changed(|chunk| {
        chunk.structural_payloads.push(StructuralPayloadRef {
            representation: StructuralRepresentationId::new(0),
            variant: VariantId::new([6; 32]),
            result: fixtures::field(),
            result_representation: None,
        })
    });
    changed(|chunk| chunk.required_capabilities.push(CapabilityKind::Clock));
    changed(|chunk| chunk.global_names.push("global".into()));
    changed(|chunk| chunk.global_prototypes.push(Some(0)));
    changed(|chunk| {
        chunk.products.push(ProductMetadata {
            id: ProductId::new(0),
            identity: RuntimeLayoutId::new([8; 32]),
            region: false,
            name: "point".into(),
            fields: vec!["x".into()],
            region_fields: vec![RegionProductFieldKind::I64],
        })
    });
    changed(|chunk| {
        chunk.product_fields.push(ProductFieldRef {
            product: ProductId::new(0),
            field: 0,
        })
    });
    changed(|chunk| chunk.enums.push(fixtures::enumeration()));
    changed(|chunk| {
        chunk.enum_constructions.push(EnumConstructionRef {
            enum_id: EnumId::new([4; 32]),
            variant: VariantId::new([6; 32]),
            layout: RuntimeLayoutId::new([5; 32]),
            substitution_arity: 0,
        })
    });
    changed(|chunk| {
        chunk.enum_variants.push(EnumVariantRef {
            enum_id: EnumId::new([4; 32]),
            variant: VariantId::new([6; 32]),
            layout: RuntimeLayoutId::new([5; 32]),
        })
    });
    changed(|chunk| {
        chunk.enum_fields.push(EnumFieldRef {
            enum_id: EnumId::new([4; 32]),
            variant: VariantId::new([6; 32]),
            field: VariantFieldId::new([7; 32]),
            layout: RuntimeLayoutId::new([5; 32]),
        })
    });
}

#[test]
fn semantic_table_order_changes_identity() {
    let mut first = Chunk::new();
    first.constants = vec![Constant::I64(1), Constant::I64(2)];
    let mut second = Chunk::new();
    second.constants = vec![Constant::I64(2), Constant::I64(1)];
    assert!(raw(&first) != raw(&second));
}
