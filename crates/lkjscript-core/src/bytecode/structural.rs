use super::{EnumId, ProductId, RuntimeLayoutId, VariantId};

pub const MAX_STRUCTURAL_TYPES: usize = 16_384;
pub const MAX_STRUCTURAL_LAYOUTS: usize = 16_384;
pub const MAX_STRUCTURAL_REPRESENTATIONS: usize = 65_536;
pub const MAX_STRUCTURAL_LAYOUT_FIELDS: usize = 65_536;
pub const MAX_STRUCTURAL_DESTINATIONS: usize = 65_536;
pub const MAX_STRUCTURAL_OPERATION_REFS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryPlanId([u8; 32]);

impl MemoryPlanId {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

macro_rules! structural_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
        pub struct $name(u16);

        impl $name {
            pub const fn new(raw: u16) -> Self {
                Self(raw)
            }

            pub const fn raw(self) -> u16 {
                self.0
            }

            pub const fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

structural_id!(StructuralTypeId);
structural_id!(StructuralLayoutId);
structural_id!(StructuralRepresentationId);
structural_id!(StructuralDestinationId);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StructuralTypeKind {
    String,
    Path,
    Product(ProductId),
    Enum(EnumId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructuralTypeMetadata {
    pub id: StructuralTypeId,
    pub identity: RuntimeLayoutId,
    pub runtime_type: crate::StructuralType,
    pub kind: StructuralTypeKind,
    pub layout: StructuralLayoutId,
    pub mode: StructuralTypeMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StructuralFieldRoute {
    Copy,
    Structural(StructuralTypeId),
    Unique,
    Resource,
    LegacyHeap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructuralFieldMetadata {
    pub identity: RuntimeLayoutId,
    pub runtime_type: Option<crate::StructuralType>,
    pub route: StructuralFieldRoute,
    pub resource: Option<crate::ResourceKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StructuralLayoutKind {
    String,
    Path,
    Product {
        product: ProductId,
        fields: Vec<StructuralFieldMetadata>,
    },
    Enum {
        enum_id: EnumId,
        runtime_layout: RuntimeLayoutId,
        variants: Vec<StructuralVariantLayout>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructuralVariantLayout {
    pub variant: VariantId,
    pub physical_tag: u16,
    pub fields: Vec<StructuralFieldMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructuralDestinationMetadata {
    pub id: StructuralDestinationId,
    pub representation: StructuralRepresentationId,
    pub owner_representation: StructuralRepresentationId,
    pub active_variant: Option<VariantId>,
    pub fields: Vec<StructuralFieldMetadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructuralDestinationFieldRef {
    pub destination: StructuralDestinationId,
    pub field: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructuralAggregateFieldRef {
    pub representation: StructuralRepresentationId,
    pub active_variant: Option<VariantId>,
    pub field: u16,
    pub result: StructuralFieldMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructuralPayloadRef {
    pub representation: StructuralRepresentationId,
    pub variant: VariantId,
    pub result: StructuralFieldMetadata,
}
