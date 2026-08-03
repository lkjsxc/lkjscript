use super::super::StructuralValueKey;
use super::{
    StructuralEventKind, StructuralImage, StructuralObject, StructuralType, StructuralValueError,
    StructuralValueRuntime, TreeFacts,
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

    pub(super) fn take_owned_image(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<(StructuralImage, TreeFacts), StructuralValueError> {
        let image = self.remove_owned_image(key, expected)?;
        self.metrics.moves = self.metrics.moves.saturating_add(1);
        Ok(image)
    }

    pub(super) fn drop_owned_image(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<(StructuralImage, TreeFacts), StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        self.require_owned_root(root, expected)?;
        self.runtime.preflight_release(&[root.domain()])?;
        self.objects.preflight_take(root)?;
        let root = self.roots.drop_owned(key)?;
        let StructuralObject::Owned { image, facts } = self.objects.take(root)? else {
            return Err(StructuralValueError::InvariantViolation);
        };
        self.runtime.release(root.domain())?;
        self.note_object_removed(facts);
        Ok((image, facts))
    }

    pub(super) fn remove_owned_image(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<(StructuralImage, TreeFacts), StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        self.require_owned_root(root, expected)?;
        self.runtime.preflight_release(&[root.domain()])?;
        self.objects.preflight_take(root)?;
        let root = self.roots.take_owned(key)?;
        let StructuralObject::Owned { image, facts } = self.objects.take(root)? else {
            return Err(StructuralValueError::InvariantViolation);
        };
        self.runtime.release(root.domain())?;
        self.note_object_removed(facts);
        Ok((image, facts))
    }

    pub(super) fn release_image(&mut self, image: StructuralImage, facts: TreeFacts) {
        let work = u64::from(facts.nodes);
        drop(image);
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
