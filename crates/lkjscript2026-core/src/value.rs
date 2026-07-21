//! Tagged values and heap object kinds.

use std::fmt;

/// Low 3 bits are the tag; payload lives in the upper bits or as a heap index.
#[derive(Clone, Copy, PartialEq)]
pub struct Value(u64);

const TAG_MASK: u64 = 0b111;
const TAG_NIL: u64 = 0;
const TAG_BOOL: u64 = 1;
const TAG_INT: u64 = 2;
const TAG_HEAP: u64 = 3;

impl Value {
    pub const NIL: Self = Self(TAG_NIL);
    pub const FALSE: Self = Self(TAG_BOOL);
    pub const TRUE: Self = Self((1 << 3) | TAG_BOOL);

    pub fn from_bool(b: bool) -> Self {
        if b {
            Self::TRUE
        } else {
            Self::FALSE
        }
    }

    pub fn from_int(n: i64) -> Self {
        let bits = (n as u64) << 3;
        Self(bits | TAG_INT)
    }

    pub fn from_heap(index: u32) -> Self {
        Self(((index as u64) << 3) | TAG_HEAP)
    }

    pub fn is_nil(self) -> bool {
        self.0 & TAG_MASK == TAG_NIL
    }

    pub fn as_bool(self) -> Option<bool> {
        if self.0 & TAG_MASK != TAG_BOOL {
            return None;
        }
        Some(self.0 >> 3 != 0)
    }

    pub fn as_int(self) -> Option<i64> {
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

    pub fn is_truthy(self) -> bool {
        if self.is_nil() {
            return false;
        }
        if let Some(b) = self.as_bool() {
            return b;
        }
        true
    }

    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_nil() {
            return write!(f, "nil");
        }
        if let Some(b) = self.as_bool() {
            return write!(f, "{b}");
        }
        if let Some(n) = self.as_int() {
            return write!(f, "{n}");
        }
        if let Some(i) = self.as_heap() {
            return write!(f, "heap#{i}");
        }
        write!(f, "value({:x})", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum HeapObj {
    Float(f64),
    Str(String),
    Symbol(String),
    Pair { car: Value, cdr: Value },
    Closure { proto: u32, captures: Vec<Value> },
    Builtin(u16),
    Buf(Vec<u8>),
}
