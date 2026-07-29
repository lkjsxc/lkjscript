mod allocation;
mod model;
mod preflight;
mod release;
mod validation;

use std::marker::PhantomData;
use std::num::NonZeroU32;

pub use model::{RegionOwner, RegionRef, RegionReleaseReport};

use super::ledger::BoundedLedger;
use super::{
    DomainClass, DomainKey, LayoutIdentity, RegionMetrics, SemanticTypeIdentity, StructuralError,
    StructuralLimit, StructuralLimits, StructuralRuntime, StructuralRuntimeId,
};
use model::RegionRecord;

#[derive(Debug)]
pub struct RegionStore<T: Copy, D: Copy> {
    runtime: StructuralRuntimeId,
    layout: LayoutIdentity,
    semantic_type: SemanticTypeIdentity,
    limits: StructuralLimits,
    records: Vec<(DomainKey, RegionRecord<T, D>)>,
    metrics: RegionMetrics,
}

impl<T: Copy, D: Copy> RegionStore<T, D> {
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
            metrics: RegionMetrics::default(),
        })
    }

    pub const fn metrics(&self) -> RegionMetrics {
        self.metrics
    }

    pub fn create(
        &mut self,
        runtime: &mut StructuralRuntime,
    ) -> Result<RegionOwner<T, D>, StructuralError> {
        self.require_runtime(runtime)?;
        self.records
            .try_reserve(1)
            .map_err(|_| StructuralError::AllocationFailed)?;
        let key = runtime.allocate(DomainClass::RegionOwned)?;
        self.records.push((
            key,
            RegionRecord {
                epoch: NonZeroU32::MIN,
                chunks: Vec::new(),
                large: Vec::new(),
                roots: Vec::new(),
                internal_edges: BoundedLedger::new(self.limits.max_dependencies),
                children: BoundedLedger::new(self.limits.max_dependencies),
                drops: BoundedLedger::new(self.limits.max_drop_entries),
                parent: None,
                release_work: 1,
                loans: 0,
                bytes: 0,
            },
        ));
        self.metrics.regions_created = self.metrics.regions_created.saturating_add(1);
        Ok(RegionOwner {
            key,
            marker: PhantomData,
        })
    }

    pub fn attach(
        &mut self,
        parent: &RegionOwner<T, D>,
        child: RegionOwner<T, D>,
    ) -> Result<(), (StructuralError, RegionOwner<T, D>)> {
        let parent_index = match self.record_index(parent.key) {
            Ok(index) => index,
            Err(error) => return Err((error, child)),
        };
        let child_index = match self.record_index(child.key) {
            Ok(index) => index,
            Err(error) => return Err((error, child)),
        };
        if parent.key == child.key || self.is_ancestor(child.key, parent.key) {
            let witness = vec![parent.key, child.key, parent.key];
            return Err((StructuralError::DependencyCycle(witness), child));
        }
        if self.records[child_index].1.parent.is_some() {
            return Err((StructuralError::DuplicateDependency, child));
        }
        if self.records[parent_index].1.parent.is_some() {
            return Err((StructuralError::UnsupportedDependency, child));
        }
        let release_work = match self.records[parent_index]
            .1
            .release_work
            .checked_add(self.records[child_index].1.release_work)
        {
            Some(work) if work <= self.limits.max_release_work => work,
            _ => {
                return Err((
                    StructuralError::LimitExceeded(StructuralLimit::ReleaseWork),
                    child,
                ));
            }
        };
        if let Err(error) = self.records[parent_index]
            .1
            .children
            .push_unique(child.key, StructuralLimit::Dependencies)
        {
            return Err((error, child));
        }
        self.records[child_index].1.parent = Some(parent.key);
        self.records[parent_index].1.release_work = release_work;
        self.metrics.dependency_edges = self.metrics.dependency_edges.saturating_add(1);
        Ok(())
    }

    pub fn register_drop(
        &mut self,
        owner: &RegionOwner<T, D>,
        drop: D,
    ) -> Result<(), StructuralError> {
        let record = self.record_mut(owner.key)?;
        record.drops.push(drop, StructuralLimit::DropEntries)?;
        self.metrics.drop_entries = self.metrics.drop_entries.saturating_add(1);
        Ok(())
    }

    pub fn begin_loan(&mut self, owner: &RegionOwner<T, D>) -> Result<(), StructuralError> {
        let record = self.record_mut(owner.key)?;
        record.loans = record
            .loans
            .checked_add(1)
            .ok_or(StructuralError::ArithmeticOverflow)?;
        Ok(())
    }

    pub fn end_loan(&mut self, owner: &RegionOwner<T, D>) -> Result<(), StructuralError> {
        let record = self.record_mut(owner.key)?;
        record.loans = record
            .loans
            .checked_sub(1)
            .ok_or(StructuralError::LoanUnderflow)?;
        Ok(())
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

    fn record_mut(&mut self, key: DomainKey) -> Result<&mut RegionRecord<T, D>, StructuralError> {
        let index = self.record_index(key)?;
        Ok(&mut self.records[index].1)
    }

    fn is_ancestor(&self, candidate: DomainKey, mut key: DomainKey) -> bool {
        while let Ok(index) = self.record_index(key) {
            let Some(parent) = self.records[index].1.parent else {
                return false;
            };
            if parent == candidate {
                return true;
            }
            key = parent;
        }
        false
    }
}
