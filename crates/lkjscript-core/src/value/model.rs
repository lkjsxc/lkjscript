//! Closed typed runtime values.

use super::CapabilityKind;

mod bytes;
mod list;
mod region_product;
mod scalar;
mod structural;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum ValueKind {
    Invalid,
    Unit,
    Bool,
    I64,
    F64,
    EmptyList,
    SegmentedList,
    OwnedList,
    RegionProduct,
    Capability,
    Resource,
    FunctionPrototype,
    Function,
    Symbol,
    AggregateAdapter,
    StructuralRoot,
    StructuralView,
    StructuralDestination,
    StaticString,
    StaticBytes,
    BytesKey,
    ByteVectorKey,
    BytesBorrow,
    ByteSlice,
    ByteSliceMut,
}

/// Safe closed value storage with one exact payload and an explicit category.
///
/// The C layout is intentionally 16 bytes on supported targets: an eight-byte
/// payload followed by closed metadata and padding. Private fields prevent one
/// category from being reinterpreted as another.
#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(C)]
pub struct Value {
    payload: u64,
    kind: ValueKind,
}

impl Value {
    pub const INVALID: Self = Self::new(ValueKind::Invalid, 0);
    pub const UNIT: Self = Self::new(ValueKind::Unit, 0);
    pub const EMPTY_LIST: Self = Self::new(ValueKind::EmptyList, 0);
    pub const FALSE: Self = Self::new(ValueKind::Bool, 0);
    pub const TRUE: Self = Self::new(ValueKind::Bool, 1);

    const fn new(kind: ValueKind, payload: u64) -> Self {
        Self { payload, kind }
    }

    pub const fn from_bool(value: bool) -> Self {
        if value {
            Self::TRUE
        } else {
            Self::FALSE
        }
    }

    pub const fn from_i64(value: i64) -> Self {
        Self::new(ValueKind::I64, value as u64)
    }

    pub const fn from_f64_bits(bits: u64) -> Self {
        Self::new(ValueKind::F64, bits)
    }

    pub const fn from_static_string(index: u64) -> Self {
        Self::new(ValueKind::StaticString, index)
    }

    pub const fn as_static_string(self) -> Option<u64> {
        match self.kind {
            ValueKind::StaticString => Some(self.payload),
            _ => None,
        }
    }

    pub const fn from_resource(token: u64) -> Self {
        Self::new(ValueKind::Resource, token)
    }

    #[doc(hidden)]
    pub const fn from_function_prototype(prototype: u64) -> Self {
        Self::new(ValueKind::FunctionPrototype, prototype)
    }

    #[doc(hidden)]
    pub const fn as_function_prototype(self) -> Option<u64> {
        match self.kind {
            ValueKind::FunctionPrototype => Some(self.payload),
            _ => None,
        }
    }

    pub(crate) const fn from_function(prototype: u64) -> Self {
        Self::new(ValueKind::Function, prototype)
    }

    pub(crate) const fn from_symbol(constant: u64) -> Self {
        Self::new(ValueKind::Symbol, constant)
    }

    pub const fn from_capability(kind: CapabilityKind) -> Self {
        Self::new(ValueKind::Capability, kind as u64)
    }

    pub const fn is_invalid(self) -> bool {
        matches!(self.kind, ValueKind::Invalid)
    }

    pub const fn is_unit(self) -> bool {
        matches!(self.kind, ValueKind::Unit)
    }

    pub const fn is_empty_list(self) -> bool {
        matches!(self.kind, ValueKind::EmptyList)
    }

    pub const fn as_bool(self) -> Option<bool> {
        match self.kind {
            ValueKind::Bool => Some(self.payload != 0),
            _ => None,
        }
    }

    pub const fn as_i64(self) -> Option<i64> {
        match self.kind {
            ValueKind::I64 => Some(self.payload as i64),
            _ => None,
        }
    }

    pub const fn as_f64_bits(self) -> Option<u64> {
        match self.kind {
            ValueKind::F64 => Some(self.payload),
            _ => None,
        }
    }

    pub fn as_f64(self) -> Option<f64> {
        self.as_f64_bits().map(f64::from_bits)
    }

    pub const fn as_resource(self) -> Option<u64> {
        match self.kind {
            ValueKind::Resource => Some(self.payload),
            _ => None,
        }
    }

    pub fn as_capability(self) -> Option<CapabilityKind> {
        match self.kind {
            ValueKind::Capability => u8::try_from(self.payload)
                .ok()
                .and_then(CapabilityKind::from_tag),
            _ => None,
        }
    }
}
