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
    Scalar,
    Reference,
    ReferenceClearing,
}

/// Host-independent heap semantics retained as exact versioned runtime-site
/// identity. Literal bytes and nominal identities are bounded image metadata,
/// never source pointers.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum HeapOperation {
    ConstantStr(String),
    EmptyStr,
    EmptyList,
    None,
    ProductValue {
        product: u32,
        fields: u8,
    },
    ProductField {
        product: u32,
        field: u8,
        field_type: ValueType,
    },
    WithProductField {
        product: u32,
        field: u8,
        field_type: ValueType,
    },
    EnumValue {
        enum_id: [u8; 32],
        variant: [u8; 32],
        layout: [u8; 32],
        physical_tag: u16,
        substitutions: Vec<LayoutIdentity>,
        fields: u8,
    },
    EnumIsVariant {
        enum_id: [u8; 32],
        variant: [u8; 32],
        layout: [u8; 32],
        physical_tag: u16,
    },
    EnumField {
        enum_id: [u8; 32],
        variant: [u8; 32],
        field: [u8; 32],
        layout: [u8; 32],
        physical_tag: u16,
        field_index: u8,
        field_type: ValueType,
    },
    Cons,
    Car,
    Cdr,
    IsEmptyList,
    Some,
    IsSome,
    UnwrapSome,
    Ok,
    Err,
    IsOk,
    UnwrapOk,
    UnwrapErr,
    BufNew,
    BufLen,
    BufRef,
    BufSet,
    BufClone,
    BufFromStr,
    BufToStr,
    BufSlice,
    BufGetU32,
    BufSetU32,
    StrLen,
    StrRef,
    StrAppend,
    StrSlice,
    StrFromByte,
    StrFromI64,
    StrFromF64,
    F64FromI64Exact,
    F64FromI64Rounded,
    I64FromF64Exact,
    I64FromF64Trunc,
    EqualValue,
    SameObject,
    ListEqual,
}

impl HeapOperation {
    pub(crate) fn expected_arity(&self) -> usize {
        match self {
            Self::EmptyStr | Self::EmptyList | Self::None | Self::ConstantStr(_) => 0,
            Self::ProductValue { fields, .. } | Self::EnumValue { fields, .. } => {
                usize::from(*fields)
            }
            Self::BufSet | Self::BufSlice | Self::BufSetU32 | Self::StrSlice => 3,
            Self::ProductField { .. }
            | Self::EnumIsVariant { .. }
            | Self::EnumField { .. }
            | Self::Car
            | Self::Cdr
            | Self::IsEmptyList
            | Self::Some
            | Self::IsSome
            | Self::UnwrapSome
            | Self::Ok
            | Self::Err
            | Self::IsOk
            | Self::UnwrapOk
            | Self::UnwrapErr
            | Self::BufNew
            | Self::BufLen
            | Self::BufClone
            | Self::BufFromStr
            | Self::BufToStr
            | Self::StrLen
            | Self::StrFromByte
            | Self::StrFromI64
            | Self::StrFromF64
            | Self::F64FromI64Exact
            | Self::F64FromI64Rounded
            | Self::I64FromF64Exact
            | Self::I64FromF64Trunc => 1,
            Self::WithProductField { .. }
            | Self::Cons
            | Self::BufRef
            | Self::BufGetU32
            | Self::StrRef
            | Self::StrAppend
            | Self::EqualValue
            | Self::SameObject
            | Self::ListEqual => 2,
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
