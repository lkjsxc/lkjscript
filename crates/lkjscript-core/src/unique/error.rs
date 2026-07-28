use std::fmt;

use super::UniqueLayout;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidUniqueStoreLimits {
    ObjectsExceedSlots,
    ZeroGeneration,
}

impl fmt::Display for InvalidUniqueStoreLimits {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObjectsExceedSlots => formatter.write_str("unique-store objects exceed slots"),
            Self::ZeroGeneration => formatter.write_str("unique-store generation limit is zero"),
        }
    }
}

impl std::error::Error for InvalidUniqueStoreLimits {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidUniqueKeyWord {
    ZeroGeneration,
}

impl fmt::Display for InvalidUniqueKeyWord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroGeneration => {
                formatter.write_str("unique key word generation must be nonzero")
            }
        }
    }
}

impl std::error::Error for InvalidUniqueKeyWord {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UniqueStoreError {
    AllocationLimit,
    ObjectLimit,
    ByteLimit,
    SlotLimit,
    ArithmeticOverflow,
    StorageCapacity,
    StoreMismatch,
    StaleKey,
    RangeOverflow {
        start: usize,
        len: usize,
    },
    RangeOutOfBounds {
        start: usize,
        len: usize,
        available: usize,
    },
    WrongLayout {
        expected: UniqueLayout,
        actual: UniqueLayout,
    },
}

impl fmt::Display for UniqueStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllocationLimit => formatter.write_str("unique-store allocation limit exceeded"),
            Self::ObjectLimit => formatter.write_str("unique-store live object limit exceeded"),
            Self::ByteLimit => formatter.write_str("unique-store live byte limit exceeded"),
            Self::SlotLimit => formatter.write_str("unique-store slot limit exceeded"),
            Self::ArithmeticOverflow => formatter.write_str("unique-store arithmetic overflow"),
            Self::StorageCapacity => {
                formatter.write_str("unique-store backing capacity unavailable")
            }
            Self::StoreMismatch => formatter.write_str("unique-store key belongs to another store"),
            Self::StaleKey => formatter.write_str("unique-store key is stale"),
            Self::RangeOverflow { start, len } => write!(
                formatter,
                "unique-store range start {start} plus length {len} overflows"
            ),
            Self::RangeOutOfBounds {
                start,
                len,
                available,
            } => write!(
                formatter,
                "unique-store range {start}..+{len} exceeds length {available}"
            ),
            Self::WrongLayout { expected, actual } => write!(
                formatter,
                "unique-store layout mismatch: expected {expected:?}, found {actual:?}"
            ),
        }
    }
}

impl std::error::Error for UniqueStoreError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UniqueStoreLeak {
    pub live_objects: u32,
    pub live_bytes: u64,
}

impl fmt::Display for UniqueStoreLeak {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "unique-store has {} live objects retaining {} bytes",
            self.live_objects, self.live_bytes
        )
    }
}

impl std::error::Error for UniqueStoreLeak {}
