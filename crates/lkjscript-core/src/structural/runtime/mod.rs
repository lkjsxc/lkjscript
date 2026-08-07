mod release;
mod transition;

use std::num::{NonZeroU32, NonZeroU64};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{
    DomainClass, DomainKey, StructuralError, StructuralRuntimeId, StructuralRuntimeMetrics,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SlotState {
    Vacant,
    Live(DomainClass),
    Retired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DomainSlot {
    generation: NonZeroU32,
    state: SlotState,
}

static NEXT_RUNTIME_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct StructuralRuntime {
    identity: StructuralRuntimeId,
    slots: Vec<DomainSlot>,
    free: Vec<u32>,
    metrics: StructuralRuntimeMetrics,
}

impl StructuralRuntime {
    pub fn new() -> Result<Self, StructuralError> {
        let raw = NEXT_RUNTIME_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                next.checked_add(1)
            })
            .map_err(|_| StructuralError::RuntimeIdentityExhausted)?;
        let identity = StructuralRuntimeId::new(
            NonZeroU64::new(raw).ok_or(StructuralError::RuntimeIdentityExhausted)?,
        );
        Ok(Self {
            identity,
            slots: Vec::new(),
            free: Vec::new(),
            metrics: StructuralRuntimeMetrics::default(),
        })
    }

    pub const fn identity(&self) -> StructuralRuntimeId {
        self.identity
    }

    pub const fn metrics(&self) -> StructuralRuntimeMetrics {
        self.metrics
    }

    pub(in crate::structural) fn retained_bytes_estimate(&self) -> Result<u64, StructuralError> {
        let slots = u64::try_from(self.slots.capacity())
            .ok()
            .and_then(|capacity| capacity.checked_mul(std::mem::size_of::<DomainSlot>() as u64))
            .ok_or(StructuralError::ArithmeticOverflow)?;
        let free = u64::try_from(self.free.capacity())
            .ok()
            .and_then(|capacity| capacity.checked_mul(std::mem::size_of::<u32>() as u64))
            .ok_or(StructuralError::ArithmeticOverflow)?;
        slots
            .checked_add(free)
            .ok_or(StructuralError::ArithmeticOverflow)
    }

    pub(super) fn allocate(&mut self, class: DomainClass) -> Result<DomainKey, StructuralError> {
        let (slot, generation, reused) = if let Some(&slot) = self.free.last() {
            let record = self
                .slots
                .get(slot as usize)
                .ok_or(StructuralError::ArithmeticOverflow)?;
            if record.state != SlotState::Vacant {
                return Err(StructuralError::StaleDomain(DomainKey::from_parts(
                    self.identity,
                    class,
                    slot,
                    record.generation,
                )));
            }
            self.free.pop();
            (slot, record.generation, true)
        } else {
            self.slots
                .try_reserve(1)
                .map_err(|_| StructuralError::AllocationFailed)?;
            let slot =
                u32::try_from(self.slots.len()).map_err(|_| StructuralError::ArithmeticOverflow)?;
            self.slots.push(DomainSlot {
                generation: NonZeroU32::MIN,
                state: SlotState::Vacant,
            });
            (slot, NonZeroU32::MIN, false)
        };
        self.slots[slot as usize].state = SlotState::Live(class);
        self.metrics.domains_created = self.metrics.domains_created.saturating_add(1);
        self.metrics.live_domains = self.metrics.live_domains.saturating_add(1);
        self.metrics.peak_live_domains = self
            .metrics
            .peak_live_domains
            .max(self.metrics.live_domains);
        if reused {
            self.metrics.slots_reused = self.metrics.slots_reused.saturating_add(1);
        }
        Ok(DomainKey::from_parts(
            self.identity,
            class,
            slot,
            generation,
        ))
    }

    pub fn validate(&self) -> Result<(), StructuralError> {
        for &slot in &self.free {
            if self.slots.get(slot as usize).map(|entry| entry.state) != Some(SlotState::Vacant) {
                return Err(StructuralError::SlotVacant);
            }
        }
        let live = self
            .slots
            .iter()
            .filter(|slot| matches!(slot.state, SlotState::Live(_)))
            .count() as u64;
        (live == self.metrics.live_domains)
            .then_some(())
            .ok_or(StructuralError::ArithmeticOverflow)
    }
}
