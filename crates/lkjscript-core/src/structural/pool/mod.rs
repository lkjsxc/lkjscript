mod access;
mod model;

use std::cell::Cell;
use std::marker::PhantomData;
use std::num::NonZeroU32;

pub use model::{PoolId, PoolPartition};
use model::{PoolSlot, PoolSlotState};

use super::{
    DomainClass, DomainKey, LayoutIdentity, PoolMetrics, RootClass, RootKey, SemanticTypeIdentity,
    StructuralError, StructuralRuntime, StructuralRuntimeId,
};

#[derive(Debug)]
pub struct TypedPool<T> {
    runtime: StructuralRuntimeId,
    domain: DomainKey,
    layout: LayoutIdentity,
    semantic_type: SemanticTypeIdentity,
    slots: Vec<PoolSlot<T>>,
    free: Vec<u32>,
    metrics: Cell<PoolMetrics>,
}

impl<T> TypedPool<T> {
    pub fn new(
        runtime: &mut StructuralRuntime,
        layout: LayoutIdentity,
        semantic_type: SemanticTypeIdentity,
    ) -> Result<Self, StructuralError> {
        let domain = runtime.allocate(DomainClass::Pool)?;
        Ok(Self {
            runtime: runtime.identity(),
            domain,
            layout,
            semantic_type,
            slots: Vec::new(),
            free: Vec::new(),
            metrics: Cell::new(PoolMetrics::default()),
        })
    }

    pub const fn domain(&self) -> DomainKey {
        self.domain
    }

    pub fn metrics(&self) -> PoolMetrics {
        self.metrics.get()
    }

    pub fn insert(&mut self, value: T) -> Result<PoolId<T>, StructuralError> {
        let (slot, generation, reused) = if let Some(&slot) = self.free.last() {
            let entry = self
                .slots
                .get(slot as usize)
                .ok_or(StructuralError::SlotVacant)?;
            if entry.state != PoolSlotState::Vacant {
                return Err(StructuralError::SlotVacant);
            }
            self.free.pop();
            (slot, entry.generation, true)
        } else {
            self.slots
                .try_reserve(1)
                .map_err(|_| StructuralError::AllocationFailed)?;
            let slot =
                u32::try_from(self.slots.len()).map_err(|_| StructuralError::ArithmeticOverflow)?;
            self.slots.push(PoolSlot {
                generation: NonZeroU32::MIN,
                state: PoolSlotState::Vacant,
                value: None,
            });
            (slot, NonZeroU32::MIN, false)
        };
        let entry = &mut self.slots[slot as usize];
        entry.state = PoolSlotState::Initializing;
        entry.value = Some(value);
        entry.state = PoolSlotState::Live;
        let mut metrics = self.metrics.get();
        metrics.inserts = metrics.inserts.saturating_add(1);
        metrics.live_slots = metrics.live_slots.saturating_add(1);
        metrics.peak_live_slots = metrics.peak_live_slots.max(metrics.live_slots);
        metrics.bytes_live = metrics
            .bytes_live
            .saturating_add(std::mem::size_of::<T>() as u64);
        if reused {
            metrics.slots_reused = metrics.slots_reused.saturating_add(1);
        }
        self.metrics.set(metrics);
        Ok(PoolId {
            key: RootKey::from_parts(
                self.domain,
                RootClass::PoolElement,
                slot,
                generation,
                self.layout,
                self.semantic_type,
            ),
            marker: PhantomData,
        })
    }

    pub fn remove(&mut self, id: PoolId<T>) -> Result<T, StructuralError> {
        let index = self.validate_id(id)?;
        let generation = self.slots[index].generation.get();
        let retires = generation == u32::MAX;
        if !retires {
            self.free
                .try_reserve(1)
                .map_err(|_| StructuralError::AllocationFailed)?;
        }
        let entry = &mut self.slots[index];
        entry.state = PoolSlotState::Removing;
        let value = entry.value.take().ok_or(StructuralError::SlotVacant)?;
        if retires {
            entry.state = PoolSlotState::Retired;
        } else {
            entry.generation =
                NonZeroU32::new(generation + 1).ok_or(StructuralError::GenerationExhausted)?;
            entry.state = PoolSlotState::Vacant;
            self.free.push(id.key.slot());
        }
        let mut metrics = self.metrics.get();
        metrics.removes = metrics.removes.saturating_add(1);
        metrics.live_slots = metrics.live_slots.saturating_sub(1);
        metrics.bytes_live = metrics
            .bytes_live
            .saturating_sub(std::mem::size_of::<T>() as u64);
        if retires {
            metrics.slots_retired = metrics.slots_retired.saturating_add(1);
        }
        self.metrics.set(metrics);
        Ok(value)
    }

    pub fn partition(&self, start: u32, end: u32) -> Result<PoolPartition<T>, StructuralError> {
        if start > end || end as usize > self.slots.len() {
            return Err(StructuralError::WrongPartition);
        }
        Ok(PoolPartition {
            domain: self.domain,
            start,
            end,
            marker: PhantomData,
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (PoolId<T>, &T)> {
        self.slots.iter().enumerate().filter_map(|(slot, entry)| {
            let value = entry.value.as_ref()?;
            (entry.state == PoolSlotState::Live).then(|| {
                let id = PoolId {
                    key: RootKey::from_parts(
                        self.domain,
                        RootClass::PoolElement,
                        slot as u32,
                        entry.generation,
                        self.layout,
                        self.semantic_type,
                    ),
                    marker: PhantomData,
                };
                (id, value)
            })
        })
    }

    pub fn destroy(
        mut self,
        runtime: &mut StructuralRuntime,
    ) -> Result<PoolMetrics, StructuralError> {
        if runtime.identity() != self.runtime {
            return Err(StructuralError::WrongRuntime);
        }
        runtime.preflight_release(&[self.domain])?;
        for entry in &mut self.slots {
            if entry.state == PoolSlotState::Live {
                drop(entry.value.take());
                entry.state = PoolSlotState::Retired;
            }
        }
        runtime.release(self.domain)?;
        let mut metrics = self.metrics.get();
        metrics.live_slots = 0;
        metrics.bytes_live = 0;
        self.metrics.set(metrics);
        Ok(metrics)
    }
}
