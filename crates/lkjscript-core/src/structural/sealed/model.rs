use std::marker::PhantomData;
use std::num::NonZeroU32;

use super::super::bookkeeping::Ledger;
use super::super::{DomainKey, RootKey, StructuralError};

#[derive(Debug)]
pub struct SealedBuilder<T, D> {
    pub(super) key: DomainKey,
    pub(super) marker: PhantomData<fn() -> (T, D)>,
}

impl<T, D> SealedBuilder<T, D> {
    pub const fn domain(&self) -> DomainKey {
        self.key
    }
}

#[derive(Debug)]
pub struct SealedOwner<T, D> {
    pub(super) key: DomainKey,
    pub(super) marker: PhantomData<fn() -> (T, D)>,
}

impl<T, D> SealedOwner<T, D> {
    pub const fn domain(&self) -> DomainKey {
        self.key
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SealedRef<T> {
    pub(super) key: RootKey,
    pub(super) marker: PhantomData<fn() -> T>,
}

impl<T> SealedRef<T> {
    pub const fn root(self) -> RootKey {
        self.key
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WeakSealedRef<T> {
    pub(super) key: RootKey,
    pub(super) marker: PhantomData<fn() -> T>,
}

#[derive(Debug)]
pub struct SealedBorrow<T> {
    pub(super) key: RootKey,
    pub(super) marker: PhantomData<fn() -> T>,
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
pub(super) struct SealedRecord<T, D> {
    pub(super) chunks: Vec<Vec<T>>,
    pub(super) large: Vec<Vec<T>>,
    pub(super) roots: Vec<RootRecord>,
    pub(super) internal_edges: Ledger<(u32, u32)>,
    pub(super) dependencies: Ledger<DomainKey>,
    pub(super) drops: Ledger<D>,
    pub(super) owners: u64,
    pub(super) release_work: u64,
    pub(super) loans: u64,
    pub(super) bytes: u64,
}

pub type SealedUpgrade<T, D> = (SealedOwner<T, D>, SealedRef<T>);

#[derive(Debug)]
pub struct SealFailure<T, D> {
    pub error: StructuralError,
    pub builders: Vec<SealedBuilder<T, D>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealedReleaseReport<E> {
    pub regions_released: u64,
    pub objects_released: u64,
    pub dependency_releases: u64,
    pub drop_failures: Vec<E>,
}
