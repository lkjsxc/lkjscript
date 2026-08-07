use std::marker::PhantomData;
use std::num::NonZeroU32;

use super::super::bookkeeping::Ledger;
use super::super::{DomainKey, RootKey};

#[derive(Debug)]
pub struct RegionOwner<T, D> {
    pub(super) key: DomainKey,
    pub(super) marker: PhantomData<fn() -> (T, D)>,
}

impl<T, D> RegionOwner<T, D> {
    pub const fn domain(&self) -> DomainKey {
        self.key
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RegionRef<T> {
    pub(super) key: RootKey,
    pub(super) marker: PhantomData<fn() -> T>,
}

impl<T> RegionRef<T> {
    pub const fn root(self) -> RootKey {
        self.key
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ObjectLocation {
    Chunk { chunk: u32, offset: u32 },
    Large { index: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RootRecord {
    pub(super) generation: NonZeroU32,
    pub(super) location: ObjectLocation,
}

#[derive(Debug)]
pub(super) struct RegionRecord<T, D> {
    pub(super) epoch: NonZeroU32,
    pub(super) chunks: Vec<Vec<T>>,
    pub(super) large: Vec<Vec<T>>,
    pub(super) roots: Vec<RootRecord>,
    pub(super) internal_edges: Ledger<(u32, u32)>,
    pub(super) children: Ledger<DomainKey>,
    pub(super) drops: Ledger<D>,
    pub(super) parent: Option<DomainKey>,
    pub(super) release_work: u64,
    pub(super) loans: u64,
    pub(super) bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionReleaseReport<E> {
    pub domains_released: u64,
    pub chunks_released: u64,
    pub large_objects_released: u64,
    pub objects_released: u64,
    pub drop_failures: Vec<E>,
}
