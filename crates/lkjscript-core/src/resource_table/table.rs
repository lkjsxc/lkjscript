use std::num::NonZeroU64;

use super::slot::{Slot, SlotState};
use super::{
    EmergencyObligations, ResourceObservation, ResourceTableError, ResourceTableLimit,
    ResourceTableLimits, ResourceTableStats, ScopeId,
};

pub struct ResourceTable<P> {
    pub(super) scope: ScopeId,
    pub(super) limits: ResourceTableLimits,
    pub(super) slots: Vec<Slot<P>>,
    pub(super) reusable: Vec<usize>,
    pub(super) owned_order: Vec<usize>,
    pub(super) next_sequence: u64,
}

impl<P> ResourceTable<P> {
    pub const fn new(scope: ScopeId, limits: ResourceTableLimits) -> Self {
        Self {
            scope,
            limits,
            slots: Vec::new(),
            reusable: Vec::new(),
            owned_order: Vec::new(),
            next_sequence: 0,
        }
    }

    pub const fn scope(&self) -> ScopeId {
        self.scope
    }

    pub const fn limits(&self) -> ResourceTableLimits {
        self.limits
    }

    pub fn observations(&self) -> Vec<ResourceObservation> {
        self.slots.iter().map(Slot::observation).collect()
    }

    pub fn stats(&self) -> ResourceTableStats {
        let mut stats = ResourceTableStats::empty(self.slots.len());
        for slot in &self.slots {
            stats.record(&slot.observation());
        }
        stats
    }

    pub fn emergency_obligations(&self) -> EmergencyObligations {
        let mut resources: Vec<_> = self
            .slots
            .iter()
            .filter_map(|slot| {
                slot.acquisition_sequence()
                    .map(|sequence| (sequence, slot.observation()))
            })
            .collect();
        resources.sort_by_key(|resource| std::cmp::Reverse(resource.0));
        EmergencyObligations {
            resources: resources
                .into_iter()
                .map(|(_, observation)| observation)
                .collect(),
        }
    }

    pub fn assert_zero_ordinary_obligations(&self) -> Result<(), ResourceTableError> {
        let count = self.stats().ordinary_obligations();
        if count == 0 {
            Ok(())
        } else {
            Err(ResourceTableError::OutstandingOrdinaryObligations { count })
        }
    }

    pub(super) fn take_reusable_slot(&mut self) -> Result<usize, ResourceTableError> {
        while let Some(index) = self.reusable.pop() {
            match self.slots[index].state {
                SlotState::Vacant => return Ok(index),
                SlotState::Closed => {
                    let generation = self.slots[index].generation;
                    let next = generation.get().checked_add(1).and_then(NonZeroU64::new);
                    if let Some(next) = next.filter(|value| {
                        self.limits
                            .max_generation()
                            .is_none_or(|maximum| *value <= maximum)
                    }) {
                        self.slots[index].generation = next;
                        self.slots[index].state = SlotState::Vacant;
                        return Ok(index);
                    }
                    self.slots[index].state = SlotState::Retired;
                }
                SlotState::Reserved(_)
                | SlotState::OwnedOpen(_)
                | SlotState::BorrowedOpen(_)
                | SlotState::Closing(_)
                | SlotState::Retired => {}
            }
        }
        if self
            .limits
            .max_slots()
            .is_none_or(|maximum| self.slots.len() < maximum)
        {
            let index = self.slots.len();
            self.slots.push(Slot::new(NonZeroU64::MIN));
            return Ok(index);
        }
        let retired_slots = self.stats().retired();
        if retired_slots != 0 {
            Err(ResourceTableError::GenerationExhausted { retired_slots })
        } else {
            Err(ResourceTableError::LimitExceeded {
                limit: ResourceTableLimit::Slots,
                configured: self
                    .limits
                    .max_slots()
                    .ok_or(ResourceTableError::AcquisitionSequenceExhausted)?,
                observed: self.slots.len().saturating_add(1),
            })
        }
    }

    pub(super) fn release_parent(&mut self, parent: Option<super::slot::ParentRef>) {
        let Some(parent) = parent else {
            return;
        };
        let Some(slot) = self.slots.get_mut(parent.slot) else {
            return;
        };
        if slot.generation != parent.generation {
            return;
        }
        match &mut slot.state {
            SlotState::OwnedOpen(open) | SlotState::BorrowedOpen(open) => {
                open.live_children = open.live_children.saturating_sub(1);
            }
            _ => {}
        }
    }
}
