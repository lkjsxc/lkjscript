use super::*;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LayoutIdentity {
    Unit,
    Bool,
    I64,
    F64,
    StructuralKey,
    StaticBytes,
    MemoryWitnessLocator,
    Structural(u64),
    Capability(lkjscript_contracts::CapabilityKind),
    Resource(lkjscript_contracts::ResourceKind),
    LoanBytes,
    UniqueByteVector,
    LoanByteSlice,
    LoanByteSliceMut,
    UniqueBytes,
    Product(u64),
}

impl LayoutIdentity {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self::Structural(value)
    }

    #[must_use]
    pub const fn product(product: u64) -> Self {
        Self::Product(product)
    }

    #[must_use]
    pub const fn product_id(self) -> Option<u64> {
        match self {
            Self::Product(value) => Some(value),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReferenceType {
    List(LayoutIdentity, u64, LayoutIdentity, u64),
    RegionProduct(LayoutIdentity, [u8; 32]),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum UniqueType {
    ByteVector,
    Bytes,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LoanType {
    ByteSlice,
    ByteSliceMut,
    Bytes,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ValueType {
    I64,
    F64,
    Bool,
    Unit,
    StaticBytes,
    StaticString(StructuralTypeIdentity),
    Capability(lkjscript_contracts::CapabilityKind),
    Resource(lkjscript_contracts::ResourceKind),
    Unique(UniqueType),
    Loan(LoanType),
    StructuralKey,
    MemoryWitnessLocator,
    StructuralOwner(StructuralTypeIdentity),
    StructuralView(StructuralViewType),
    StructuralDestination(StructuralDestinationType),
    Reference(ReferenceType),
}

impl ValueType {
    #[must_use]
    pub const fn reference_type(self) -> Option<ReferenceType> {
        match self {
            Self::Reference(reference_type) => Some(reference_type),
            Self::I64
            | Self::F64
            | Self::Bool
            | Self::Unit
            | Self::StaticBytes
            | Self::StaticString(_)
            | Self::Capability(_)
            | Self::Resource(_)
            | Self::Unique(_)
            | Self::Loan(_)
            | Self::StructuralKey
            | Self::MemoryWitnessLocator
            | Self::StructuralOwner(_)
            | Self::StructuralView(_)
            | Self::StructuralDestination(_) => None,
        }
    }

    #[must_use]
    pub const fn is_structural_owner(self) -> bool {
        matches!(self, Self::StructuralOwner(_))
    }

    #[must_use]
    pub const fn is_affine(self) -> bool {
        match self {
            Self::StructuralOwner(value_type) => !value_type.copyable(),
            Self::StructuralKey | Self::StructuralView(_) | Self::StructuralDestination(_) => true,
            Self::MemoryWitnessLocator => false,
            _ => false,
        }
    }

    #[must_use]
    pub const fn layout_identity(self) -> LayoutIdentity {
        match self {
            Self::Unit => LayoutIdentity::Unit,
            Self::Bool => LayoutIdentity::Bool,
            Self::I64 => LayoutIdentity::I64,
            Self::F64 => LayoutIdentity::F64,
            Self::StructuralKey => LayoutIdentity::StructuralKey,
            Self::StaticBytes => LayoutIdentity::StaticBytes,
            Self::MemoryWitnessLocator => LayoutIdentity::MemoryWitnessLocator,
            Self::StaticString(value_type) | Self::StructuralOwner(value_type) => {
                LayoutIdentity::Structural(value_type.layout())
            }
            Self::StructuralView(view) => LayoutIdentity::Structural(view.projected().layout()),
            Self::StructuralDestination(destination) => {
                LayoutIdentity::Structural(destination.value_type().layout())
            }
            Self::Capability(kind) => LayoutIdentity::Capability(kind),
            Self::Resource(kind) => LayoutIdentity::Resource(kind),
            Self::Loan(LoanType::Bytes) => LayoutIdentity::LoanBytes,
            Self::Unique(UniqueType::ByteVector) => LayoutIdentity::UniqueByteVector,
            Self::Loan(LoanType::ByteSlice) => LayoutIdentity::LoanByteSlice,
            Self::Loan(LoanType::ByteSliceMut) => LayoutIdentity::LoanByteSliceMut,
            Self::Unique(UniqueType::Bytes) => LayoutIdentity::UniqueBytes,
            Self::Reference(ReferenceType::RegionProduct(layout, _))
            | Self::Reference(ReferenceType::List(layout, _, _, _)) => layout,
        }
    }
}
