use super::*;

mod identity;
mod witness;
pub use identity::{
    runtime_product_contract_identity, runtime_product_identity, runtime_product_layout_identity,
    runtime_product_semantic_type, runtime_structural_semantic_type, runtime_structural_type,
};
pub use witness::{
    MemoryWitnessDescriptor, MAX_MEMORY_WITNESSES, MAX_MEMORY_WITNESS_DEPENDENCIES,
    MAX_MEMORY_WITNESS_PARAMETERS,
};

pub const MAX_STRUCTURAL_TYPES: usize = 16_384;
pub const MAX_REGION_PRODUCTS: usize = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionProductMetadata {
    pub product: ProductId,
    pub identity: RuntimeLayoutId,
}
pub const MAX_STRUCTURAL_LAYOUTS: usize = 16_384;
pub const MAX_STRUCTURAL_REPRESENTATIONS: usize = 65_536;
pub const MAX_STRUCTURAL_LAYOUT_FIELDS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StructuralValueCategory {
    Owner,
    View,
    Destination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StructuralStorage {
    Static,
    Unique,
    Stack,
    CallerDestination,
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
    pub physical_tag: u16,
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
    pub layout: StructuralLayoutId,
    pub category: StructuralValueCategory,
    pub storage: StructuralStorage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralMemoryMetadata {
    pub plan: MemoryPlanId,
    pub witnesses: Vec<MemoryWitnessDescriptor>,
    pub types: Vec<StructuralTypeMetadata>,
    pub layouts: Vec<StructuralLayoutMetadata>,
    pub representations: Vec<StructuralRepresentationMetadata>,
}

impl Default for StructuralMemoryMetadata {
    fn default() -> Self {
        Self {
            plan: MemoryPlanId::new([0; 32]),
            witnesses: Vec::new(),
            types: Vec::new(),
            layouts: Vec::new(),
            representations: Vec::new(),
        }
    }
}

impl StructuralMemoryMetadata {
    pub fn witness(&self, id: MemoryWitnessId) -> Option<&MemoryWitnessDescriptor> {
        self.witnesses
            .binary_search_by_key(&id, |item| item.id)
            .ok()
            .and_then(|index| self.witnesses.get(index))
    }

    pub fn type_for(&self, ty: &SsaType) -> Option<&StructuralTypeMetadata> {
        self.types.iter().find(|item| &item.ty == ty)
    }

    pub fn representation(
        &self,
        ty: &SsaType,
        category: StructuralValueCategory,
    ) -> Option<StructuralRepresentationId> {
        let type_id = self.type_for(ty)?.id;
        self.representations
            .iter()
            .find(|item| item.type_id == type_id && item.category == category)
            .map(|item| item.id)
    }

    pub fn is_owned(&self, ty: &SsaType) -> bool {
        self.type_for(ty)
            .is_some_and(|item| item.mode != StructuralTypeMode::Copy)
    }

    pub fn is_immutable(&self, ty: &SsaType) -> bool {
        self.type_for(ty)
            .is_some_and(|item| item.mode == StructuralTypeMode::Immutable)
    }
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
