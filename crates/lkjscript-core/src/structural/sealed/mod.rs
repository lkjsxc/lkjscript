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
use std::num::NonZeroU32;

pub use model::{
    SealFailure, SealedBorrow, SealedBuilder, SealedOwner, SealedRef, SealedReleaseReport,
    SealedUpgrade, WeakSealedRef,
};

use super::ledger::BoundedLedger;
use super::{
    DomainClass, DomainKey, LayoutIdentity, SealedRegionMetrics, SemanticTypeIdentity,
    StructuralError, StructuralLimit, StructuralLimits, StructuralRuntime, StructuralRuntimeId,
};
use model::SealedRecord;

#[derive(Debug)]
pub struct SealedRegionStore<T: Copy, D: Copy> {
    runtime: StructuralRuntimeId,
    layout: LayoutIdentity,
    semantic_type: SemanticTypeIdentity,
    limits: StructuralLimits,
    records: Vec<(DomainKey, SealedRecord<T, D>)>,
    metrics: SealedRegionMetrics,
}

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub fn new(
        runtime: StructuralRuntimeId,
        layout: LayoutIdentity,
        semantic_type: SemanticTypeIdentity,
        limits: StructuralLimits,
    ) -> Result<Self, StructuralError> {
        Ok(Self {
            runtime,
            layout,
            semantic_type,
            limits: limits.validate()?,
            records: Vec::new(),
            metrics: SealedRegionMetrics::default(),
        })
    }

    pub const fn metrics(&self) -> SealedRegionMetrics {
        self.metrics
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
        self.records.push((
            key,
            SealedRecord {
                chunks: Vec::new(),
                large: Vec::new(),
                roots: Vec::new(),
                internal_edges: BoundedLedger::new(self.limits.max_dependencies),
                dependencies: BoundedLedger::new(self.limits.max_dependencies),
                drops: BoundedLedger::new(self.limits.max_drop_entries),
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
        record
            .dependencies
            .push_unique(target, StructuralLimit::Dependencies)?;
        self.metrics.dependency_edges = self.metrics.dependency_edges.saturating_add(1);
        Ok(())
    }

    pub fn register_drop(
        &mut self,
        builder: &SealedBuilder<T, D>,
        drop: D,
    ) -> Result<(), StructuralError> {
        let record = self.record_mut(builder.key)?;
        record.drops.push(drop, StructuralLimit::DropEntries)
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
        self.records
            .iter()
            .position(|(candidate, _)| *candidate == key)
            .ok_or(StructuralError::StaleDomain(key))
    }

    fn record_mut(&mut self, key: DomainKey) -> Result<&mut SealedRecord<T, D>, StructuralError> {
        let index = self.record_index(key)?;
        Ok(&mut self.records[index].1)
    }

    fn sealed_key(&self, key: DomainKey) -> DomainKey {
        key.with_class(DomainClass::RegionSealed)
    }

    fn epoch(key: DomainKey) -> NonZeroU32 {
        key.generation()
    }
}
