use super::*;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AllocationClass {
    None,
    Bounded,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StoreClass {
    None,
    Initialization,
}

/// Host-independent heap semantics retained as exact versioned runtime-site
/// identity. Literal bytes and nominal identities are bounded image metadata,
/// never source pointers.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum HeapOperation {
    EmptyList,
    ProductValue {
        product: u64,
        fields: u64,
    },
    ProductField {
        product: u64,
        field: u64,
        field_type: ValueType,
    },
    WithProductField {
        product: u64,
        field: u64,
        field_type: ValueType,
    },
    Cons,
    Car,
    Cdr,
    IsEmptyList,
    ListEqual,
}

impl HeapOperation {
    pub(crate) fn expected_arity(&self) -> Option<usize> {
        match self {
            Self::EmptyList => Some(0),
            Self::ProductValue { fields, .. } => usize::try_from(*fields).ok(),
            Self::ProductField { .. } | Self::Car | Self::Cdr | Self::IsEmptyList => Some(1),
            Self::WithProductField { .. } | Self::Cons | Self::ListEqual => Some(2),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct HeapCallDescriptor {
    pub(super) operation: HeapOperation,
    pub(super) input_types: Vec<ValueType>,
    pub(super) result_type: ValueType,
    pub(super) allocation: AllocationClass,
    pub(super) store: StoreClass,
}
