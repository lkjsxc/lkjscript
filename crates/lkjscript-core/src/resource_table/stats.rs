use lkjscript_contracts::ResourceKind;

use super::{ResourceObservation, ResourceOwnership, ResourceState};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceTableStats {
    pub(super) allocated_slots: usize,
    pub(super) vacant: usize,
    pub(super) reserved: usize,
    pub(super) reserved_owned: usize,
    pub(super) reserved_borrowed: usize,
    pub(super) owned_open: usize,
    pub(super) borrowed_open: usize,
    pub(super) closing: usize,
    pub(super) closed: usize,
    pub(super) retired: usize,
    pub(super) active_by_kind: [usize; ResourceKind::ALL.len()],
}

impl ResourceTableStats {
    pub(super) const fn empty(allocated_slots: usize) -> Self {
        Self {
            allocated_slots,
            vacant: 0,
            reserved: 0,
            reserved_owned: 0,
            reserved_borrowed: 0,
            owned_open: 0,
            borrowed_open: 0,
            closing: 0,
            closed: 0,
            retired: 0,
            active_by_kind: [0; ResourceKind::ALL.len()],
        }
    }

    pub(super) fn record(&mut self, observation: &ResourceObservation) {
        match observation.state() {
            ResourceState::Vacant => self.vacant += 1,
            ResourceState::Reserved => {
                self.reserved += 1;
                match observation.ownership() {
                    Some(ResourceOwnership::Owned) => self.reserved_owned += 1,
                    Some(ResourceOwnership::Borrowed) => self.reserved_borrowed += 1,
                    None => {}
                }
            }
            ResourceState::OwnedOpen => self.owned_open += 1,
            ResourceState::BorrowedOpen => self.borrowed_open += 1,
            ResourceState::Closing => self.closing += 1,
            ResourceState::Closed => self.closed += 1,
            ResourceState::Retired => self.retired += 1,
        }
        if let Some(kind) = observation.kind() {
            if let Some(index) = ResourceKind::ALL
                .iter()
                .position(|candidate| *candidate == kind)
            {
                self.active_by_kind[index] += 1;
            }
        }
    }

    pub const fn allocated_slots(&self) -> usize {
        self.allocated_slots
    }

    pub const fn vacant(&self) -> usize {
        self.vacant
    }

    pub const fn reserved(&self) -> usize {
        self.reserved
    }

    pub const fn reserved_owned(&self) -> usize {
        self.reserved_owned
    }

    pub const fn reserved_borrowed(&self) -> usize {
        self.reserved_borrowed
    }

    pub const fn owned_open(&self) -> usize {
        self.owned_open
    }

    pub const fn borrowed_open(&self) -> usize {
        self.borrowed_open
    }

    pub const fn closing(&self) -> usize {
        self.closing
    }

    pub const fn closed(&self) -> usize {
        self.closed
    }

    pub const fn retired(&self) -> usize {
        self.retired
    }

    pub const fn ordinary_obligations(&self) -> usize {
        self.reserved_owned + self.owned_open + self.closing
    }

    pub fn active_for(&self, kind: ResourceKind) -> usize {
        ResourceKind::ALL
            .iter()
            .position(|candidate| *candidate == kind)
            .map_or(0, |index| self.active_by_kind[index])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyObligations {
    pub(super) resources: Vec<ResourceObservation>,
}

impl EmergencyObligations {
    pub fn resources(&self) -> &[ResourceObservation] {
        &self.resources
    }

    pub const fn count(&self) -> usize {
        self.resources.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }
}
