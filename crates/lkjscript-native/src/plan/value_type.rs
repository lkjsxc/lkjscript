use super::*;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LayoutIdentity(u32);

impl LayoutIdentity {
    const PRODUCT_BASE: u32 = 32;

    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn product(product: u32) -> Self {
        Self(Self::PRODUCT_BASE + product)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReferenceType {
    List(LayoutIdentity, LayoutIdentity),
    Product(LayoutIdentity),
    Enum(LayoutIdentity, [u8; 32]),
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
        matches!(
            self,
            Self::StructuralOwner(_) | Self::StructuralView(_) | Self::StructuralDestination(_)
        )
    }

    #[must_use]
    pub const fn layout_identity(self) -> LayoutIdentity {
        match self {
            Self::Unit => LayoutIdentity::new(1),
            Self::Bool => LayoutIdentity::new(2),
            Self::I64 => LayoutIdentity::new(3),
            Self::F64 => LayoutIdentity::new(4),
            Self::StaticBytes => LayoutIdentity::new(6),
            Self::StaticString(value_type) | Self::StructuralOwner(value_type) => {
                LayoutIdentity::new(value_type.layout() as u32)
            }
            Self::StructuralView(view) => LayoutIdentity::new(view.projected().layout() as u32),
            Self::StructuralDestination(destination) => {
                LayoutIdentity::new(destination.value_type().layout() as u32)
            }
            Self::Capability(kind) => LayoutIdentity::new(8 + kind as u32),
            Self::Resource(kind) => LayoutIdentity::new(16 + kind as u32),
            Self::Loan(LoanType::Bytes) => LayoutIdentity::new(27),
            Self::Unique(UniqueType::ByteVector) => LayoutIdentity::new(28),
            Self::Loan(LoanType::ByteSlice) => LayoutIdentity::new(29),
            Self::Loan(LoanType::ByteSliceMut) => LayoutIdentity::new(30),
            Self::Unique(UniqueType::Bytes) => LayoutIdentity::new(31),
            Self::Reference(ReferenceType::Product(layout))
            | Self::Reference(ReferenceType::List(layout, _))
            | Self::Reference(ReferenceType::Enum(layout, _)) => layout,
        }
    }
}
