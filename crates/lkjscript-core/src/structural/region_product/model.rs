use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_REGION_PRODUCT_ARENA: AtomicU32 = AtomicU32::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RegionProductArenaId(NonZeroU32);

impl RegionProductArenaId {
    pub(super) fn fresh() -> Result<Self, RegionProductError> {
        let id = NEXT_REGION_PRODUCT_ARENA
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1).filter(|next| *next != 0)
            })
            .map_err(|_| RegionProductError::IdentityExhausted)?;
        NonZeroU32::new(id)
            .map(Self)
            .ok_or(RegionProductError::IdentityExhausted)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RegionProductKey {
    pub(super) arena: RegionProductArenaId,
    pub(super) record: NonZeroU32,
}

impl RegionProductKey {
    pub(super) const fn new(arena: RegionProductArenaId, record: NonZeroU32) -> Self {
        Self { arena, record }
    }

    pub const fn to_word(self) -> u64 {
        ((self.arena.0.get() as u64) << 32) | self.record.get() as u64
    }

    pub fn from_word(arena: RegionProductArenaId, word: u64) -> Option<Self> {
        let encoded_arena = u32::try_from(word >> 32).ok()?;
        let record = NonZeroU32::new(word as u32)?;
        (encoded_arena == arena.0.get()).then_some(Self { arena, record })
    }

    pub(super) fn index(self) -> Option<usize> {
        usize::try_from(self.record.get().checked_sub(1)?).ok()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RegionProductMetrics {
    pub records: u64,
    pub fields: u64,
    pub reserved_bytes_estimate: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionProductError {
    IdentityExhausted,
    RepresentationExhausted,
    ArithmeticOverflow,
    HostAllocation,
    InvalidKey,
    WrongType,
    FieldOutOfRange,
}

pub(super) struct RegionProductRecord<T> {
    pub identity: crate::RuntimeLayoutId,
    pub fields: Vec<T>,
}
