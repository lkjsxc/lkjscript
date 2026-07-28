//! Closed typed runtime values.

use std::fmt;

use super::CapabilityKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum ValueKind {
    Invalid,
    Unit,
    Bool,
    I64,
    F64,
    EmptyList,
    Capability,
    Resource,
    LegacyTraced,
    OpaqueUniqueKey,
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

    pub const fn from_legacy_traced(index: u32) -> Self {
        Self::new(ValueKind::LegacyTraced, index as u64)
    }

    pub const fn from_resource(index: u32) -> Self {
        Self::new(ValueKind::Resource, index as u64)
    }

    pub const fn from_capability(kind: CapabilityKind) -> Self {
        Self::new(ValueKind::Capability, kind as u64)
    }

    /// Runtime-only storage for identities minted by an external uniqueness
    /// authority. This category has no source constructor.
    #[doc(hidden)]
    pub const fn from_opaque_unique_key(key: u64) -> Self {
        Self::new(ValueKind::OpaqueUniqueKey, key)
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

    pub const fn as_legacy_traced(self) -> Option<u32> {
        match self.kind {
            ValueKind::LegacyTraced => Some(self.payload as u32),
            _ => None,
        }
    }

    pub const fn as_resource(self) -> Option<u32> {
        match self.kind {
            ValueKind::Resource => Some(self.payload as u32),
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

    #[doc(hidden)]
    pub const fn as_opaque_unique_key(self) -> Option<u64> {
        match self.kind {
            ValueKind::OpaqueUniqueKey => Some(self.payload),
            _ => None,
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_invalid() {
            return formatter.write_str("#<invalid>");
        }
        if self.is_unit() {
            return formatter.write_str("unit");
        }
        if self.is_empty_list() {
            return formatter.write_str("empty-list");
        }
        if let Some(value) = self.as_bool() {
            return value.fmt(formatter);
        }
        if let Some(value) = self.as_i64() {
            return value.fmt(formatter);
        }
        if let Some(value) = self.as_f64() {
            return value.fmt(formatter);
        }
        if let Some(index) = self.as_resource() {
            return write!(formatter, "resource#{index}");
        }
        if let Some(kind) = self.as_capability() {
            return write!(formatter, "capability#{}", kind.as_str());
        }
        if let Some(index) = self.as_legacy_traced() {
            return write!(formatter, "legacy-traced#{index}");
        }
        if let Some(key) = self.as_opaque_unique_key() {
            return write!(formatter, "opaque-unique#{key}");
        }
        formatter.write_str("#<invalid-value-category>")
    }
}
