use std::marker::PhantomData;
use std::num::NonZeroU32;

use super::super::RootKey;

#[derive(Debug)]
pub struct PoolId<T> {
    pub(super) key: RootKey,
    pub(super) marker: PhantomData<fn() -> T>,
}

impl<T> Clone for PoolId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for PoolId<T> {}

impl<T> PartialEq for PoolId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T> Eq for PoolId<T> {}

impl<T> PoolId<T> {
    pub const fn root(self) -> RootKey {
        self.key
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoolSlotState {
    Vacant,
    Initializing,
    Live,
    Removing,
    Retired,
}

#[derive(Debug)]
pub(super) struct PoolSlot<T> {
    pub(super) generation: NonZeroU32,
    pub(super) state: PoolSlotState,
    pub(super) value: Option<T>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PoolPartition<T> {
    pub(super) domain: super::super::DomainKey,
    pub(super) start: u32,
    pub(super) end: u32,
    pub(super) marker: PhantomData<fn() -> T>,
}

impl<T> PoolPartition<T> {
    pub const fn range(&self) -> (u32, u32) {
        (self.start, self.end)
    }

    pub fn contains(&self, id: PoolId<T>) -> bool {
        id.key.domain() == self.domain && id.key.slot() >= self.start && id.key.slot() < self.end
    }
}
