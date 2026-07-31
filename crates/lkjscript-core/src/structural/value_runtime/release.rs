use super::{
    SemanticPayload, SemanticValue, StructuralEventKind, StructuralValueRuntime, TreeFacts,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DestinationCleanupReport {
    pub sequence: u64,
    pub initialized_fields: u16,
    pub cleanup_order: Vec<u16>,
    pub nodes_released: u32,
    pub bytes_released: u64,
}

impl StructuralValueRuntime {
    pub(super) fn note_slot_reuse(&mut self, reused: bool) {
        self.metrics.object_slots_reused = self
            .metrics
            .object_slots_reused
            .saturating_add(u64::from(reused));
    }

    pub(super) fn note_publication(&mut self, facts: TreeFacts) {
        self.metrics.allocations = self.metrics.allocations.saturating_add(1);
        self.metrics.publications = self.metrics.publications.saturating_add(1);
        self.metrics.live_objects = self.metrics.live_objects.saturating_add(1);
        self.metrics.peak_live_objects = self
            .metrics
            .peak_live_objects
            .max(self.metrics.live_objects);
        self.metrics.string_bytes_allocated = self
            .metrics
            .string_bytes_allocated
            .saturating_add(facts.string_bytes);
        self.metrics.string_bytes_live = self
            .metrics
            .string_bytes_live
            .saturating_add(facts.string_bytes);
        self.metrics.path_bytes_allocated = self
            .metrics
            .path_bytes_allocated
            .saturating_add(facts.path_bytes);
        self.metrics.path_bytes_live = self
            .metrics
            .path_bytes_live
            .saturating_add(facts.path_bytes);
        self.metrics.payload_bytes_live =
            self.metrics.payload_bytes_live.saturating_add(facts.bytes);
        self.metrics.payload_bytes_peak = self
            .metrics
            .payload_bytes_peak
            .max(self.metrics.payload_bytes_live);
    }

    pub(super) fn note_object_removed(&mut self, facts: TreeFacts) {
        self.metrics.live_objects = self.metrics.live_objects.saturating_sub(1);
        self.metrics.string_bytes_live = self
            .metrics
            .string_bytes_live
            .saturating_sub(facts.string_bytes);
        self.metrics.path_bytes_live = self
            .metrics
            .path_bytes_live
            .saturating_sub(facts.path_bytes);
        self.metrics.payload_bytes_live =
            self.metrics.payload_bytes_live.saturating_sub(facts.bytes);
    }

    pub(super) fn release_tree(&mut self, value: SemanticValue, facts: TreeFacts) {
        self.release_stack.clear();
        self.release_stack.push(value);
        let mut work = 0_u64;
        while let Some(value) = self.release_stack.pop() {
            work = work.saturating_add(1);
            match value.payload {
                SemanticPayload::Product(fields)
                | SemanticPayload::Enum {
                    active_payload: fields,
                    ..
                } => self.release_stack.extend(fields),
                SemanticPayload::Inline(_)
                | SemanticPayload::Static(_)
                | SemanticPayload::String(_)
                | SemanticPayload::Path(_)
                | SemanticPayload::Bytes(_)
                | SemanticPayload::ByteVector(_) => {}
            }
        }
        self.metrics.releases = self.metrics.releases.saturating_add(1);
        self.metrics.release_work = self.metrics.release_work.saturating_add(work);
        self.metrics.string_bytes_released = self
            .metrics
            .string_bytes_released
            .saturating_add(facts.string_bytes);
        self.metrics.path_bytes_released = self
            .metrics
            .path_bytes_released
            .saturating_add(facts.path_bytes);
        self.record(StructuralEventKind::Release, 0, work);
    }
}
