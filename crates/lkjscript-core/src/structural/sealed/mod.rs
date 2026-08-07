mod allocation;
mod graph;
mod model;
mod owner;
mod owner_counts;
mod preflight;
mod release;
mod rollback;
mod seal;
mod validation;
mod weight;

use std::marker::PhantomData;
use std::num::NonZeroU64;

pub use model::{
    SealFailure, SealedBorrow, SealedBuilder, SealedOwner, SealedRef, SealedReleaseReport,
    SealedUpgrade, WeakSealedRef,
};

use super::bookkeeping::Ledger;
use super::{
    DomainClass, DomainKey, LayoutIdentity, SealedRegionMetrics, SemanticTypeIdentity,
    StructuralError, StructuralRuntime, StructuralRuntimeId,
};
use model::SealedRecord;

#[derive(Debug)]
pub struct SealedRegionStore<T: Copy, D: Copy> {
    runtime: StructuralRuntimeId,
    layout: LayoutIdentity,
    semantic_type: SemanticTypeIdentity,
    records: Vec<(DomainKey, SealedRecord<T, D>)>,
    indices: Vec<Option<usize>>,
    metrics: SealedRegionMetrics,
}

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub fn new(
        runtime: StructuralRuntimeId,
        layout: LayoutIdentity,
        semantic_type: SemanticTypeIdentity,
    ) -> Result<Self, StructuralError> {
        Ok(Self {
            runtime,
            layout,
            semantic_type,
            records: Vec::new(),
            indices: Vec::new(),
            metrics: SealedRegionMetrics::default(),
        })
    }

    pub const fn metrics(&self) -> SealedRegionMetrics {
        self.metrics
    }

    pub(crate) fn exact_live_state(&self) -> (u64, u64, u64, u64, u64) {
        self.records.iter().fold(
            (0, 0, 0, 0, 0),
            |(regions, owners, loans, dependencies, backlog), (_, record)| {
                (
                    regions + 1,
                    owners + record.owners,
                    loans + record.loans,
                    dependencies + record.dependencies.as_slice().len() as u64,
                    backlog + record.release_work,
                )
            },
        )
    }

    pub fn begin(
        &mut self,
        runtime: &mut StructuralRuntime,
    ) -> Result<SealedBuilder<T, D>, StructuralError> {
        self.require_runtime(runtime)?;
        self.records
            .try_reserve(1)
            .map_err(|_| StructuralError::AllocationFailed)?;
        let key = runtime.allocate(DomainClass::RegionBuilding)?;
        if let Err(error) = self.install_index(key) {
            runtime.rollback_allocation(key);
            return Err(error);
        }
        self.records.push((
            key,
            SealedRecord {
                chunks: Vec::new(),
                large: Vec::new(),
                roots: Vec::new(),
                internal_edges: Ledger::new(),
                dependencies: Ledger::new(),
                drops: Ledger::new(),
                owners: 0,
                release_work: 1,
                loans: 0,
                bytes: 0,
            },
        ));
        self.metrics.builders_created = self.metrics.builders_created.saturating_add(1);
        Ok(SealedBuilder {
            key,
            marker: PhantomData,
        })
    }

    pub fn add_dependency(
        &mut self,
        builder: &SealedBuilder<T, D>,
        target: DomainKey,
    ) -> Result<(), StructuralError> {
        if target.runtime() != self.runtime {
            return Err(StructuralError::WrongRuntime);
        }
        if !matches!(
            target.class(),
            DomainClass::RegionBuilding | DomainClass::RegionSealed
        ) {
            return Err(StructuralError::UnsupportedDependency);
        }
        let record = self.record_mut(builder.key)?;
        record.dependencies.push_unique(target)?;
        self.metrics.dependency_edges = self.metrics.dependency_edges.saturating_add(1);
        Ok(())
    }

    pub fn register_drop(
        &mut self,
        builder: &SealedBuilder<T, D>,
        drop: D,
    ) -> Result<(), StructuralError> {
        let record = self.record_mut(builder.key)?;
        record.drops.push(drop)
    }

    fn require_runtime(&self, runtime: &StructuralRuntime) -> Result<(), StructuralError> {
        (runtime.identity() == self.runtime)
            .then_some(())
            .ok_or(StructuralError::WrongRuntime)
    }

    fn record_index(&self, key: DomainKey) -> Result<usize, StructuralError> {
        if key.runtime() != self.runtime {
            return Err(StructuralError::WrongRuntime);
        }
        let slot = usize::try_from(key.slot()).map_err(|_| StructuralError::ArithmeticOverflow)?;
        self.indices
            .get(slot)
            .and_then(|index| *index)
            .filter(|index| {
                self.records
                    .get(*index)
                    .is_some_and(|record| record.0 == key)
            })
            .ok_or(StructuralError::StaleDomain(key))
    }

    fn install_index(&mut self, key: DomainKey) -> Result<(), StructuralError> {
        let slot = usize::try_from(key.slot()).map_err(|_| StructuralError::ArithmeticOverflow)?;
        if slot >= self.indices.len() {
            let length = slot
                .checked_add(1)
                .ok_or(StructuralError::ArithmeticOverflow)?;
            let additional = length
                .checked_sub(self.indices.len())
                .ok_or(StructuralError::ArithmeticOverflow)?;
            self.indices
                .try_reserve(additional)
                .map_err(|_| StructuralError::AllocationFailed)?;
            self.indices.resize(length, None);
        }
        if self.indices[slot].is_some() {
            return Err(StructuralError::DuplicateDependency);
        }
        self.indices[slot] = Some(self.records.len());
        Ok(())
    }

    fn take_record(&mut self, key: DomainKey) -> Result<SealedRecord<T, D>, StructuralError> {
        let index = self.record_index(key)?;
        let slot = usize::try_from(key.slot()).map_err(|_| StructuralError::ArithmeticOverflow)?;
        self.indices[slot] = None;
        let (_, record) = self.records.swap_remove(index);
        if let Some((moved, _)) = self.records.get(index) {
            let moved_slot =
                usize::try_from(moved.slot()).map_err(|_| StructuralError::ArithmeticOverflow)?;
            self.indices[moved_slot] = Some(index);
        }
        Ok(record)
    }

    fn record_mut(&mut self, key: DomainKey) -> Result<&mut SealedRecord<T, D>, StructuralError> {
        let index = self.record_index(key)?;
        Ok(&mut self.records[index].1)
    }

    fn sealed_key(&self, key: DomainKey) -> DomainKey {
        key.with_class(DomainClass::RegionSealed)
    }

    fn epoch(key: DomainKey) -> NonZeroU64 {
        key.generation()
    }
}
