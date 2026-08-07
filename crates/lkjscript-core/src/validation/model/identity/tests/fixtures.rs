use crate::*;
use lkjscript_contracts::{
    ExecutableMemoryWitnessFacts, MemoryWitnessCapabilities, MemoryWitnessContention,
    MemoryWitnessCopy, MemoryWitnessDomain, MemoryWitnessDrop, MemoryWitnessEquality,
    MemoryWitnessListElement, MemoryWitnessMode, MemoryWitnessOperation, MemoryWitnessPortability,
    MemoryWitnessRoot, MemoryWitnessSize, MemoryWitnessSnapshot, SemanticDescriptor,
    SemanticPrimitiveKind, SemanticType,
};
use std::num::NonZeroU64;

pub(super) fn witness() -> InstalledMemoryWitness {
    let semantic = SemanticDescriptor {
        root: SemanticType::Primitive(SemanticPrimitiveKind::I64),
        declarations: Vec::new(),
    };
    let facts = ExecutableMemoryWitnessFacts {
        semantic_type: lkjscript_contracts::semantic_type_closure_hash(&semantic)
            .expect("semantic type"),
        semantic_contract: lkjscript_contracts::semantic_contract_hash(&semantic)
            .expect("semantic contract"),
        semantic,
        mode: MemoryWitnessMode::Copy,
        capabilities: MemoryWitnessCapabilities {
            inline: true,
            static_value: false,
            unique: false,
            ordinary_region: false,
            sealed_region: false,
            borrow: true,
            semantic_snapshot: true,
            list_element: true,
            equality: true,
        },
        domain: MemoryWitnessDomain::Inline,
        root: MemoryWitnessRoot::None,
        copy: MemoryWitnessCopy::Trivial,
        drop: MemoryWitnessDrop::Trivial,
        equality: MemoryWitnessEquality::Value,
        snapshot: MemoryWitnessSnapshot::Eligible,
        list_element: MemoryWitnessListElement::Copy,
        size: MemoryWitnessSize::Fixed(8),
        alignment: 8,
        contains_borrow: false,
        contains_dynamic_owner: false,
        portability: MemoryWitnessPortability::Portable,
        contention: MemoryWitnessContention::None,
        operations: vec![MemoryWitnessOperation::Transport],
    };
    let member = lkjscript_contracts::ExecutableMemoryWitnessGroupMember {
        id: [0; 32],
        ordinal: 0,
        semantic_identity: facts.semantic_type,
        facts: facts.clone(),
        dependencies: Vec::new(),
    };
    let group = lkjscript_contracts::executable_memory_witness_group_id(false, &[member]);
    let id =
        lkjscript_contracts::executable_memory_witness_member_id(group, 0, facts.semantic_type);
    InstalledMemoryWitness {
        id: MemoryWitnessId::new(id),
        group: MemoryWitnessGroupId::new(group),
        ordinal: 0,
        facts,
        dependencies: Vec::new(),
        value_kind: MemoryWitnessValueKind::I64,
    }
}
pub(super) fn witness_group() -> InstalledMemoryWitnessGroup {
    let witness = witness();
    InstalledMemoryWitnessGroup {
        id: witness.group,
        recursive: false,
        members: vec![InstalledMemoryWitnessGroupMember {
            witness: witness.id,
            ordinal: witness.ordinal,
            semantic_identity: witness.facts.semantic_type,
        }],
    }
}

pub(super) fn runtime_type() -> StructuralType {
    StructuralType::new(
        LayoutIdentity::new(NonZeroU64::MIN),
        SemanticTypeIdentity::new(NonZeroU64::MIN),
        StructuralKind::String,
    )
}
pub(super) fn field() -> StructuralFieldMetadata {
    StructuralFieldMetadata {
        identity: RuntimeLayoutId::new([9; 32]),
        runtime_type: Some(runtime_type()),
        route: StructuralFieldRoute::Copy,
        resource: None,
    }
}
pub(super) fn enumeration() -> EnumMetadata {
    EnumMetadata {
        id: EnumId::new([4; 32]),
        name: "choice".into(),
        type_parameter_count: 0,
        layout: RuntimeLayoutId::new([5; 32]),
        variants: vec![EnumVariantMetadata {
            id: VariantId::new([6; 32]),
            name: "only".into(),
            source_order: 0,
            physical_tag: 0,
            fields: vec![EnumFieldMetadata {
                id: VariantFieldId::new([7; 32]),
                name: "value".into(),
            }],
        }],
    }
}
