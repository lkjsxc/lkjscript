use std::collections::VecDeque;

use super::StructuralValueError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralEventKind {
    Allocate,
    Initialize,
    Borrow,
    EndView,
    Move,
    Publish,
    Clone,
    Drop,
    Release,
    Stale,
    SlotReuse,
    DestinationCreate,
    DestinationComplete,
    DestinationAbort,
    DestinationCleanup,
    StringView,
    StaticRegister,
    StaticUnregister,
    Export,
    Seal,
    SealedAcquire,
    SealedRelease,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralEvent {
    pub sequence: u64,
    pub kind: StructuralEventKind,
    pub subject: u32,
    pub amount: u64,
}

#[derive(Debug)]
pub struct StructuralEventLog {
    next_sequence: u64,
    records: VecDeque<StructuralEvent>,
    omitted: u64,
}

impl StructuralEventLog {
    pub(super) const fn new() -> Self {
        Self {
            next_sequence: 1,
            records: VecDeque::new(),
            omitted: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub const fn omitted(&self) -> u64 {
        self.omitted
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &StructuralEvent> {
        self.records.iter()
    }

    pub(super) fn retained_bytes_estimate(&self) -> Result<u64, StructuralValueError> {
        u64::try_from(self.records.capacity())
            .ok()
            .and_then(|capacity| {
                capacity.checked_mul(std::mem::size_of::<StructuralEvent>() as u64)
            })
            .ok_or(StructuralValueError::ArithmeticOverflow)
    }

    /// Retains diagnostics opportunistically. Allocation failure must not stop
    /// ownership cleanup or alter structural semantics.
    pub(super) fn record(&mut self, kind: StructuralEventKind, subject: u32, amount: u64) -> bool {
        if self.records.try_reserve(1).is_err() {
            self.omitted = self.omitted.saturating_add(1);
            return true;
        }
        self.records.push_back(StructuralEvent {
            sequence: self.next_sequence,
            kind,
            subject,
            amount,
        });
        self.next_sequence = self.next_sequence.saturating_add(1);
        false
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StructuralValueRuntimeMetrics {
    pub allocations: u64,
    pub initializations: u64,
    pub borrows: u64,
    pub moves: u64,
    pub publications: u64,
    pub clones: u64,
    pub drops: u64,
    pub releases: u64,
    pub stale_rejections: u64,
    pub object_slots_reused: u64,
    pub live_objects: u32,
    pub peak_live_objects: u32,
    pub destinations_created: u64,
    pub destinations_completed: u64,
    pub destinations_aborted: u64,
    pub destination_fields_initialized: u64,
    pub destination_cleanup_work: u64,
    pub live_destinations: u32,
    pub views_created: u64,
    pub views_ended: u64,
    pub live_views: u32,
    pub peak_live_views: u32,
    pub string_bytes_allocated: u64,
    pub string_bytes_live: u64,
    pub string_bytes_cloned: u64,
    pub string_bytes_released: u64,
    pub path_bytes_allocated: u64,
    pub path_bytes_live: u64,
    pub path_bytes_cloned: u64,
    pub path_bytes_released: u64,
    pub payload_bytes_live: u64,
    pub payload_bytes_peak: u64,
    pub clone_nodes: u64,
    pub release_work: u64,
    pub release_backlog: u32,
    pub sealed_publications: u64,
    pub zero_copy_adoptions: u64,
    pub sealed_acquisitions: u64,
    pub sealed_releases: u64,
    pub live_sealed_domains: u32,
    pub live_sealed_owners: u32,
    pub sealed_release_work: u64,
    pub sealed_nodes_reclaimed: u64,
    pub copied_publication_bytes: u64,
    pub events_overwritten: u64,
}
