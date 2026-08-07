mod allocation;
mod model;
mod preflight;
mod release;
mod validation;

use std::marker::PhantomData;
use std::num::NonZeroU64;

pub use model::{RegionOwner, RegionRef, RegionReleaseReport};

use super::bookkeeping::Ledger;
use super::{
    DomainClass, DomainKey, LayoutIdentity, RegionMetrics, SemanticTypeIdentity, StructuralError,
    StructuralRuntime, StructuralRuntimeId,
};
use model::RegionRecord;

#[derive(Debug)]
pub struct RegionStore<T: Copy, D: Copy> {
    runtime: StructuralRuntimeId,
    layout: LayoutIdentity,
    semantic_type: SemanticTypeIdentity,
    records: Vec<(DomainKey, RegionRecord<T, D>)>,
    indices: Vec<Option<usize>>,
    metrics: RegionMetrics,
}

impl<T: Copy, D: Copy> RegionStore<T, D> {
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
        if let Err(error) = self.install_index(key) {
            runtime.rollback_allocation(key);
            return Err(error);
        }
        self.records.push((
            key,
            RegionRecord {
                epoch: NonZeroU64::MIN,
                chunks: Vec::new(),
                large: Vec::new(),
                roots: Vec::new(),
                internal_edges: Ledger::new(),
                children: Ledger::new(),
                drops: Ledger::new(),
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
            Some(work) => work,
            None => return Err((StructuralError::ArithmeticOverflow, child)),
        };
        if let Err(error) = self.records[parent_index].1.children.push_unique(child.key) {
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
        record.drops.push(drop)?;
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
            let additional = slot
                .checked_add(1)
                .and_then(|length| length.checked_sub(self.indices.len()))
                .ok_or(StructuralError::ArithmeticOverflow)?;
            self.indices
                .try_reserve(additional)
                .map_err(|_| StructuralError::AllocationFailed)?;
            self.indices.resize(slot + 1, None);
        }
        if self.indices[slot].is_some() {
            return Err(StructuralError::DuplicateDependency);
        }
        self.indices[slot] = Some(self.records.len());
        Ok(())
    }

    fn take_record(&mut self, key: DomainKey) -> Result<RegionRecord<T, D>, StructuralError> {
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
