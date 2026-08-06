use super::{EnumId, ProductId, RuntimeLayoutId, VariantId};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct MemoryWitnessId([u8; 32]);

impl MemoryWitnessId {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn is_resolved(self) -> bool {
        self.0 != [0; 32]
    }
}

pub trait StructuralDenseId: Copy {
    fn host_index(self) -> Option<usize>;
}

pub trait StructuralSliceExt<T> {
    fn get_structural<I: StructuralDenseId>(&self, id: I) -> Option<&T>;
}

impl<T> StructuralSliceExt<T> for [T] {
    fn get_structural<I: StructuralDenseId>(&self, id: I) -> Option<&T> {
        id.host_index().and_then(|index| self.get(index))
    }
}

macro_rules! structural_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
        pub struct $name(u64);

        impl StructuralDenseId for $name {
            fn host_index(self) -> Option<usize> {
                usize::try_from(self.0).ok()
            }
        }

        impl $name {
            pub const fn new(raw: u64) -> Self {
                Self(raw)
            }

            pub const fn raw(self) -> u64 {
                self.0
            }

            pub fn index(self) -> Option<usize> {
                self.host_index()
            }

            pub fn checked_index(self) -> crate::Result<usize> {
                self.index().ok_or_else(|| {
                    crate::Error::msg("bytecode structural identity exceeds host index width")
                })
            }
        }
    };
}

structural_id!(StructuralTypeId);
structural_id!(StructuralLayoutId);
structural_id!(StructuralRepresentationId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct StructuralDestinationId(u64);

impl StructuralDestinationId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn index(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StructuralValueCategory {
    Owner,
    View,
    Destination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub witness: MemoryWitnessId,
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
    pub source_order: u64,
    pub physical_tag: u64,
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
    pub witness: MemoryWitnessId,
    pub witness_group: super::MemoryWitnessGroupId,
    pub witness_member: u64,
    pub layout: StructuralLayoutId,
    pub category: StructuralValueCategory,
    pub storage: StructuralStorage,
    pub route: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructuralDestinationMetadata {
    pub id: StructuralDestinationId,
    pub representation: StructuralRepresentationId,
    pub owner_representation: StructuralRepresentationId,
    pub active_variant: Option<VariantId>,
    pub fields: Vec<StructuralFieldMetadata>,
}

include!("structural/references.rs");
