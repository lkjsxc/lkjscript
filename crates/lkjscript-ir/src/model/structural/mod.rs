use super::*;

mod identity;
mod memory;
mod witness;
pub use identity::{
    runtime_product_contract_identity, runtime_product_identity, runtime_product_layout_identity,
    runtime_product_semantic_type, runtime_structural_semantic_type, runtime_structural_type,
};
pub use witness::{
    MemoryWitnessDescriptor, MemoryWitnessGroupDescriptor, MemoryWitnessGroupMember,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionProductMetadata {
    pub product: ProductId,
    pub identity: RuntimeLayoutId,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StructuralValueCategory {
    Owner,
    View,
    Destination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StructuralStorage {
    Inline,
    Static,
    Stack,
    CallerDestination,
    UniqueStructural,
    OrdinaryRegion,
    SealedRegion,
    BorrowedView,
    ExternalResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StructuralTypeMode {
    Copy,
    Immutable,
    Affine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralTypeMetadata {
    pub id: StructuralTypeId,
    pub witness: MemoryWitnessId,
    pub ty: SsaType,
    pub layout: StructuralLayoutId,
    pub mode: StructuralTypeMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuralLayoutKind {
    String,
    Path,
    Product {
        product: ProductId,
        fields: Vec<SsaType>,
    },
    Enum {
        enum_id: EnumId,
        runtime_layout: RuntimeLayoutId,
        variants: Vec<StructuralVariantLayout>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralVariantLayout {
    pub variant: VariantId,
    pub source_order: u64,
    pub physical_tag: u64,
    pub fields: Vec<SsaType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralLayoutMetadata {
    pub id: StructuralLayoutId,
    pub identity: RuntimeLayoutId,
    pub kind: StructuralLayoutKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructuralRepresentationMetadata {
    pub id: StructuralRepresentationId,
    pub type_id: StructuralTypeId,
    pub witness: MemoryWitnessId,
    pub witness_group: MemoryWitnessGroupId,
    pub witness_member: u64,
    pub layout: StructuralLayoutId,
    pub category: StructuralValueCategory,
    pub storage: StructuralStorage,
    pub route: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralMemoryMetadata {
    pub plan: MemoryPlanId,
    pub witness_groups: Vec<MemoryWitnessGroupDescriptor>,
    pub witnesses: Vec<MemoryWitnessDescriptor>,
    pub types: Vec<StructuralTypeMetadata>,
    pub layouts: Vec<StructuralLayoutMetadata>,
    pub representations: Vec<StructuralRepresentationMetadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StructuralDropGlueIdentity {
    String {
        type_id: StructuralTypeId,
        layout: StructuralLayoutId,
    },
    Path {
        type_id: StructuralTypeId,
        layout: StructuralLayoutId,
    },
    Product {
        type_id: StructuralTypeId,
        product: ProductId,
        layout: StructuralLayoutId,
    },
    Enum {
        type_id: StructuralTypeId,
        enum_id: EnumId,
        layout: StructuralLayoutId,
        runtime_layout: RuntimeLayoutId,
    },
    Destination {
        type_id: StructuralTypeId,
        layout: StructuralLayoutId,
    },
}
