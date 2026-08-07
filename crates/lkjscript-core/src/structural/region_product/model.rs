use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_REGION_PRODUCT_ARENA: AtomicU64 = AtomicU64::new(1);
static NEXT_REGION_PRODUCT_TOKEN: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RegionProductArenaId(NonZeroU64);

impl RegionProductArenaId {
    pub(super) fn fresh() -> Result<Self, RegionProductError> {
        let id = NEXT_REGION_PRODUCT_ARENA
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| RegionProductError::IdentityExhausted)?;
        NonZeroU64::new(id)
            .map(Self)
            .ok_or(RegionProductError::IdentityExhausted)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RegionProductKey {
    pub(super) arena: RegionProductArenaId,
    pub(super) token: NonZeroU64,
}

impl RegionProductKey {
    pub(super) const fn new(arena: RegionProductArenaId, token: NonZeroU64) -> Self {
        Self { arena, token }
    }

    pub const fn to_word(self) -> u64 {
        self.token.get()
    }

    pub fn from_word(arena: RegionProductArenaId, word: u64) -> Option<Self> {
        NonZeroU64::new(word).map(|token| Self { arena, token })
    }
}

pub(super) fn next_region_product_token() -> Result<NonZeroU64, RegionProductError> {
    let token = NEXT_REGION_PRODUCT_TOKEN
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| RegionProductError::RepresentationExhausted)?;
    NonZeroU64::new(token).ok_or(RegionProductError::RepresentationExhausted)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RegionProductMetrics {
    pub records: u64,
    pub fields: u64,
    pub retained_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionProductError {
    IdentityExhausted,
    RepresentationExhausted,
    HostAllocation,
    ArithmeticOverflow,
    InvalidKey,
    WrongType,
    FieldOutOfRange,
}

pub(super) struct RegionProductRecord<T> {
    pub identity: crate::RuntimeLayoutId,
    pub fields: Vec<T>,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn arena_and_opaque_key_words_preserve_values_above_u32() {
        let high = u64::from(u32::MAX) + 37;
        let arena = RegionProductArenaId(NonZeroU64::new(high).expect("high arena"));
        let key = RegionProductKey::new(arena, NonZeroU64::new(high + 1).expect("high token"));
        assert_eq!(arena.get(), high);
        assert_eq!(key.to_word(), high + 1);
        assert_eq!(RegionProductKey::from_word(arena, high + 1), Some(key));
    }
}
