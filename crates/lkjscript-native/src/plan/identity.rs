use super::*;

pub(super) static NEXT_PLAN_ID: AtomicU64 = AtomicU64::new(1);

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

/// Exact runtime layout identity for a worker-local stable reference handle.
/// Layout-bearing variants leave the machine ABI unchanged while preventing
/// unrelated heap layouts from being interchanged.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReferenceType {
    Buf,
    Str,
    /// Complete interned list identity followed by its element identity.
    List(LayoutIdentity, LayoutIdentity),
    Product(LayoutIdentity),
    /// Concrete substitution identity and stable semantic runtime layout identity.
    Enum(LayoutIdentity, [u8; 32]),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum UniqueType {
    ByteVector,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LoanType {
    ByteSlice,
    ByteSliceMut,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ValueType {
    I64,
    F64,
    Bool,
    Unit,
    Capability(lkjscript_contracts::CapabilityKind),
    Resource(lkjscript_contracts::ResourceKind),
    Unique(UniqueType),
    Loan(LoanType),
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
            | Self::Capability(_)
            | Self::Resource(_)
            | Self::Unique(_)
            | Self::Loan(_) => None,
        }
    }

    /// Exact structural identity used by List/Option/Result payload facts.
    /// Nested reference variants retain their pre-interned complete identity.
    #[must_use]
    pub const fn layout_identity(self) -> LayoutIdentity {
        match self {
            Self::Unit => LayoutIdentity::new(1),
            Self::Bool => LayoutIdentity::new(2),
            Self::I64 => LayoutIdentity::new(3),
            Self::F64 => LayoutIdentity::new(4),
            Self::Reference(ReferenceType::Str) => LayoutIdentity::new(5),
            Self::Reference(ReferenceType::Buf) => LayoutIdentity::new(7),
            Self::Capability(kind) => LayoutIdentity::new(8 + kind as u32),
            Self::Resource(kind) => LayoutIdentity::new(16 + kind as u32),
            Self::Unique(UniqueType::ByteVector) => LayoutIdentity::new(28),
            Self::Loan(LoanType::ByteSlice) => LayoutIdentity::new(29),
            Self::Loan(LoanType::ByteSliceMut) => LayoutIdentity::new(30),
            Self::Reference(ReferenceType::Product(layout))
            | Self::Reference(ReferenceType::List(layout, _))
            | Self::Reference(ReferenceType::Enum(layout, _)) => layout,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signature {
    pub(super) parameters: Vec<ValueType>,
    pub(super) result: ValueType,
}

impl Signature {
    pub fn new(parameters: Vec<ValueType>, result: ValueType) -> Result<Self, PlanError> {
        if parameters.len() > 16 {
            return Err(PlanError::TooManyParameters {
                count: parameters.len(),
                maximum: 16,
            });
        }
        Ok(Self { parameters, result })
    }

    #[must_use]
    pub fn parameters(&self) -> &[ValueType] {
        &self.parameters
    }

    #[must_use]
    pub const fn result(&self) -> ValueType {
        self.result
    }

    pub(crate) fn machine_parameter_count(&self) -> usize {
        self.parameters
            .iter()
            .filter(|parameter| **parameter != ValueType::Unit)
            .count()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceFunctionId(u32);

impl SourceFunctionId {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceOrigin(u32);

impl SourceOrigin {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FunctionId {
    pub(crate) plan: u64,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BlockId {
    pub(crate) function: FunctionId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ValueId {
    pub(crate) function: FunctionId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LocalId {
    pub(crate) function: FunctionId,
    pub(crate) index: u32,
}
