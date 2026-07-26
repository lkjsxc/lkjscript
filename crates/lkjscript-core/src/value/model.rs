//! Tagged values and heap object kinds.

use std::fmt;

use super::CapabilityKind;

/// Low 3 bits are the tag; payload lives in the upper bits or as a heap index.
#[derive(Clone, Copy, PartialEq)]
pub struct Value(u64);

const TAG_MASK: u64 = 0b111;
const TAG_INVALID: u64 = 0;
const TAG_BOOL: u64 = 1;
const TAG_INT: u64 = 2;
const TAG_HEAP: u64 = 3;
const TAG_HANDLE: u64 = 4;
const TAG_UNIT: u64 = 5;
const TAG_EMPTY_LIST: u64 = 6;
const TAG_CAPABILITY: u64 = 7;

pub const MIN_SMALL_I64: i64 = -(1_i64 << 60);
pub const MAX_SMALL_I64: i64 = (1_i64 << 60) - 1;

impl Value {
    pub const INVALID: Self = Self(TAG_INVALID);
    pub const UNIT: Self = Self(TAG_UNIT);
    pub const EMPTY_LIST: Self = Self(TAG_EMPTY_LIST);
    pub const FALSE: Self = Self(TAG_BOOL);
    pub const TRUE: Self = Self((1 << 3) | TAG_BOOL);

    pub fn from_bool(b: bool) -> Self {
        if b {
            Self::TRUE
        } else {
            Self::FALSE
        }
    }

    pub fn from_small_i64(number: i64) -> Option<Self> {
        if !(MIN_SMALL_I64..=MAX_SMALL_I64).contains(&number) {
            return None;
        }
        Some(Self(((number as u64) << 3) | TAG_INT))
    }

    pub fn from_heap(index: u32) -> Self {
        Self(((index as u64) << 3) | TAG_HEAP)
    }

    pub fn from_handle(index: u32) -> Self {
        Self(((index as u64) << 3) | TAG_HANDLE)
    }

    pub const fn from_capability(kind: CapabilityKind) -> Self {
        Self(((kind as u64) << 3) | TAG_CAPABILITY)
    }

    pub fn is_invalid(self) -> bool {
        self.0 == TAG_INVALID
    }

    pub fn is_unit(self) -> bool {
        self.0 == TAG_UNIT
    }

    pub fn is_empty_list(self) -> bool {
        self.0 == TAG_EMPTY_LIST
    }

    pub fn as_bool(self) -> Option<bool> {
        if self.0 & TAG_MASK != TAG_BOOL {
            return None;
        }
        Some(self.0 >> 3 != 0)
    }

    pub fn as_small_i64(self) -> Option<i64> {
        if self.0 & TAG_MASK != TAG_INT {
            return None;
        }
        Some((self.0 as i64) >> 3)
    }

    pub fn as_heap(self) -> Option<u32> {
        if self.0 & TAG_MASK != TAG_HEAP {
            return None;
        }
        Some((self.0 >> 3) as u32)
    }

    pub fn as_handle(self) -> Option<u32> {
        if self.0 & TAG_MASK != TAG_HANDLE {
            return None;
        }
        Some((self.0 >> 3) as u32)
    }

    pub fn as_capability(self) -> Option<CapabilityKind> {
        if self.0 & TAG_MASK != TAG_CAPABILITY {
            return None;
        }
        u8::try_from(self.0 >> 3)
            .ok()
            .and_then(CapabilityKind::from_tag)
    }

    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_invalid() {
            return write!(f, "#<invalid>");
        }
        if self.is_unit() {
            return write!(f, "unit");
        }
        if self.is_empty_list() {
            return write!(f, "empty-list");
        }
        if let Some(b) = self.as_bool() {
            return write!(f, "{b}");
        }
        if let Some(n) = self.as_small_i64() {
            return write!(f, "{n}");
        }
        if let Some(h) = self.as_handle() {
            return write!(f, "handle#{h}");
        }
        if let Some(kind) = self.as_capability() {
            return write!(f, "capability#{}", kind.as_str());
        }
        if let Some(i) = self.as_heap() {
            return write!(f, "heap#{i}");
        }
        write!(f, "value({:x})", self.0)
    }
}
